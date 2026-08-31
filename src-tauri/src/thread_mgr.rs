//! 后台线程统一生命周期管理（plan-2026-08-31 §4.1）。
//!
//! 目标：所有常驻后台线程具备统一的优雅退出能力。
//! - [`ExitFlag`]：线程循环内轮询/阻塞等待的退出标志（`Arc<AtomicBool>` + Condvar 唤醒）
//! - [`spawn_managed`]：spawn 同时登记退出标志与 JoinHandle（带名称，便于追踪与按需停止）
//! - [`stop_thread`]：按名称请求退出并 join
//!
//! 应用整体退出仍走托盘「强制退出」的 `std::process::exit(0)`（进程死亡即线程终结）；
//! 本机制主要服务于单线程按需停止（如音频频谱 §4.2）以及未来优雅退出路径。
//!
//! 注意：循环型线程请用 `sleep_interruptible` 替代裸 `thread::sleep`；
//! 阻塞挂起型线程（零唤醒诉求）用 `wait_for` 传长超时，被 signal 时会立即醒来。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

/// 线程退出标志：`signal` 后 `is_exiting()` 立即为 true，
/// `wait_for` / `sleep_interruptible` 立即返回（阻塞中被 Condvar 唤醒）。
#[derive(Clone)]
pub struct ExitFlag {
    flag: Arc<AtomicBool>,
    pair: Arc<(Mutex<bool>, Condvar)>,
}

impl ExitFlag {
    fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            pair: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    pub fn is_exiting(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    pub(crate) fn signal(&self) {
        self.flag.store(true, Ordering::SeqCst);
        let (guard, cv) = &*self.pair;
        let mut signaled = guard.lock().unwrap_or_else(|e| e.into_inner());
        *signaled = true;
        cv.notify_all();
    }

    /// 阻塞等待退出信号或超时；返回 true 表示已请求退出。
    /// 空闲线程可传长超时实现「零唤醒挂起」，signal 后立即醒来。
    pub fn wait_for(&self, timeout: Duration) -> bool {
        if self.is_exiting() {
            return true;
        }
        let (guard, cv) = &*self.pair;
        let g = guard.lock().unwrap_or_else(|e| e.into_inner());
        if *g {
            return true;
        }
        let (g, _result) = cv
            .wait_timeout_while(g, timeout, |signaled| !*signaled)
            .unwrap_or_else(|e| e.into_inner());
        *g
    }

    /// 可中断休眠：总时长 total；检测到退出信号立即返回 true（循环型线程用）
    pub fn sleep_interruptible(&self, total: Duration) -> bool {
        self.wait_for(total)
    }
}

struct ManagedThread {
    name: &'static str,
    flag: ExitFlag,
    handle: Option<JoinHandle<()>>,
}

static REGISTRY: OnceLock<Mutex<Vec<ManagedThread>>> = OnceLock::new();

fn registry() -> &'static Mutex<Vec<ManagedThread>> {
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

/// 清理已自然结束的登记项（防止可重启线程如 notification_listener 反复启停时堆积）
fn prune_finished(reg: &mut Vec<ManagedThread>) {
    reg.retain_mut(|t| match t.handle.as_ref() {
        Some(h) => !h.is_finished(),
        None => true,
    });
}

/// 生成受管线程：登记退出标志与 JoinHandle，线程名为 name（便于调试与 stop_thread）。
/// 返回 [`ExitFlag`] 供调用方按需停止（如 audio_spectrum）。
pub fn spawn_managed<F>(name: &'static str, f: F) -> ExitFlag
where
    F: FnOnce(ExitFlag) + Send + 'static,
{
    let flag = ExitFlag::new();
    let flag_for_thread = flag.clone();
    let handle = std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || f(flag_for_thread))
        .ok();
    let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
    prune_finished(&mut reg);
    reg.push(ManagedThread {
        name,
        flag: flag.clone(),
        handle,
    });
    flag
}

/// 按名称请求线程退出并阻塞等待其结束；线程不存在返回 false。
/// 注意：不得在线程自身内部调用（会 join 自己造成死锁）。
pub fn stop_thread(name: &str) -> bool {
    let thread = {
        let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
        let pos = match reg.iter().position(|t| t.name == name) {
            Some(p) => p,
            None => return false,
        };
        reg.remove(pos)
    };
    thread.flag.signal();
    if let Some(handle) = thread.handle {
        let _ = handle.join();
    }
    true
}
