//! 剪贴板链接监听（移植自上游 NetSpeed Dynamic 2.4.5，适配 MDI 架构）。
//!
//! 与上游实现的差异（架构适配）：
//! - 链接提取在后端完成：仅当复制内容含 http/https 链接时才 emit `clipboard-link`
//!   事件给前端（上游每次复制都唤醒岛 WebView 再由前端读剪贴板过滤，非链接复制
//!   不再产生 IPC 唤醒）。
//! - 复用剪贴板历史（clipboard.rs）的自写豁免 seq 集合：面板内复制产生的写回不弹卡片。
//! - 启停经 thread_mgr 登记，可优雅退出；上游为进程级常驻无启停。
//!
//! 实现机制（事件驱动，空闲零 CPU，无轮询线程）：隐藏 message-only 窗口 +
//! `AddClipboardFormatListener`，剪贴板变化时窗口过程收到 `WM_CLIPBOARDUPDATE`。

// 仅 Windows 侧使用（emit 事件）；非 Windows 全为空实现，避免未使用导入告警
#[cfg(target_os = "windows")]
use tauri::Emitter;

// Win32 剪贴板标准格式常量（稳定 API 契约：CF_UNICODETEXT=13），
// 手写规避 windows-sys 常量在 metadata 中的模块路径不确定性（同 clipboard.rs 先例）
#[cfg(target_os = "windows")]
const CF_UNICODETEXT: u32 = 13;
// 剪贴板内容变化通知（AddClipboardFormatListener 的回调消息，Win32 稳定契约）
#[cfg(target_os = "windows")]
const WM_CLIPBOARDUPDATE: u32 = 0x031D;

// 监听窗口句柄（线程创建窗口后写入），供停止路径 PostMessageW(WM_CLOSE) 解阻塞消息循环
#[cfg(target_os = "windows")]
static MONITOR_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
// 功能总开关：关闭时窗口过程直接吞掉通知（先置标记再停线程，杜绝停用后仍弹卡片）
#[cfg(target_os = "windows")]
static ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 从文本中提取第一个 http/https 链接（对齐上游 JS 的 /https?:\/\/[^\s]+/ 匹配 +
/// 尾部标点裁剪；用 ASCII 小写化保证字节下标一致，链接本身必为 ASCII 前缀）
#[cfg(target_os = "windows")]
fn extract_first_link(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let pos = lower
        .find("https://")
        .into_iter()
        .chain(lower.find("http://"))
        .min()?;
    let rest = &text[pos..];
    // 链接遇到首个空白字符即结束（对齐上游 [^\s]+ 语义）
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let link = rest[..end]
        .trim_end_matches(['.', ',', ';', '，', '。', '、', '；']);
    if link.len() > "http://".len() {
        Some(link.to_string())
    } else {
        None
    }
}

/// 读取 CF_UNICODETEXT 文本（与 clipboard.rs::read_text 同模式：NUL 截断 + GlobalSize 上限）
#[cfg(target_os = "windows")]
fn read_clipboard_text() -> Option<String> {
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    unsafe {
        // 剪贴板可能被复制方短暂占用：单次尝试，失败静默放弃（下次复制自然重试）
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let h = GetClipboardData(CF_UNICODETEXT);
        if h.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(h) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        let capacity = GlobalSize(h) as usize / 2;
        let mut len = 0usize;
        while len < capacity && *ptr.add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        GlobalUnlock(h);
        CloseClipboard();
        if text.is_empty() { None } else { Some(text) }
    }
}

