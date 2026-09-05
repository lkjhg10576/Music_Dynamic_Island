//! 日程同步（plan-2026-09-05 阶段 F）：把"实时活动"从相对时间（倒计时）扩展到绝对时间（日程）。
//!
//! 数据源两路合并：
//! - 系统日历：WinRT `AppointmentManager::RequestStoreAsync(AllCalendarsReadOnly)` 只读查询
//!   未来 24h 日程，每 5 分钟重查一次；未打包应用常见 `E_ACCESSDENIED` 时自动降级
//!   （system_ok=false，仅手动提醒生效，不打断其余功能）；
//! - 手动提醒：前端添加的一次性 / 每日重复提醒，持久化到 config.json（设置单一数据源），
//!   无系统日历权限也能用。
//!
//! 后台线程每秒轻量重算"未来 24h 内的日程列表"，列表变化或每 30 秒 emit `calendar-tick`
//! （"还有 X 分钟"为分钟级精度）；前端经 `calendar_get_state` 做窗口重建后的状态恢复。

#[cfg(target_os = "windows")]
use windows::ApplicationModel::Appointments::{AppointmentManager, AppointmentStoreAccessType};
#[cfg(target_os = "windows")]
use windows::Foundation::{DateTime, TimeSpan};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// 手动提醒持久化键（config.json 单一数据源，前端经 calendar_* 命令增删）
const MANUAL_KEY: &str = "nsd_calendar_manual_events";
/// 查询与展示窗口：未来 24 小时
const HORIZON_SECS: u64 = 24 * 3600;
/// 系统日历重查间隔：5 分钟
const SYS_QUERY_INTERVAL_SECS: u64 = 300;
/// tick 推送间隔：30 秒（倒计时文案为分钟级精度，足够新鲜）
const EMIT_INTERVAL_SECS: u64 = 30;
/// unix 纪元（1970-01-01）与 FILETIME 纪元（1601-01-01）的秒差
const UNIX_TO_FILETIME_EPOCH_SECS: i64 = 11_644_473_600;
/// FILETIME 的计时单位：100 纳秒
const FILETIME_TICKS_PER_SEC: i64 = 10_000_000;
/// 岛上/控制台最多展示的日程条数
const MAX_UPCOMING: usize = 8;

/// 手动提醒条目（config.json 持久化结构）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ManualEvent {
    pub id: u64,
    pub title: String,
    /// 绝对起始时刻（unix 秒；一次性为具体时刻，每日重复仅取其本地时分做锚点）
    pub start_secs: u64,
    pub duration_mins: u32,
    pub repeat_daily: bool,
}

/// 统一日程条目（calendar-tick / calendar_get_state 载荷字段）
#[derive(Serialize, Clone, Debug)]
pub struct CalEvent {
    pub title: String,
    pub start_secs: u64,
    pub end_secs: u64,
    pub all_day: bool,
    /// "system" | "manual"
    pub source: &'static str,
}

/// 系统日历最近一次查询结果（绝对时刻，跨重算复用）
static SYSTEM_EVENTS: Lazy<Mutex<Vec<CalEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));
/// 手动提醒内存缓存（线程启动时载入，命令改动后经 RECALC_DIRTY 重载）
static MANUAL_CACHE: Lazy<Mutex<Vec<ManualEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));
/// 系统日历是否可用（WinRT 查询成功过；false = 仅手动提醒生效）
static SYSTEM_OK: AtomicBool = AtomicBool::new(false);
/// 上次系统日历查询时刻（unix 秒，0 = 尚未查询）
static LAST_SYS_QUERY: AtomicU64 = AtomicU64::new(0);
/// 手动提醒增删后置位：线程下个循环立即重载缓存 + 重查 + 推送
static RECALC_DIRTY: AtomicBool = AtomicBool::new(false);
/// 系统日历失败日志去重：仅在状态翻转时打 warn，避免每 5 分钟刷屏
static SYS_FAIL_LOGGED: AtomicBool = AtomicBool::new(false);

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 本地时区相对 UTC 的偏移秒数。Windows 用 GetLocalTime（真本地时区，与
/// storage::local_date_string 同源口径）；其余平台 0 = UTC 近似。
fn local_tz_offset_secs(now_utc: u64) -> i64 {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::System::SystemInformation::{GetLocalTime, SYSTEMTIME};
        let mut st: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut st);
        let local_sod = st.wHour as i64 * 3600 + st.wMinute as i64 * 60 + st.wSecond as i64;
        let utc_sod = (now_utc as i64).rem_euclid(86_400);
        // 归一到 ±18h 邻域：本地与 UTC 可能分属不同日期，防止偏移量出现 ±24h 跳变
        let mut off = local_sod - utc_sod;
        if off > 18 * 3600 {
            off -= 86_400;
        } else if off < -18 * 3600 {
            off += 86_400;
        }
        off
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = now_utc;
        0
    }
}

