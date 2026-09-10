//! 任务栏全局进度监测（UIA 轮询模式，零 hook）
//!
//! 数据源: explorer.exe 把 taskbar 上的进度条作为 UIA 元素暴露，
//! 通过查找 Shell_TrayWnd → MSTaskListWClass → RangeValuePattern 即可。
//! 调度: 经 thread_mgr 登记，可优雅退出；停止后清理 COM 句柄。

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::thread_mgr;
use crate::win32_utils::ComGuard;

/// 任务栏进度 tick 载荷(与前端 camelCase 一致)
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarProgressState {
    pub active: bool,
    pub app_name: String,
    pub percent: u8,           // 0-100, 0 表示无活动
    pub ts: u64,               // unix seconds
}

// 全局状态
static MONITOR_ENABLED: AtomicBool = AtomicBool::new(true);
static MONITOR_STARTED: AtomicBool = AtomicBool::new(false);
// 扫描频率上下限: 500ms ~ 3000ms(用户确认)
static SCAN_INTERVAL_MS: AtomicU32 = AtomicU32::new(1000);  // 默认 1s
const SCAN_INTERVAL_MIN_MS: u32 = 500;
const SCAN_INTERVAL_MAX_MS: u32 = 3000;

/// 开发者桥接注入的假进度: Some 时监控线程与状态查询一律用它，跳过 UIA 扫描。
/// 真实场景（浏览器下载 / 文件复制 / 解压）的进度既不可控也不可复现，
/// 想验证"岛上的进度条在某百分比渲染成什么样"就必须能直接喂值。
static INJECT_OVERRIDE: Mutex<Option<TaskbarProgressState>> = Mutex::new(None);

/// 取当前有效状态：注入优先，否则真扫一次
fn effective_scan() -> TaskbarProgressState {
    let injected = INJECT_OVERRIDE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    match injected {
        Some(s) => s,
        None => scan_taskbar_progress(),
    }
}

/// 启动后台扫描线程
pub fn start_taskbar_progress_monitor(app: AppHandle) {
    if MONITOR_STARTED.swap(true, Ordering::Relaxed) {
        return;  // 幂等
    }
    thread_mgr::spawn_managed("taskbar_progress", move |exit| {
        // UIA 调用需要 COM 套间
        let _com = ComGuard::new();

        let mut last_active = false;
        let mut last_app = String::new();
        let mut last_pct: u8 = 0;
        let mut last_emit_at: u64 = 0;  // ms 截断,用于去重

        while !exit.is_exiting() {
            if !MONITOR_ENABLED.load(Ordering::Relaxed) {
                // 关闭时: 仅在状态切换时 emit 一次 inactive, 然后长时间休眠
                if last_active {
                    let _ = app.emit("taskbar-progress-tick", TaskbarProgressState {
                        active: false, app_name: String::new(), percent: 0,
                        ts: now_secs(),
                    });
                    last_active = false;
                    last_app.clear();
                    last_pct = 0;
                }
                if exit.sleep_interruptible(Duration::from_millis(3000)) { break; }
                continue;
            }

            // 扫描一次（注入覆盖时直接取注入值）
            let scan = effective_scan();
            let now_ms = now_millis();

            // 去重 + 节流: 状态完全没变, 且距上次 emit < throttle, 就跳过
            let state_changed = scan.active != last_active
                || scan.app_name != last_app
                || scan.percent != last_pct;
            let throttle_ms = SCAN_INTERVAL_MS.load(Ordering::Relaxed) as u64;
            if state_changed || now_ms.saturating_sub(last_emit_at) >= throttle_ms {
                let _ = app.emit("taskbar-progress-tick", &scan);
                last_active = scan.active;
                last_app = scan.app_name.clone();
                last_pct = scan.percent;
                last_emit_at = now_ms;
            }

            // 扫描间隔
            let interval = SCAN_INTERVAL_MS.load(Ordering::Relaxed) as u64;
            if exit.sleep_interruptible(Duration::from_millis(interval)) { break; }
        }
    });
}

#[tauri::command]
pub fn set_taskbar_progress_enabled(enabled: bool) {
    MONITOR_ENABLED.store(enabled, Ordering::Relaxed);
}

#[tauri::command]
pub fn set_taskbar_progress_interval(ms: u32) {
    SCAN_INTERVAL_MS.store(ms.clamp(SCAN_INTERVAL_MIN_MS, SCAN_INTERVAL_MAX_MS), Ordering::Relaxed);
}

#[tauri::command]
pub fn get_taskbar_progress_state() -> TaskbarProgressState {
    let scan = effective_scan();
    TaskbarProgressState { ts: now_secs(), ..scan }
}

// ══════════════════════════════════════════════
// 开发者桥接接口（仅供 dev_bridge 调用）
// ══════════════════════════════════════════════

