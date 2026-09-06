//! 临时诊断日志（带外 9.9.9-1 专用）：Rust 与前端统一落盘到应用数据目录 `nsd_debug.log`。
//!
//! 目的：定位「灵动岛本体不可点击 / 字符与图标不显示」两类问题——release 构建下
//! `windows_subsystem = "windows"` 使 eprintln 无处可去，必须落文件才能在用户实机上取到证据。
//!
//! 设计约束：
//! - 单文件追加写，每条一行；写入经 Mutex 串行化，崩溃不损坏已写内容；
//! - init 时超过 [`MAX_LOG_BYTES`] 轮转为 `nsd_debug.old.log`（保留上一份），避免无限增长；
//! - 未初始化（init 之前）时 [`write`] 为 no-op，任何路径都不因日志而 panic；
//! - 诊断结束后：本模块、lib.rs 的 `mod debug_log;` 与 `append_frontend_log` 注册、
//!   win32_utils::log_err 的一行调用、以及全部 `debug_log::write` 埋点随 9.9.9-1 一起移除。

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use once_cell::sync::OnceCell;

/// 日志文件上限：init 时超过则轮转为 .old
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
/// 单条消息最大字符数（截断，防止 base64 图片类载荷撑爆日志）
const MAX_MESSAGE_CHARS: usize = 2000;

static LOG_PATH: OnceCell<Option<PathBuf>> = OnceCell::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// 在 setup 最早阶段调用：确定日志路径、必要时轮转、写启动横幅。
pub fn init(app: &tauri::AppHandle) {
    let resolved = crate::storage::app_data_dir(app)
        .ok()
        .map(|dir| dir.join("nsd_debug.log"));
    let _ = LOG_PATH.set(resolved.clone());
    match resolved {
        Some(path) => {
            rotate_if_oversized(&path);
            write(
                "info",
                "boot",
                &format!(
                    "===== NSD v{} 调试日志启动，文件路径: {} =====",
                    env!("CARGO_PKG_VERSION"),
                    path.display()
                ),
            );
        }
        None => {
            eprintln!("[NSD][warn] 调试日志目录解析失败，本次运行不落盘");
        }
    }
}

/// 写入一条日志（level: info/warn/error；tag 为点位标识）。未初始化时静默跳过。
pub fn write(level: &str, tag: &str, message: &str) {
    let Some(path) = LOG_PATH.get().and_then(|p| p.as_ref()) else {
        return;
    };
    let entry = format!(
        "{} [{:<5}] [{:8}] {}\r\n",
        local_timestamp(),
        level,
        tag,
        sanitize(message)
    );
    // 打开-追加-关闭：崩溃安全优先；调试日志频度低，无需常驻句柄
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(entry.as_bytes());
    }
    eprintln!("[NSD:{}] [{}] {}", tag, level, message);
}

/// 单行化 + 截断：换行转为可见转义，超长截断并标注原始长度
fn sanitize(message: &str) -> String {
    let mut line = message.replace('\r', "\\r").replace('\n', "\\n");
    let char_count = line.chars().count();
    if char_count > MAX_MESSAGE_CHARS {
        line = line.chars().take(MAX_MESSAGE_CHARS).collect();
        line.push_str(&format!("…(截断,原长{}字符)", char_count));
    }
    line
}

/// 超限轮转：nsd_debug.log → nsd_debug.old.log（覆盖旧 .old）
fn rotate_if_oversized(path: &PathBuf) {
    let oversized = std::fs::metadata(path)
        .map(|m| m.len() > MAX_LOG_BYTES)
        .unwrap_or(false);
    if !oversized {
        return;
    }
    let mut old = path.clone();
    old.set_extension("old.log");
    let _ = std::fs::remove_file(&old);
    let _ = std::fs::rename(path, &old);
}

/// 本地时间戳（含毫秒）。GetLocalTime 用法与 storage::local_date_string 同源（windows-sys，
/// 所需 feature Win32_System_SystemInformation / Win32_Foundation 均已在 Cargo.toml 启用）。
fn local_timestamp() -> String {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut st) };
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, st.wMilliseconds
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        // 云构建仅面向 Windows；非 Windows 平台退化为 UTC 秒级占位
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("epoch+{}s", secs)
    }
}

/// 前端日志通道：与 Rust 侧写入同一文件（前端在 message 里自带窗口标签前缀）
#[tauri::command]
pub fn append_frontend_log(level: String, tag: String, message: String) {
    write(&level, &tag, &message);
}
