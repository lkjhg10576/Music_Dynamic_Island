//! 打印队列 Win32 调用集中封装（plan-2026-08-31 §3.3 / §4.3 / §5.1）。
//!
//! winspool / 同步原语的 unsafe 全部收口到本模块；打印机与变更通知句柄一律 RAII
//! （Drop 时 ClosePrinter / FindClosePrinterChangeNotification），事件句柄的
//! CloseHandle 由 print_queue 侧在监控线程退出时显式调用。
//! 统一使用 windows-sys 0.59，不再依赖 winapi 0.3。

use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE, SYSTEMTIME};
use windows_sys::Win32::Graphics::Printing::{
    self as printing, FindClosePrinterChangeNotification, FindFirstPrinterChangeNotification,
    FindNextPrinterChangeNotification, GetDefaultPrinterW, JOB_INFO_1W,
};
use windows_sys::Win32::System::Time::SystemTimeToFileTime;
use windows_sys::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects,
};

// ──────────────────────────────────────────────
// RAII 句柄
// ──────────────────────────────────────────────

/// 打印服务器句柄（OpenPrinterW(NULL) 打开本地打印服务器），Drop 时 ClosePrinter
pub struct PrinterHandle(HANDLE);

impl PrinterHandle {
    pub fn open_default_server() -> Option<Self> {
        // SAFETY: OpenPrinterW(NULL, ...) opens the local print server; the returned
        // handle is stored in this RAII wrapper and closed exactly once in Drop.
        let mut h: HANDLE = std::ptr::null_mut();
        let ok = unsafe { printing::OpenPrinterW(std::ptr::null_mut(), &mut h, std::ptr::null_mut()) };
        if ok == 0 || h.is_null() {
            None
        } else {
            Some(Self(h))
        }
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for PrinterHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: handle is valid and will not be used after this call.
            unsafe {
                printing::ClosePrinter(self.0);
            }
        }
    }
}

/// 打印机变更通知句柄（FindFirstPrinterChangeNotification），
/// Drop 时 FindClosePrinterChangeNotification
pub struct ChangeNotifyHandle(HANDLE);

impl ChangeNotifyHandle {
    pub fn register(printer: &PrinterHandle, change_mask: u32) -> Option<Self> {
        // SAFETY: `printer` holds a valid open printer handle; the returned change
        // notification handle is stored in this RAII wrapper and closed exactly once in Drop.
        let h = unsafe {
            FindFirstPrinterChangeNotification(printer.raw(), change_mask, 0, std::ptr::null_mut())
        };
        if h.is_null() || h == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(h))
        }
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for ChangeNotifyHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: handle is a valid change-notification handle, closed exactly once.
            unsafe {
                FindClosePrinterChangeNotification(self.0);
            }
        }
    }
}

// ──────────────────────────────────────────────
// 同步原语
// ──────────────────────────────────────────────

/// 创建手动复位、初始未信号的事件；失败返回 null
pub fn create_manual_reset_event() -> HANDLE {
    // SAFETY: CreateEventW with bManualReset=TRUE, bInitialState=FALSE, unnamed event.
    unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) }
}

pub fn set_event(h: HANDLE) {
    // SAFETY: caller guarantees `h` is a valid (open) event handle.
    unsafe {
        SetEvent(h);
    }
}

pub fn reset_event(h: HANDLE) {
    // SAFETY: caller guarantees `h` is a valid (open) event handle.
    unsafe {
        ResetEvent(h);
    }
}

/// 关闭事件句柄；调用方必须保证此后不再使用该句柄
pub fn close_handle(h: HANDLE) {
    // SAFETY: caller guarantees `h` is valid and will not be used afterwards.
    unsafe {
        CloseHandle(h);
    }
}

/// WaitForMultipleObjects 封装；返回原始 wait 代码（WAIT_OBJECT_0 / WAIT_TIMEOUT / WAIT_FAILED 等）
pub fn wait_multiple(handles: &[HANDLE], timeout_ms: u32) -> u32 {
    // SAFETY: `handles` holds valid waitable handles, count within API limits
    // (<= MAXIMUM_WAIT_OBJECTS).
    unsafe {
        WaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), 0, timeout_ms)
    }
}

