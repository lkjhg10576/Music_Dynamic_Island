//! 打印队列实时监测（事件驱动，无作业时阻塞挂起，零 CPU 轮询）
//!
//! 使用 windows-sys 0.59 的 Win32::Graphics::Printing（不再依赖 winapi）。
//! Win32 调用已全部收口到 print_utils（plan-2026-08-31 §3.3），句柄 RAII 化：
//! OpenPrinterW → FindFirstPrinterChangeNotification → WaitForMultipleObjects
//! 唤醒后 EnumJobsW 拉全量快照，经节流后 emit `print-queue-tick`。
//! 监控线程接入 thread_mgr 统一退出信号（§4.1），退出时 CloseHandle 释放 stop 事件（§4.3）。

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::print_utils;
use crate::thread_mgr::ExitFlag;
use crate::win32_utils::log_err;

// ──────────────────────────────────────────────
// 契约结构（camelCase，与前端一致）
// ──────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrintJob {
    pub job_id: u32,
    pub document: String,
    pub printer: String,
    pub pages_printed: u32,
    pub total_pages: u32,
    pub position: u32,
    pub status: String,
    pub submitted: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrintQueueState {
    pub has_jobs: bool,
    pub default_printer: String,
    pub jobs: Vec<PrintJob>,
}

impl Default for PrintQueueState {
    fn default() -> Self {
        Self {
            has_jobs: false,
            default_printer: String::new(),
            jobs: Vec::new(),
        }
    }
}

// ──────────────────────────────────────────────
// 全局状态
// ──────────────────────────────────────────────

static PRINTER_MONITOR_ENABLED: AtomicBool = AtomicBool::new(true);
static MONITOR_THREAD_STARTED: AtomicBool = AtomicBool::new(false);
// stop 事件句柄（HANDLE as isize）。改为 Mutex<Option<_>> 以支持线程退出时 CloseHandle（§4.3）
static STOP_EVENT: Mutex<Option<isize>> = Mutex::new(None);
static LAST_STATE: Mutex<PrintQueueState> = Mutex::new(PrintQueueState {
    has_jobs: false,
    default_printer: String::new(),
    jobs: Vec::new(),
});

const THROTTLE_MS: u128 = 1500;
const WAIT_TIMEOUT_MS: u32 = 5000;
const RECONNECT_WAIT_MS: u32 = 5000;

// PRINTER_CHANGE_* (winspool.h)
const PRINTER_CHANGE_ADD_JOB: u32 = 0x0000_0100;
const PRINTER_CHANGE_SET_JOB: u32 = 0x0000_0200;
const PRINTER_CHANGE_DELETE_JOB: u32 = 0x0000_0400;
const PRINTER_CHANGE_WRITE_JOB: u32 = 0x0000_0800;
const PRINTER_CHANGE_JOB: u32 =
    PRINTER_CHANGE_ADD_JOB | PRINTER_CHANGE_SET_JOB | PRINTER_CHANGE_DELETE_JOB | PRINTER_CHANGE_WRITE_JOB;

// JOB_STATUS_*
const JOB_STATUS_PAUSED: u32 = 0x0000_0001;
const JOB_STATUS_ERROR: u32 = 0x0000_0002;
const JOB_STATUS_DELETING: u32 = 0x0000_0004;
const JOB_STATUS_SPOOLING: u32 = 0x0000_0008;
const JOB_STATUS_PRINTING: u32 = 0x0000_0010;
const JOB_STATUS_OFFLINE: u32 = 0x0000_0020;
const JOB_STATUS_PAPEROUT: u32 = 0x0000_0040;
const JOB_STATUS_PRINTED: u32 = 0x0000_0080;
const JOB_STATUS_DELETED: u32 = 0x0000_0100;
const JOB_STATUS_BLOCKED_DEVQ: u32 = 0x0000_0200;
const JOB_STATUS_USER_INTERVENTION: u32 = 0x0000_0400;
const JOB_STATUS_RESTART: u32 = 0x0000_0800;

// WaitForMultipleObjects 返回值
const WAIT_OBJECT_0: u32 = 0x0000_0000;
const WAIT_TIMEOUT: u32 = 0x0000_0102;
const WAIT_FAILED: u32 = 0xFFFF_FFFF;

