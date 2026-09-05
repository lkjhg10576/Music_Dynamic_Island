//! Win32 / WinRT 调用集中封装（plan-2026-08-31 §3.2 / §3.3 / §3.4）。
//!
//! 原则（跨批次通用）：
//! - 不减少 unsafe 总量，但把散落各处的 unsafe 收口到本模块，集中审计；
//! - 每个封装函数带英文 `// SAFETY:` 注释说明前置条件；
//! - COM 初始化通过 [`ComGuard`] RAII 保证 CoInitializeEx / CoUninitialize 严格配对；
//! - Result 类错误禁止静默吞掉，统一经 [`log_err`] 打 warn。

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging as winuser;
// 显示器相关 API 在 windows-sys 中归属 Gdi 模块（非 WindowsAndMessaging）；
// MessageBeep 已从 windows-sys 0.59 移除，按项目规范走 winapi 0.3。
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTONEAREST, MONITORINFO,
};
use winapi::um::winuser::MessageBeep;

// ──────────────────────────────────────────────
// COM RAII（§3.2）
// ──────────────────────────────────────────────

/// COM 初始化 RAII 守卫：`new()` 调用 `CoInitializeEx(COINIT_MULTITHREADED)`，
/// `Drop` 时 `CoUninitialize()` 严格配对。
/// S_OK / S_FALSE 均需配对释放；RPC_E_CHANGED_MODE 等失败（is_ok() == false）不释放。
pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    pub fn new() -> Self {
        let hr = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
        Self {
            initialized: hr.is_ok(),
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: paired with the successful CoInitializeEx in new()
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        }
    }
}