/// 注入 / 清除假的任务栏进度。state 为 None 表示清除注入、恢复真实 UIA 扫描。
/// 立即推一次 tick，使岛上与状态查询同步生效（不必等下一个扫描周期）。
pub(crate) fn dev_set_override(app: &AppHandle, state: Option<TaskbarProgressState>) {
    let payload = match state {
        Some(s) => {
            let s = TaskbarProgressState { ts: now_secs(), ..s };
            *INJECT_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) = Some(s.clone());
            s
        }
        None => {
            *INJECT_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) = None;
            inactive()
        }
    };
    crate::win32_utils::log_err(
        app.emit("taskbar-progress-tick", payload),
        "emit taskbar-progress-tick (dev)",
    );
}

/// 桥接用状态快照（含注入标记与最近一次真实扫描结果）
pub(crate) fn dev_status() -> serde_json::Value {
    let injected = INJECT_OVERRIDE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some();
    // UIA 调用需要 COM 套间：桥接连接线程不是监控线程，必须自己初始化一次，
    // 否则 CoCreateInstance 直接返回 CO_E_NOTINITIALIZED，真扫结果恒为 inactive。
    let _com = ComGuard::new();
    let real = scan_taskbar_progress();
    serde_json::json!({
        "enabled": MONITOR_ENABLED.load(Ordering::Relaxed),
        "started": MONITOR_STARTED.load(Ordering::Relaxed),
        "intervalMs": SCAN_INTERVAL_MS.load(Ordering::Relaxed),
        "injected": injected,
        "realScan": real,
    })
}

// ──────────────────────────────────────────────
// UIA 扫描函数
// ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn scan_taskbar_progress() -> TaskbarProgressState {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationRangeValuePattern, TreeScope_Children,
        TreeScope_Descendants, UIA_ClassNamePropertyId,
        UIA_IsRangeValuePatternAvailablePropertyId, UIA_RangeValuePatternId,
    };
    use windows::core::{BSTR, VARIANT};

    // SAFETY: 本函数只在监控线程内调用，该线程入口已由 ComGuard 完成
    // CoInitializeEx(COINIT_MULTITHREADED)；以下均为同线程同步 COM 调用，
    // 接口指针在函数返回前全部释放。
    unsafe {
        // 1. 创建 UIA 实例, 取 root, 找 taskbar 窗口
        let uia: IUIAutomation = match CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) {
            Ok(uia) => uia,
            Err(_) => return inactive(),
        };
        let Ok(root) = uia.GetRootElement() else { return inactive(); };

        // VARIANT 非 Copy，windows-core 只为 &VARIANT 实现 Param<VARIANT>，
        // 故这里必须传引用，否则 E0277。
        let Ok(taskbar_cond) =
            uia.CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from(BSTR::from("Shell_TrayWnd")))
        else {
            return inactive();
        };

        let Ok(taskbar) = root.FindFirst(TreeScope_Children, &taskbar_cond) else {
            return inactive();
        };

        // 2. 找 taskbar 内的图标列表容器(MSTaskListWClass)
        let list_element = uia
            .CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from(BSTR::from("MSTaskListWClass")))
            .ok()
            .and_then(|cond| taskbar.FindFirst(TreeScope_Descendants, &cond).ok());
        let root_for_scan = list_element.unwrap_or(taskbar);

        // 3. 找所有支持 RangeValuePattern 的元素
        let Ok(progress_cond) = uia.CreatePropertyCondition(
            UIA_IsRangeValuePatternAvailablePropertyId,
            &VARIANT::from(true),
        ) else {
            return inactive();
        };
        let Ok(elements) = root_for_scan.FindAll(TreeScope_Descendants, &progress_cond) else {
            return inactive();
        };

        // 4. 取第一个有效进度条(0 < percent < 100)
        for i in 0..elements.Length().unwrap_or(0) {
            let Ok(el) = elements.GetElement(i) else { continue; };
            let Ok(pattern) =
                el.GetCurrentPatternAs::<IUIAutomationRangeValuePattern>(UIA_RangeValuePatternId)
            else {
                continue;
            };

            let Ok(value) = pattern.CurrentValue() else { continue; };
            let Ok(maximum) = pattern.CurrentMaximum() else { continue; };

            if maximum <= 0.0 { continue; }
            let pct_f = (value / maximum) * 100.0;
            if pct_f <= 0.0 || pct_f >= 100.0 { continue; }
            let pct = pct_f as u8;

            // 提取 app_name
            let app_name = el.CurrentName()
                .map(|s| s.to_string())
                .unwrap_or_default();

            return TaskbarProgressState {
                active: true,
                app_name,
                percent: pct,
                ts: now_secs(),
            };
        }
        inactive()
    }
}

#[cfg(not(target_os = "windows"))]
fn scan_taskbar_progress() -> TaskbarProgressState {
    inactive()
}

// ──────────────────────────────────────────────
// 辅助函数
// ──────────────────────────────────────────────

fn inactive() -> TaskbarProgressState {
    TaskbarProgressState {
        active: false,
        app_name: String::new(),
        percent: 0,
        ts: now_secs(),
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