// ──────────────────────────────────────────────
// 公共 API
// ──────────────────────────────────────────────

/// 启动打印队列监控线程（幂等：重复调用不会生成多个线程）
pub fn start_print_queue_monitor(app: AppHandle) {
    if MONITOR_THREAD_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    // 预创建 stop 事件，供 set_printer_monitor_enabled(false) 立即唤醒
    ensure_stop_event();

    crate::thread_mgr::spawn_managed("print_queue_monitor", move |exit| {
        #[cfg(target_os = "windows")]
        {
            monitor_loop(app, exit);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (app, exit);
            // 非 Windows：线程立即结束（保持编译通过）
        }
    });
}

#[tauri::command]
pub fn set_printer_monitor_enabled(enabled: bool) {
    PRINTER_MONITOR_ENABLED.store(enabled, Ordering::SeqCst);
    // 无论启停都脉冲唤醒监控线程（手动复位事件：Set 后由循环侧 Reset）
    signal_stop_event();
    if !enabled {
        // 清空快照；空队列 emit 由监控线程在退出等待后负责
        let empty = PrintQueueState::default();
        if let Ok(mut guard) = LAST_STATE.lock() {
            *guard = empty;
        }
    }
}

#[tauri::command]
pub fn get_printer_state() -> PrintQueueState {
    LAST_STATE
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

// ──────────────────────────────────────────────
// stop 事件工具
// ──────────────────────────────────────────────

fn ensure_stop_event() -> isize {
    let mut guard = STOP_EVENT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(h) = *guard {
        return h;
    }
    let h = print_utils::create_manual_reset_event() as isize;
    *guard = Some(h);
    h
}

fn signal_stop_event() {
    let h = STOP_EVENT.lock().map(|g| *g).unwrap_or(None);
    if let Some(h) = h {
        if h != 0 {
            print_utils::set_event(h as windows_sys::Win32::Foundation::HANDLE);
        }
    }
}

fn reset_stop_event() {
    let h = STOP_EVENT.lock().map(|g| *g).unwrap_or(None);
    if let Some(h) = h {
        if h != 0 {
            print_utils::reset_event(h as windows_sys::Win32::Foundation::HANDLE);
        }
    }
}

/// 监控线程退出时关闭 stop 事件句柄，杜绝泄漏（§4.3）。
/// 仅在监控线程末尾调用一次；此后不再有 signal/reset 请求路径。
fn close_stop_event() {
    let mut guard = STOP_EVENT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(h) = guard.take() {
        if h != 0 {
            print_utils::close_handle(h as windows_sys::Win32::Foundation::HANDLE);
        }
    }
}

// ──────────────────────────────────────────────
// Windows 监控主循环
// ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn monitor_loop(app: AppHandle, exit: ExitFlag) {
    use windows_sys::Win32::Foundation::HANDLE;

    let stop_handle = ensure_stop_event() as HANDLE;
    let mut last_emit = Instant::now() - Duration::from_secs(10);
    let mut last_jobs_sig: Vec<(u32, u32, u32, String)> = Vec::new(); // (id, pages, total, status)
    let mut last_set_sig: Vec<u32> = Vec::new(); // job ids 集合

    // 启动后先拉一次快照，保证前端晚订阅也能 get_printer_state 恢复
    if PRINTER_MONITOR_ENABLED.load(Ordering::SeqCst) {
        let state = snapshot_queue();
        store_and_emit(&app, &state, true);
        last_jobs_sig = jobs_progress_sig(&state.jobs);
        last_set_sig = jobs_set_sig(&state.jobs);
        last_emit = Instant::now();
    }

    loop {
        if exit.is_exiting() {
            break;
        }

        // 禁用时：清空状态，阻塞等待 re-enable（可被 stop 事件立即唤醒，无忙循环）
        if !PRINTER_MONITOR_ENABLED.load(Ordering::SeqCst) {
            let empty = PrintQueueState::default();
            store_and_emit(&app, &empty, true);
            last_jobs_sig.clear();
            last_set_sig.clear();
            // 先 Reset 掉本次唤醒信号，再阻塞等待下次 SetEvent（启用）或超时复查
            reset_stop_event();
            print_utils::wait_multiple(&[stop_handle], RECONNECT_WAIT_MS);
            // 醒来后清信号，避免下一轮瞬时返回
            reset_stop_event();
            continue;
        }

        // OpenPrinterW(NULL) —— 打开本地打印服务器（RAII：Drop 时 ClosePrinter）
        let Some(printer) = print_utils::PrinterHandle::open_default_server() else {
            // spooler 不可用：可被 stop 打断的阻塞等待后重试，不忙循环
            print_utils::wait_multiple(&[stop_handle], RECONNECT_WAIT_MS);
            continue;
        };

        // FindFirstPrinterChangeNotification（RAII：Drop 时 FindClosePrinterChangeNotification）
        let Some(change) = print_utils::ChangeNotifyHandle::register(&printer, PRINTER_CHANGE_JOB) else {
            print_utils::wait_multiple(&[stop_handle], RECONNECT_WAIT_MS);
            continue;
        };

        // 进入事件等待循环（printer / change 离开作用域时由 RAII 自动关闭）
        loop {
            if exit.is_exiting() || !PRINTER_MONITOR_ENABLED.load(Ordering::SeqCst) {
                break;
            }

            let handles: [HANDLE; 2] = [change.raw(), stop_handle];
            let wait_rc = print_utils::wait_multiple(&handles, WAIT_TIMEOUT_MS);

            // stop 事件（index 1）
            if wait_rc == WAIT_OBJECT_0 + 1 {
                reset_stop_event();
                break;
            }

            // WAIT_FAILED：句柄失效，重建
            if wait_rc == WAIT_FAILED {
                break;
            }

            let from_notification = wait_rc == WAIT_OBJECT_0;
            let from_timeout = wait_rc == WAIT_TIMEOUT;

            if !from_notification && !from_timeout {
                // 其他返回值也重建
                break;
            }

            if from_notification {
                // 必须确认通知，否则不会再触发
                print_utils::find_next_change(&change);
            }

            // 拉全量快照
            let state = snapshot_with_printer(printer.raw());
            let set_sig = jobs_set_sig(&state.jobs);
            let prog_sig = jobs_progress_sig(&state.jobs);

            // 作业集合（id）或状态变化 → 立即；仅页数进度变化 → 1.5s 节流；
            // 5s 超时兜底：有变化则发，无变化只更新共享快照。
            let ids_changed = set_sig != last_set_sig;
            let status_changed = {
                let old_map: std::collections::BTreeMap<u32, &str> =
                    last_jobs_sig.iter().map(|s| (s.0, s.3.as_str())).collect();
                state.jobs.iter().any(|j| {
                    old_map.get(&j.job_id).copied().unwrap_or("") != j.status.as_str()
                })
            };
            let progress_changed = prog_sig != last_jobs_sig;
            let structural = ids_changed || status_changed;
            let only_progress = !structural && progress_changed;

            let should_emit = if structural {
                true
            } else if only_progress {
                last_emit.elapsed().as_millis() >= THROTTLE_MS
            } else if from_timeout && progress_changed {
                // 兜底超时且有差异（含上次节流跳过的进度）
                true
            } else {
                false
            };

            if let Ok(mut guard) = LAST_STATE.lock() {
                *guard = state.clone();
            }

            if should_emit {
                log_err(app.emit("print-queue-tick", &state), "emit print-queue-tick");
                last_emit = Instant::now();
                last_jobs_sig = prog_sig;
                last_set_sig = set_sig;
            }
        }

        // ── printer / change 的 RAII Drop 在此（FindClosePrinterChangeNotification + ClosePrinter）──

        // 禁用时 emit 空队列
        if !PRINTER_MONITOR_ENABLED.load(Ordering::SeqCst) {
            let empty = PrintQueueState::default();
            store_and_emit(&app, &empty, true);
            last_jobs_sig.clear();
            last_set_sig.clear();
        }
        // 否则是 spooler 重启/句柄失效，短暂等待后重建（可被 stop 打断）
        else if !exit.is_exiting() {
            print_utils::wait_multiple(&[stop_handle], 500);
        }
    }

    // 监控线程退出：关闭 stop 事件句柄（§4.3）
    close_stop_event();
}

