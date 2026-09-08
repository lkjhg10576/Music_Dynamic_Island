//! 任务栏全局进度监测（UIA 轮询模式，零 hook）
//!
//! 数据源: explorer.exe 把 taskbar 上的进度条作为 UIA 元素暴露，
//! 通过查找 Shell_TrayWnd → MSTaskListWClass → RangeValuePattern 即可。
//! 调度: 经 thread_mgr 登记，可优雅退出；停止后清理 COM 句柄。

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::thread_mgr;
use crate::win32_utils::{log_err, ComGuard};

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

            // 扫描一次
            let scan = scan_taskbar_progress();
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
    let scan = scan_taskbar_progress();
    TaskbarProgressState { ts: now_secs(), ..scan }
}

// ──────────────────────────────────────────────
// UIA 扫描函数
// ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn scan_taskbar_progress() -> TaskbarProgressState {
    use windows::UI::Automation::*;
    use windows_core::Interface;

    // 1. 获取 UIA root, 找 taskbar 窗口
    let Ok(root) = AutomationElement::GetRootElement() else { return inactive(); };

    let Ok(taskbar_cond) = PropertyCondition::Create(
        AutomationElement::ClassNameProperty(),
        "Shell_TrayWnd".into(),
    ) else { return inactive(); };

    let Ok(taskbar) = root.FindFirst(TreeScope_Children, &taskbar_cond) else {
        return inactive();
    };

    // 2. 找 taskbar 内的图标列表容器(MSTaskListWClass)
    let list_element = PropertyCondition::Create(
        AutomationElement::ClassNameProperty(),
        "MSTaskListWClass".into(),
    ).ok().and_then(|cond| taskbar.FindFirst(TreeScope_Descendants, &cond).ok());
    let root_for_scan = list_element.unwrap_or(taskbar);

    // 3. 找所有支持 RangeValuePattern 的元素
    let Ok(progress_cond) = PropertyCondition::Create(
        AutomationElement::IsRangeValuePatternAvailableProperty(),
        true.into(),
    ) else { return inactive(); };
    let Ok(elements) = root_for_scan.FindAll(TreeScope_Descendants, &progress_cond) else {
        return inactive();
    };

    // 4. 取第一个有效进度条(0 < percent < 100)
    for i in 0..elements.Length().unwrap_or(0) {
        let Ok(el) = elements.GetElement(i) else { continue; };
        let Ok(pattern_obj) = el.GetCurrentPattern(RangeValuePattern::IID()) else { continue; };
        let Ok(pattern) = pattern_obj.cast::<IRangeValueProvider>() else { continue; };

        let Ok(value) = pattern.Value() else { continue; };
        let Ok(maximum) = pattern.Maximum() else { continue; };

        if maximum <= 0.0 { continue; }
        let pct_f = (value / maximum) * 100.0;
        if pct_f <= 0.0 || pct_f >= 100.0 { continue; }
        let pct = pct_f as u8;

        // 提取 app_name
        let app_name = el.CurrentName()
            .map(|s| s.to_string_lossy())
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