/// 确认变更通知（必须在每次 WAIT_OBJECT_0 唤醒后调用，否则不会再触发）
pub fn find_next_change(h: &ChangeNotifyHandle) -> u32 {
    // SAFETY: `h` is a valid change-notification handle returned by
    // FindFirstPrinterChangeNotification; args follow the documented contract.
    let mut change: u32 = 0;
    unsafe {
        FindNextPrinterChangeNotification(
            h.raw(),
            &mut change,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
    change
}

// ──────────────────────────────────────────────
// 打印队列快照
// ──────────────────────────────────────────────

/// EnumJobsW(JOB_INFO_1) 解析后的单条作业原始信息（状态 → 文案映射归 print_queue 业务层）
pub struct JobRaw {
    pub job_id: u32,
    pub document: String,
    pub printer: String,
    pub status: u32,
    pub custom_status: String,
    pub position: u32,
    pub total_pages: u32,
    pub pages_printed: u32,
    pub submitted_ms: u64,
}

/// 两阶段 EnumJobsW 拉取全部作业快照
pub fn enum_jobs_level1(printer: HANDLE) -> Vec<JobRaw> {
    // SAFETY: two-phase EnumJobsW — the first probe call is expected to fail with
    // ERROR_INSUFFICIENT_BUFFER and only reports `needed`; the second call receives
    // the JOB_INFO_1W array into a buffer of exactly `needed` bytes. Pointer reads
    // are limited to the `returned` count reported by the successful call.
    let mut needed: u32 = 0;
    let mut returned: u32 = 0;
    unsafe {
        printing::EnumJobsW(
            printer,
            0,
            0xFFFF_FFFF,
            1, // JOB_INFO_1
            std::ptr::null_mut(),
            0,
            &mut needed,
            &mut returned,
        );
    }

    let mut jobs: Vec<JobRaw> = Vec::new();
    if needed == 0 {
        return jobs;
    }

    let mut buf = vec![0u8; needed as usize];
    let ok = unsafe {
        printing::EnumJobsW(
            printer,
            0,
            0xFFFF_FFFF,
            1,
            buf.as_mut_ptr() as *mut _,
            needed,
            &mut needed,
            &mut returned,
        )
    };
    if ok == 0 || returned == 0 {
        return jobs;
    }

    let base = buf.as_ptr() as *const JOB_INFO_1W;
    for i in 0..returned as isize {
        let info = unsafe { &*base.offset(i) };
        jobs.push(JobRaw {
            job_id: info.JobId,
            document: wide_ptr_to_string(info.pDocument),
            printer: wide_ptr_to_string(info.pPrinterName),
            status: info.Status,
            custom_status: wide_ptr_to_string(info.pStatus),
            position: info.Position,
            total_pages: info.TotalPages,
            pages_printed: info.PagesPrinted,
            submitted_ms: systemtime_to_unix_ms(&info.Submitted),
        });
    }
    jobs
}

/// 当前默认打印机名；失败返回空串
pub fn get_default_printer() -> String {
    // SAFETY: two-phase GetDefaultPrinterW — the probe reports the required size,
    // the second call receives the string into a buffer of exactly that size.
    let mut needed: u32 = 0;
    unsafe {
        GetDefaultPrinterW(std::ptr::null_mut(), &mut needed);
    }
    if needed == 0 {
        return String::new();
    }
    let mut buf = vec![0u16; needed as usize];
    let ok = unsafe { GetDefaultPrinterW(buf.as_mut_ptr(), &mut needed) };
    if ok == 0 {
        return String::new();
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// 防御性 UTF-16 指针转字符串（上限 4096 单元，避免异常指针死循环）
pub(crate) fn wide_ptr_to_string(ptr: *mut u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: `ptr` points to a NUL-terminated UTF-16 string owned by the spooler
    // buffer, valid for the duration of this call; the scan is capped at 4096 units.
    unsafe {
        let mut len = 0usize;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }
}

/// SYSTEMTIME → unix 毫秒（FILETIME 中转，1601→1970 偏移）；失败返回 0
pub(crate) fn systemtime_to_unix_ms(st: &SYSTEMTIME) -> u64 {
    // SAFETY: `st` points to a valid SYSTEMTIME within the spooler buffer;
    // ft is a zero-initialized stack value.
    unsafe {
        let mut ft: FILETIME = std::mem::zeroed();
        if SystemTimeToFileTime(st as *const SYSTEMTIME, &mut ft) == 0 {
            return 0;
        }
        let ticks = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
        // FILETIME: 100-ns intervals since 1601-01-01 UTC
        const EPOCH_DIFF_100NS: u64 = 11644473600u64 * 10_000_000;
        if ticks < EPOCH_DIFF_100NS {
            return 0;
        }
        (ticks - EPOCH_DIFF_100NS) / 10_000 // → 毫秒
    }
}