#[cfg(target_os = "windows")]
fn store_and_emit(app: &AppHandle, state: &PrintQueueState, emit: bool) {
    if let Ok(mut guard) = LAST_STATE.lock() {
        *guard = state.clone();
    }
    if emit {
        log_err(app.emit("print-queue-tick", state), "emit print-queue-tick");
    }
}

#[cfg(target_os = "windows")]
fn jobs_set_sig(jobs: &[PrintJob]) -> Vec<u32> {
    let mut ids: Vec<u32> = jobs.iter().map(|j| j.job_id).collect();
    ids.sort_unstable();
    ids
}

#[cfg(target_os = "windows")]
fn jobs_progress_sig(jobs: &[PrintJob]) -> Vec<(u32, u32, u32, String)> {
    jobs.iter()
        .map(|j| (j.job_id, j.pages_printed, j.total_pages, j.status.clone()))
        .collect()
}

#[cfg(target_os = "windows")]
fn snapshot_queue() -> PrintQueueState {
    match print_utils::PrinterHandle::open_default_server() {
        Some(printer) => {
            // printer 由 RAII 在此作用域结束时 ClosePrinter
            snapshot_with_printer(printer.raw())
        }
        None => PrintQueueState {
            has_jobs: false,
            default_printer: print_utils::get_default_printer(),
            jobs: Vec::new(),
        },
    }
}