/// 每日重复提醒的下一次发生时刻（unix 秒）：今天未结束取今天，已结束取明天。
/// 每日提醒以"添加时的本地时分"为锚点，跨夏令时按本地墙钟对齐。
fn next_daily_occurrence(start_secs: u64, duration_secs: u64, now: u64, tz_off: i64) -> u64 {
    let sod = (start_secs as i64 + tz_off).rem_euclid(86_400);
    // 本地当天 0 点对应的 unix 秒
    let today_start = (now as i64 + tz_off).div_euclid(86_400) * 86_400 - tz_off;
    let today_occurrence = today_start + sod;
    if today_occurrence + duration_secs as i64 > now as i64 {
        today_occurrence.max(0) as u64
    } else {
        (today_occurrence + 86_400).max(0) as u64
    }
}

fn load_manual(app: &AppHandle) -> Vec<ManualEvent> {
    crate::config_store::ensure_loaded(app);
    crate::config_store::get(MANUAL_KEY)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn save_manual(app: &AppHandle, list: &[ManualEvent]) {
    if let Ok(value) = serde_json::to_value(list) {
        // config_store::set 内部含落盘 + config-changed 广播；增删频度极低，无需节流
        let _ = crate::config_store::set(app, MANUAL_KEY.to_string(), value);
    }
}

/// 查询系统日历未来 24h 日程（只读）。任何失败都以 Err 返回，由调用方统一降级。
#[cfg(target_os = "windows")]
fn query_system_events() -> Result<Vec<CalEvent>, String> {
    // WinRT 调用需要 COM 套间，与本 crate 其他后台线程一致走 MTA（ComGuard RAII 配对释放）
    let _com = crate::win32_utils::ComGuard::new();
    let store = AppointmentManager::RequestStoreAsync(AppointmentStoreAccessType::AllCalendarsReadOnly)
        .map_err(|e| format!("RequestStoreAsync: {e}"))?
        .get()
        .map_err(|e| format!("等待日历存储: {e}"))?;

    let now = unix_now();
    let range_start = DateTime {
        UniversalTime: (now as i64 + UNIX_TO_FILETIME_EPOCH_SECS) * FILETIME_TICKS_PER_SEC,
    };
    let range_len = TimeSpan {
        Duration: HORIZON_SECS as i64 * FILETIME_TICKS_PER_SEC,
    };
    let list = store
        .FindAppointmentsAsync(range_start, range_len)
        .map_err(|e| format!("FindAppointmentsAsync: {e}"))?
        .get()
        .map_err(|e| format!("等待日程列表: {e}"))?;

    let count = list.Size().map_err(|e| format!("读取日程数量: {e}"))?;
    let mut out = Vec::new();
    for i in 0..count {
        let appt = match list.GetAt(i) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let subject = appt.Subject().map(|s| s.to_string()).unwrap_or_default();
        let start = appt.StartTime().map(|t| t.UniversalTime).unwrap_or(0);
        let duration = appt.Duration().map(|d| d.Duration).unwrap_or(0);
        let all_day = appt.AllDay().unwrap_or(false);
        if start == 0 {
            continue;
        }
        let start_secs = (start / FILETIME_TICKS_PER_SEC - UNIX_TO_FILETIME_EPOCH_SECS).max(0) as u64;
        let end_secs = start_secs + (duration / FILETIME_TICKS_PER_SEC).max(0) as u64;
        out.push(CalEvent {
            title: if subject.is_empty() { "(无标题日程)".to_string() } else { subject },
            start_secs,
            end_secs,
            all_day,
            source: "system",
        });
    }
    Ok(out)
}

#[cfg(not(target_os = "windows"))]
fn query_system_events() -> Result<Vec<CalEvent>, String> {
    Err("非 Windows 平台不支持系统日历读取".to_string())
}

/// 重查系统日历并更新缓存与可用性标志；失败仅在状态翻转时打 warn（自动降级，不中断）。
/// WinRT 查询放到短命内层线程并限时等待：`IAsyncOperation::get()` 无超时语义，
/// 系统异常挂起时不能拖死日程线程（手动提醒兜底必须始终工作）。
fn refresh_system_events() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // 接收端超时后丢弃结果（发送失败属预期，静默）
        let _ = tx.send(query_system_events());
    });
    let result = match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(res) => res,
        Err(_) => Err("系统日历查询超时".to_string()),
    };
    match result {
        Ok(list) => {
            *SYSTEM_EVENTS.lock().unwrap_or_else(|e| e.into_inner()) = list;
            if !SYSTEM_OK.swap(true, Ordering::Relaxed) {
                SYS_FAIL_LOGGED.store(false, Ordering::Relaxed);
            }
        }
        Err(e) => {
            SYSTEM_OK.store(false, Ordering::Relaxed);
            if !SYS_FAIL_LOGGED.swap(true, Ordering::Relaxed) {
                eprintln!("[NSD][warn] 系统日历不可用（{}），仅手动提醒生效", e);
            }
        }
    }
    LAST_SYS_QUERY.store(unix_now(), Ordering::Relaxed);
}