/// 剪贴板监听窗口过程：内容变化（WM_CLIPBOARDUPDATE）→ 提取链接 → 推送事件给前端
#[cfg(target_os = "windows")]
unsafe extern "system" fn clipboard_link_wndproc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> isize {
    use windows_sys::Win32::System::DataExchange::RemoveClipboardFormatListener;
    use windows_sys::Win32::UI::WindowsAndMessaging::{DefWindowProcW, PostQuitMessage, WM_DESTROY};

    if msg == WM_CLIPBOARDUPDATE {
        // 双重守卫：总开关关闭 or 自写豁免（剪贴板历史面板复制）→ 静默
        if ENABLED.load(std::sync::atomic::Ordering::SeqCst)
            && !crate::clipboard::has_skip_seq(crate::clipboard::current_seq())
        {
            if let Some(link) = read_clipboard_text().as_deref().and_then(extract_first_link) {
                // 载荷与上游对齐：前端只拿链接本身，无需再读剪贴板
                if let Some(app) = CLIPBOARD_LINK_APP.get() {
                    crate::win32_utils::log_err(
                        app.emit("clipboard-link", serde_json::json!({ "link": link })),
                        "emit clipboard-link",
                    );
                }
            }
        }
        return 0;
    }
    if msg == WM_DESTROY {
        // 销毁前反注册监听，避免悬空窗口继续收通知
        RemoveClipboardFormatListener(hwnd);
        PostQuitMessage(0);
        return 0;
    }
    // WM_CLOSE 不拦：走默认过程 → DestroyWindow → WM_DESTROY（停止路径依赖此链解阻塞消息循环）
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

// 缓存 AppHandle 供窗口过程 emit 使用（OnceLock：全生命周期只设一次）
#[cfg(target_os = "windows")]
static CLIPBOARD_LINK_APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

/// 监听线程主体：注册窗口类 → 建 message-only 窗口 → AddClipboardFormatListener → 消息循环。
/// 退出路径：停止方向 MONITOR_HWND 窗口 PostMessageW(WM_CLOSE)，经默认过程销毁窗口、
/// WM_DESTROY 里 PostQuitMessage 令 GetMessageW 返回 0，线程自然结束（thread_mgr 可 join）。
#[cfg(target_os = "windows")]
fn monitor_thread_body(exit: crate::thread_mgr::ExitFlag) {
    use windows_sys::Win32::System::DataExchange::AddClipboardFormatListener;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DispatchMessageW, GetMessageW, RegisterClassW, TranslateMessage, MSG,
        WNDCLASSW, HWND_MESSAGE,
    };

    let class_name: Vec<u16> = "MdiClipboardLinkListener\0".encode_utf16().collect();
    let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };

    let wc = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(clipboard_link_wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    // 注册/建窗失败则退出监听线程（thread_mgr 的 join 不会悬挂：线程直接返回）
    if unsafe { RegisterClassW(&wc) } == 0 {
        return;
    }
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0, // message-only 窗口不需要样式
            0,
            0,
            0,
            0,
            HWND_MESSAGE, // message-only：不可见、不进任务栏、不被 EnumWindows 枚举
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        )
    };
    if hwnd.is_null() {
        return;
    }
    // 先存句柄再查开关，保证与停止路径的先后序正确（见 set_enabled_impl 注释）
    MONITOR_HWND.store(hwnd as isize, std::sync::atomic::Ordering::SeqCst);
    if !ENABLED.load(std::sync::atomic::Ordering::SeqCst) || exit.is_exiting() {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd) };
        return;
    }
    unsafe { AddClipboardFormatListener(hwnd) };

    // 消息循环：GetMessageW 阻塞等待（零 CPU），WM_CLOSE/WM_DESTROY 路径令其返回 0
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    unsafe {
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    MONITOR_HWND.store(0, std::sync::atomic::Ordering::SeqCst);
}

/// 启停监听（幂等）。enabled=true 建窗监听；false 先置开关再投递 WM_CLOSE 并 join 线程。
#[cfg(target_os = "windows")]
fn set_enabled_impl(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
    if enabled {
        if ENABLED.load(Ordering::SeqCst) {
            return Ok(());
        }
        // 兜底：快速关/开时确保上一条监听线程已退出（与 clipboard_set_enabled 同策略）
        crate::thread_mgr::stop_thread("clipboard_link_monitor");
        let _ = CLIPBOARD_LINK_APP.set(app);
        ENABLED.store(true, Ordering::SeqCst);
        crate::thread_mgr::spawn_managed("clipboard_link_monitor", monitor_thread_body);
        Ok(())
    } else {
        // 先置开关（窗口过程此后吞掉所有通知），再请求销毁窗口解阻塞消息循环，
        // 最后 join。窗口尚未建好时（MONITOR_HWND=0）线程会在建窗后自查开关立即退出
        ENABLED.store(false, Ordering::SeqCst);
        let hwnd = MONITOR_HWND.load(Ordering::SeqCst);
        if hwnd != 0 {
            unsafe {
                PostMessageW(hwnd as _, WM_CLOSE, 0, 0)
            };
        }
        crate::thread_mgr::stop_thread("clipboard_link_monitor");
        Ok(())
    }
}

/// 应用启动时按配置启停（单一数据源：config.json 的 nsd_clipboard_link，默认开启）
pub fn init_from_config(app: &tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    {
        let enabled = crate::config_store::get_bool("nsd_clipboard_link", true);
        if enabled {
            set_enabled_impl(app.clone(), true).ok();
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = app;
}

/// 前端设置开关：启停剪贴板链接监听
#[tauri::command]
pub fn clipboard_link_set_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        set_enabled_impl(app, enabled)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, enabled);
        Ok(())
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::extract_first_link;

    #[test]
    fn 提取纯链接() {
        assert_eq!(
            extract_first_link("https://music.163.com/song?id=1").as_deref(),
            Some("https://music.163.com/song?id=1")
        );
    }

    #[test]
    fn 从分享文本中提取首个链接并裁尾部标点() {
        // 对齐上游 [^\s]+ 语义：链接以空白结束；紧邻空白前的中文标点被裁掉
        // （真实网易云分享文本形如"听听这首《歌》 https://music.163.com/song?id=9 "）
        assert_eq!(
            extract_first_link("听听这首《歌》 https://music.163.com/song?id=9。 很好听").as_deref(),
            Some("https://music.163.com/song?id=9")
        );
    }

    #[test]
    fn 多链接取最先出现者() {
        assert_eq!(
            extract_first_link("看 http://a.com 与 https://b.com").as_deref(),
            Some("http://a.com")
        );
    }

    #[test]
    fn 无链接返回空() {
        assert_eq!(extract_first_link("普通文本"), None);
        assert_eq!(extract_first_link(""), None);
    }
}