impl Default for ComGuard {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// 统一错误出口（§3.4）
// ──────────────────────────────────────────────

/// Result 错误统一出口：Err 时打 warn（stderr），Ok（含任意载荷，如 LyricEntry）静默。
/// 用于替换 `let _ = ...` 式的静默忽略；事件 emit / 文件操作等路径均适用。
pub(crate) fn log_err<T, E: std::fmt::Display>(res: Result<T, E>, ctx: &str) {
    if let Err(e) = res {
        eprintln!("[NSD][warn] {}: {}", ctx, e);
    }
}

// ──────────────────────────────────────────────
// 窗口管理封装（windows-sys 0.59）
// ──────────────────────────────────────────────

/// SWP_NOZORDER | SWP_NOACTIVATE：同时改位置尺寸时不抢焦点、不动层级
const SWP_POS_NO_ACTIVATE: u32 = 0x0014;
/// SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE（置顶专用，保留原实现常量 19）
const SWP_TOPMOST_FLAGS: u32 = 19;

/// 修改窗口位置与尺寸（不抢焦点、不打乱 Z 序）
pub fn set_window_pos_no_activate(hwnd: isize, x: i32, y: i32, w: i32, h: i32) {
    // SAFETY: `hwnd` comes from Tauri `window.hwnd()` (a valid window handle) and is
    // only used for this single SetWindowPos call; coords/flags are plain values.
    unsafe {
        winuser::SetWindowPos(
            hwnd as HWND,
            std::ptr::null_mut(),
            x, y, w, h,
            SWP_POS_NO_ACTIVATE,
        );
    }
}

/// 将窗口提到最顶层（HWND_TOPMOST 占位 -1）
pub fn set_window_pos_topmost(hwnd: isize) {
    // SAFETY: same precondition as set_window_pos_no_activate; the -1 cast is the
    // documented HWND_TOPMOST placeholder value.
    unsafe {
        winuser::SetWindowPos(hwnd as HWND, -1isize as HWND, 0, 0, 0, 0, SWP_TOPMOST_FLAGS);
    }
}

/// 读取窗口屏幕矩形；失败（句柄失效）返回 None
pub fn get_window_rect(hwnd: isize) -> Option<RECT> {
    // SAFETY: `hwnd` is a valid window handle; rect is a zero-initialized stack value.
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    let ok = unsafe { winuser::GetWindowRect(hwnd as HWND, &mut rect) };
    if ok != 0 {
        Some(rect)
    } else {
        None
    }
}

/// 读取窗口所在显示器（最近）的矩形；失败返回 None
pub fn monitor_rect_of(hwnd: isize) -> Option<RECT> {
    // SAFETY: `hwnd` is a valid window handle; MONITORINFO.cbSize is set per API contract.
    unsafe {
        let monitor = MonitorFromWindow(hwnd as HWND, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut mi) != 0 {
            Some(mi.rcMonitor)
        } else {
            None
        }
    }
}

/// 前台窗口句柄（可能为 0，调用方需判空）
pub fn foreground_window() -> isize {
    // SAFETY: parameterless query; may return NULL which callers must check.
    unsafe { winuser::GetForegroundWindow() as isize }
}

/// 读取窗口类名；失败返回空串
pub fn get_class_name(hwnd: isize) -> String {
    // SAFETY: `hwnd` is a valid window handle; buffer is a fixed 256-unit stack array,
    // GetClassNameW truncates and returns the copied length per documentation.
    unsafe {
        let mut buf = [0u16; 256];
        let len = winuser::GetClassNameW(hwnd as HWND, buf.as_mut_ptr(), buf.len() as i32);
        if len <= 0 {
            String::new()
        } else {
            String::from_utf16_lossy(&buf[..len as usize])
        }
    }
}

/// force_window_topmost 前置检查：前台窗口是否属于「不该打扰置顶」的场景
/// （右键菜单 #32768 / 覆盖整屏的非桌面窗口）
pub fn should_skip_topmost() -> bool {
    let fg = foreground_window();
    if fg == 0 {
        return false;
    }
    let class_str = get_class_name(fg);
    if class_str == "#32768" {
        return true;
    }
    let Some(rect) = get_window_rect(fg) else {
        return false;
    };
    let Some(monitor) = monitor_rect_of(fg) else {
        return false;
    };
    let covers_monitor = rect.left == monitor.left
        && rect.top == monitor.top
        && rect.right == monitor.right
        && rect.bottom == monitor.bottom;
    covers_monitor && class_str != "Progman" && class_str != "WorkerW"
}

/// 全屏检测（自 lib.rs 全屏检测线程收口）：前台窗口是否覆盖其所在显示器
/// （排除桌面/外壳窗口与已知系统 UI 弹层）
pub fn is_foreground_fullscreen() -> bool {
    // SAFETY: all calls are read-only Win32 queries on handles obtained from system
    // queries (GetForegroundWindow / GetShellWindow); buffers are fixed stack arrays.
    unsafe {
        let fg = winuser::GetForegroundWindow();
        if fg.is_null() {
            return false;
        }
        let shell = winuser::GetShellWindow();
        if fg == winuser::GetDesktopWindow() || fg == shell {
            return false;
        }

        let mut shell_pid = 0;
        if !shell.is_null() {
            winuser::GetWindowThreadProcessId(shell, &mut shell_pid);
        }
        let mut fg_pid = 0;
        winuser::GetWindowThreadProcessId(fg, &mut fg_pid);
        if shell_pid != 0 && fg_pid == shell_pid {
            return false; // 系统外壳组件
        }

        let style = winuser::GetWindowLongPtrW(fg, winuser::GWL_STYLE) as u32;
        let ex_style = winuser::GetWindowLongPtrW(fg, winuser::GWL_EXSTYLE) as u32;
        if (style & winuser::WS_CHILD) != 0 || (ex_style & winuser::WS_EX_TRANSPARENT) != 0 {
            return false;
        }

        let mut class_name = [0u16; 256];
        let len = winuser::GetClassNameW(fg, class_name.as_mut_ptr(), class_name.len() as i32);
        let class_str = if len > 0 {
            String::from_utf16_lossy(&class_name[..len as usize])
        } else {
            String::new()
        };
        let is_blacklisted = class_str.contains("Windows.UI.Core.CoreWindow")
            || class_str.contains("Xaml_WindowedPopupClass")
            || class_str.contains("SearchApp")
            || class_str.contains("NotifyIconOverflowWindow");
        if is_blacklisted {
            return false;
        }

        let mut rect: RECT = std::mem::zeroed();
        if winuser::GetWindowRect(fg, &mut rect) == 0 {
            return false;
        }
        match monitor_rect_of(fg as isize) {
            Some(m) => {
                rect.left <= m.left
                    && rect.top <= m.top
                    && rect.right >= m.right
                    && rect.bottom >= m.bottom
            }
            None => false,
        }
    }
}

/// 播放系统提示音（MB_ICONEXCLAMATION 等音效类别常量）
pub fn message_beep(kind: u32) {
    // SAFETY: single system sound request with a valid sound-type constant.
    unsafe {
        MessageBeep(kind);
    }
}

/// 读取系统电源状态：(ACLineStatus, BatteryLifePercent)。
/// ACLineStatus: 0=使用电池 1=已接电源；BatteryLifePercent: 0~100，无电池（台式机）为 255。
/// 共享封装：system_events（电量提醒）与 lib.rs 硬件监控线程（monitor-stats 电池环）共用，
/// 避免两处各自轮询 GetSystemPowerStatus。
pub fn power_status() -> Option<(u8, u8)> {
    // SAFETY: SYSTEM_POWER_STATUS is a plain output struct zeroed before the call.
    unsafe {
        let mut status: windows::Win32::System::Power::SYSTEM_POWER_STATUS = std::mem::zeroed();
        if windows::Win32::System::Power::GetSystemPowerStatus(&mut status).is_ok() {
            Some((status.ACLineStatus, status.BatteryLifePercent))
        } else {
            None
        }
    }
}