/// 重算未来 24h 内的日程：手动（一次性过期清理 / 每日滚动）+ 系统（过滤已结束），
/// 按开始时间升序合并截断。返回的列表同时写回线程缓存供 emit 使用。
fn recompute_upcoming(app: &AppHandle) -> Vec<CalEvent> {
    let now = unix_now();
    let tz_off = local_tz_offset_secs(now);
    let horizon = now + HORIZON_SECS;
    let mut out: Vec<CalEvent> = Vec::new();

    let manual = MANUAL_CACHE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let mut expired: Vec<u64> = Vec::new();
    for ev in &manual {
        let duration_secs = ev.duration_mins.max(1) as u64 * 60;
        if ev.repeat_daily {
            let start = next_daily_occurrence(ev.start_secs, duration_secs, now, tz_off);
            if start < horizon {
                out.push(CalEvent {
                    title: ev.title.clone(),
                    start_secs: start,
                    end_secs: start + duration_secs,
                    all_day: false,
                    source: "manual",
                });
            }
        } else {
            let end = ev.start_secs.saturating_add(duration_secs);
            if end <= now {
                expired.push(ev.id);
            } else if ev.start_secs < horizon {
                out.push(CalEvent {
                    title: ev.title.clone(),
                    start_secs: ev.start_secs,
                    end_secs: end,
                    all_day: false,
                    source: "manual",
                });
            }
        }
    }
    if !expired.is_empty() {
        let remaining: Vec<ManualEvent> = manual
            .into_iter()
            .filter(|e| !expired.contains(&e.id))
            .collect();
        *MANUAL_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = remaining.clone();
        save_manual(app, &remaining);
    }

    let system = SYSTEM_EVENTS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for ev in system {
        if ev.end_secs > now && ev.start_secs < horizon {
            out.push(ev);
        }
    }

    out.sort_by_key(|e| e.start_secs);
    out.truncate(MAX_UPCOMING);
    out
}