#[cfg(target_os = "windows")]
fn snapshot_with_printer(h_printer: windows_sys::Win32::Foundation::HANDLE) -> PrintQueueState {
    let default_printer = print_utils::get_default_printer();
    let mut jobs: Vec<PrintJob> = Vec::new();

    for raw in print_utils::enum_jobs_level1(h_printer) {
        // 过滤已删除作业（部分 spooler 仍短暂保留；正在打印/后台处理的不算已删除）
        if raw.status & JOB_STATUS_DELETED != 0
            && raw.status & JOB_STATUS_PRINTING == 0
            && raw.status & JOB_STATUS_SPOOLING == 0
        {
            continue;
        }
        let status = map_job_status(raw.status, &raw.custom_status);
        jobs.push(PrintJob {
            job_id: raw.job_id,
            document: raw.document,
            printer: raw.printer,
            pages_printed: raw.pages_printed,
            total_pages: raw.total_pages,
            position: raw.position,
            status,
            submitted: raw.submitted_ms,
        });
    }

    // 按队列位置排序
    jobs.sort_by_key(|j| j.position);

    PrintQueueState {
        has_jobs: !jobs.is_empty(),
        default_printer,
        jobs,
    }
}

#[cfg(target_os = "windows")]
fn map_job_status(status: u32, custom: &str) -> String {
    // 优先可读 pStatus 字符串
    if !custom.is_empty() {
        return custom.to_string();
    }
    if status & JOB_STATUS_ERROR != 0 {
        return "错误".into();
    }
    if status & JOB_STATUS_PAPEROUT != 0 {
        return "缺纸".into();
    }
    if status & JOB_STATUS_OFFLINE != 0 {
        return "脱机".into();
    }
    if status & JOB_STATUS_USER_INTERVENTION != 0 {
        return "需要干预".into();
    }
    if status & JOB_STATUS_BLOCKED_DEVQ != 0 {
        return "已阻塞".into();
    }
    if status & JOB_STATUS_PAUSED != 0 {
        return "已暂停".into();
    }
    if status & JOB_STATUS_DELETING != 0 {
        return "删除中".into();
    }
    if status & JOB_STATUS_DELETED != 0 {
        return "已删除".into();
    }
    if status & JOB_STATUS_RESTART != 0 {
        return "重启中".into();
    }
    if status & JOB_STATUS_PRINTING != 0 {
        return "打印中".into();
    }
    if status & JOB_STATUS_SPOOLING != 0 {
        return "后台处理".into();
    }
    if status & JOB_STATUS_PRINTED != 0 {
        return "已完成".into();
    }
    if status == 0 {
        return "排队中".into();
    }
    "处理中".into()
}