/// 当前状态快照载荷（calendar-tick 与 calendar_get_state 同构）
fn state_payload(upcoming: &[CalEvent], manual: &[ManualEvent]) -> serde_json::Value {
    let now = unix_now();
    let remaining = upcoming.first().map(|e| e.start_secs.saturating_sub(now));
    serde_json::json!({
        "system_ok": SYSTEM_OK.load(Ordering::Relaxed),
        "upcoming": upcoming,
        "remaining_secs": remaining,
        "manual": manual,
    })
}

/// 启动日程同步后台线程（lib.rs setup 调用一次）
pub fn start_calendar_thread(app_handle: AppHandle) {
    // 手动提醒缓存初始载入（命令路径经 config_store 即时读写，线程启动仅做预热）
    let initial = load_manual(&app_handle);
    *MANUAL_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = initial;

    crate::thread_mgr::spawn_managed("calendar_tick", move |exit| {
        let mut last_sig = String::new();
        let mut last_emit_secs: u64 = 0;
        loop {
            // 每秒轻量唤醒：手动提醒重算为纯内存运算，系统日历重查每 5 分钟一次
            if exit.sleep_interruptible(Duration::from_secs(1)) {
                return;
            }
            let now = unix_now();
            let dirty = RECALC_DIRTY.swap(false, Ordering::Relaxed);
            let last_query = LAST_SYS_QUERY.load(Ordering::Relaxed);
            if dirty || last_query == 0 || now.saturating_sub(last_query) >= SYS_QUERY_INTERVAL_SECS {
                refresh_system_events();
            }
            if dirty {
                *MANUAL_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = load_manual(&app_handle);
            }

            let upcoming = recompute_upcoming(&app_handle);
            // 列表内容签名：变化即推送（含 进入/离开 24h 窗口、手动增删、过期清理）
            let sig = serde_json::to_string(&upcoming).unwrap_or_default();
            let changed = sig != last_sig;
            let due_emit = !upcoming.is_empty() && now.saturating_sub(last_emit_secs) >= EMIT_INTERVAL_SECS;
            if changed || due_emit {
                last_sig = sig;
                last_emit_secs = now;
                let manual = MANUAL_CACHE.lock().unwrap_or_else(|e| e.into_inner()).clone();
                crate::win32_utils::log_err(
                    app_handle.emit("calendar-tick", state_payload(&upcoming, &manual)),
                    "emit calendar-tick",
                );
            }
        }
    });
}

/// 添加手动提醒（一次性 / 每日重复），返回条目 id
#[tauri::command]
pub fn calendar_add_manual_event(
    app: AppHandle,
    title: String,
    start_secs: u64,
    duration_mins: u32,
    repeat_daily: bool,
) -> u64 {
    let mut list = load_manual(&app);
    // id 取现有最大值 +1：跨重启天然单调，无需额外持久化计数器
    let id = list.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    list.push(ManualEvent {
        id,
        title: title.trim().to_string(),
        start_secs,
        duration_mins: duration_mins.max(1),
        repeat_daily,
    });
    save_manual(&app, &list);
    RECALC_DIRTY.store(true, Ordering::Relaxed);
    id
}

/// 删除手动提醒
#[tauri::command]
pub fn calendar_remove_manual_event(app: AppHandle, id: u64) {
    let mut list = load_manual(&app);
    let before = list.len();
    list.retain(|e| e.id != id);
    if list.len() != before {
        save_manual(&app, &list);
        RECALC_DIRTY.store(true, Ordering::Relaxed);
    }
}

/// 当前状态快照：system_ok + upcoming + manual（窗口重建 / 首查恢复用）
#[tauri::command]
pub fn calendar_get_state(app: AppHandle) -> serde_json::Value {
    // 直接以 config.json 为准刷新缓存，避免查询早于线程的 dirty 重载时读到陈旧列表
    let manual = load_manual(&app);
    *MANUAL_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = manual.clone();
    let upcoming = recompute_upcoming(&app);
    state_payload(&upcoming, &manual)
}
