use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// ──────────────────────────────────────────────
// 久坐提醒状态
// ──────────────────────────────────────────────
static SITTING_ENABLED: AtomicBool = AtomicBool::new(false);
static SITTING_REMAINING_SECS: AtomicI32 = AtomicI32::new(0); // >0 倒计时, -1 提醒中
static SITTING_INTERVAL_SECS: AtomicU32 = AtomicU32::new(3600); // 默认 1 小时
static SITTING_ALERT_TICK: AtomicU32 = AtomicU32::new(0); // 提醒中辅助计数器
static SITTING_CAN_SKIP: AtomicBool = AtomicBool::new(true); // 是否可以跳过

// ──────────────────────────────────────────────
// 喝水提醒状态
// ──────────────────────────────────────────────
static WATER_ENABLED: AtomicBool = AtomicBool::new(false);
static WATER_REMAINING_SECS: AtomicI32 = AtomicI32::new(0); // >0 倒计时, -1 提醒中
static WATER_INTERVAL_SECS: AtomicU32 = AtomicU32::new(7200); // 默认 2 小时
static WATER_ALERT_TICK: AtomicU32 = AtomicU32::new(0); // 提醒中辅助计数器
static WATER_CAN_SKIP: AtomicBool = AtomicBool::new(true); // 是否可以跳过

/// 提醒中补响第二声的时机（进入 alerting 后第 N 秒），之后保持静默。
/// 视觉提醒继续显示到用户 dismiss，避免提示音长时间循环。久坐/喝水共用。
const ALERT_RESOUND_AT: u32 = 3;

/// 在 Windows 上播放系统"感叹号"音效
fn play_exclamation_sound() {
    #[cfg(target_os = "windows")]
    {
        // MB_ICONEXCLAMATION (0x30) 播放系统感叹号音效（收口到 win32_utils）
        crate::win32_utils::message_beep(0x30);
    }
}

/// 处理单个提醒的 tick 逻辑，返回是否需要播放音效。
/// `countdown_suspended`：番茄钟接管期间倒计时不走（节奏交给番茄钟），
/// 但已进入提醒中（-1）的补响节奏仍按 ALERT_RESOUND_AT 推进。
fn process_reminder_tick(
    enabled: &AtomicBool,
    remaining: &AtomicI32,
    alert_tick: &AtomicU32,
    play_sound: &mut dyn FnMut(),
    countdown_suspended: bool,
) -> bool {
    if !enabled.load(Ordering::Relaxed) {
        return false;
    }

    let rem = remaining.load(Ordering::Relaxed);
    if rem > 0 {
        if countdown_suspended {
            return false;
        }
        // 正常倒计时
        remaining.store(rem - 1, Ordering::Relaxed);
        if rem - 1 == 0 {
            // 倒计时结束，进入提醒状态
            remaining.store(-1, Ordering::Relaxed);
            alert_tick.store(0, Ordering::Relaxed);
            play_sound();
            return true;
        }
    } else if rem == -1 {
        // 提醒中：第 ALERT_RESOUND_AT 秒补响第二声，之后保持静默（视觉提醒挂到用户 dismiss）
        let tick = alert_tick.fetch_add(1, Ordering::Relaxed) + 1;
        if tick == ALERT_RESOUND_AT {
            play_sound();
            return true;
        }
    }
    false
}

/// 启动健康提醒后台线程（每秒 tick）
pub fn start_health_reminder_thread(app_handle: AppHandle) {
    crate::thread_mgr::spawn_managed("health_reminder_tick", move |exit| {
        let mut was_inactive = false;
        loop {
            let sitting_enabled = SITTING_ENABLED.load(Ordering::Relaxed);
            let water_enabled = WATER_ENABLED.load(Ordering::Relaxed);

            if !sitting_enabled && !water_enabled {
                if !was_inactive {
                    let _ = app_handle.emit("health-reminder-tick", serde_json::json!({
                        "sitting": { "enabled": false, "remaining_secs": 0, "alerting": false, "label": "" },
                        "water": { "enabled": false, "remaining_secs": 0, "alerting": false, "label": "" },
                    }));
                    was_inactive = true;
                }
                // 空闲时延长休眠到 5 秒（可中断）
                if exit.sleep_interruptible(Duration::from_millis(5000)) {
                    return;
                }
                continue;
            }
            was_inactive = false;

            // 可中断 1s 休眠：收到退出信号立即结束
            if exit.sleep_interruptible(Duration::from_millis(1000)) {
                return;
            }

            // 番茄钟运行中（未暂停）时接管健康提醒节奏：倒计时冻结；
            // 暂停或结束后恢复计时（POMO_ACTIVE 仍为 true，必须额外查 POMO_PAUSED）。
            // 提醒中（-1）的补响不受接管影响，否则番茄钟触发的久坐提醒只会响一声。
            let pomo_taking_over = crate::pomodoro::is_active() && !crate::pomodoro::is_paused();
            // 处理久坐提醒
            process_reminder_tick(
                &SITTING_ENABLED,
                &SITTING_REMAINING_SECS,
                &SITTING_ALERT_TICK,
                &mut || play_exclamation_sound(),
                pomo_taking_over,
            );

            // 处理喝水提醒
            process_reminder_tick(
                &WATER_ENABLED,
                &WATER_REMAINING_SECS,
                &WATER_ALERT_TICK,
                &mut || play_exclamation_sound(),
                pomo_taking_over,
            );

            let sitting_rem = SITTING_REMAINING_SECS.load(Ordering::Relaxed);
            let water_rem = WATER_REMAINING_SECS.load(Ordering::Relaxed);

            let _ = app_handle.emit("health-reminder-tick", serde_json::json!({
                "sitting": {
                    "enabled": SITTING_ENABLED.load(Ordering::Relaxed),
                    "remaining_secs": if sitting_rem > 0 { sitting_rem } else { 0 },
                    "alerting": sitting_rem == -1,
                    "label": "该起来走走了",
                    "can_skip": SITTING_CAN_SKIP.load(Ordering::Relaxed),
                },
                "water": {
                    "enabled": WATER_ENABLED.load(Ordering::Relaxed),
                    "remaining_secs": if water_rem > 0 { water_rem } else { 0 },
                    "alerting": water_rem == -1,
                    "label": "该喝水了",
                    "can_skip": WATER_CAN_SKIP.load(Ordering::Relaxed),
                },
            }));
        }
    });
}

// ──────────────────────────────────────────────
// 番茄钟联动接口（纯内部调用，不注册 #[tauri::command]）
// ──────────────────────────────────────────────

/// 久坐提醒是否启用（供番茄钟联动查询）
pub fn is_sitting_enabled() -> bool {
    SITTING_ENABLED.load(Ordering::Relaxed)
}

/// 喝水提醒是否启用（供番茄钟联动查询）
pub fn is_water_enabled() -> bool {
    WATER_ENABLED.load(Ordering::Relaxed)
}

/// 立即触发一次久坐提醒（番茄钟每完成 2 次专注时调用）：
/// 置为提醒中状态、清 tick、播一声，第二声由 tick 线程按 ALERT_RESOUND_AT 补响。
pub fn trigger_sitting_alert() {
    if !SITTING_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    SITTING_REMAINING_SECS.store(-1, Ordering::Relaxed);
    SITTING_ALERT_TICK.store(0, Ordering::Relaxed);
    play_exclamation_sound();
}

/// 退出番茄钟接管：两个计时器重置为完整间隔，并清除提醒中状态
pub fn reset_timers() {
    let sitting_interval = SITTING_INTERVAL_SECS.load(Ordering::Relaxed);
    SITTING_REMAINING_SECS.store(sitting_interval as i32, Ordering::Relaxed);
    SITTING_ALERT_TICK.store(0, Ordering::Relaxed);
    SITTING_CAN_SKIP.store(true, Ordering::Relaxed);

    let water_interval = WATER_INTERVAL_SECS.load(Ordering::Relaxed);
    WATER_REMAINING_SECS.store(water_interval as i32, Ordering::Relaxed);
    WATER_ALERT_TICK.store(0, Ordering::Relaxed);
    WATER_CAN_SKIP.store(true, Ordering::Relaxed);
}

// ──────────────────────────────────────────────
// Tauri 命令
// ──────────────────────────────────────────────

#[tauri::command]
pub fn start_sitting_reminder(interval_secs: u32) {
    SITTING_INTERVAL_SECS.store(interval_secs, Ordering::Relaxed);
    SITTING_REMAINING_SECS.store(interval_secs as i32, Ordering::Relaxed);
    SITTING_ALERT_TICK.store(0, Ordering::Relaxed);
    SITTING_ENABLED.store(true, Ordering::Relaxed);
    SITTING_CAN_SKIP.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn stop_sitting_reminder() {
    SITTING_ENABLED.store(false, Ordering::Relaxed);
    SITTING_REMAINING_SECS.store(0, Ordering::Relaxed);
    SITTING_ALERT_TICK.store(0, Ordering::Relaxed);
    SITTING_CAN_SKIP.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn dismiss_sitting_alert() {
    let interval = SITTING_INTERVAL_SECS.load(Ordering::Relaxed);
    SITTING_REMAINING_SECS.store(interval as i32, Ordering::Relaxed);
    SITTING_ALERT_TICK.store(0, Ordering::Relaxed);
    SITTING_CAN_SKIP.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn skip_sitting_reminder() {
    let remaining = SITTING_REMAINING_SECS.load(Ordering::Relaxed);
    if remaining > 0 && SITTING_CAN_SKIP.load(Ordering::Relaxed) {
        let interval = SITTING_INTERVAL_SECS.load(Ordering::Relaxed);
        SITTING_REMAINING_SECS.store(remaining + interval as i32, Ordering::Relaxed);
        SITTING_CAN_SKIP.store(false, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn start_water_reminder(interval_secs: u32) {
    WATER_INTERVAL_SECS.store(interval_secs, Ordering::Relaxed);
    WATER_REMAINING_SECS.store(interval_secs as i32, Ordering::Relaxed);
    WATER_ALERT_TICK.store(0, Ordering::Relaxed);
    WATER_ENABLED.store(true, Ordering::Relaxed);
    WATER_CAN_SKIP.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn stop_water_reminder() {
    WATER_ENABLED.store(false, Ordering::Relaxed);
    WATER_REMAINING_SECS.store(0, Ordering::Relaxed);
    WATER_ALERT_TICK.store(0, Ordering::Relaxed);
    WATER_CAN_SKIP.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn dismiss_water_alert() {
    let interval = WATER_INTERVAL_SECS.load(Ordering::Relaxed);
    WATER_REMAINING_SECS.store(interval as i32, Ordering::Relaxed);
    WATER_ALERT_TICK.store(0, Ordering::Relaxed);
    WATER_CAN_SKIP.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn skip_water_reminder() {
    let remaining = WATER_REMAINING_SECS.load(Ordering::Relaxed);
    if remaining > 0 && WATER_CAN_SKIP.load(Ordering::Relaxed) {
        let interval = WATER_INTERVAL_SECS.load(Ordering::Relaxed);
        WATER_REMAINING_SECS.store(remaining + interval as i32, Ordering::Relaxed);
        WATER_CAN_SKIP.store(false, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn get_health_reminder_state() -> serde_json::Value {
    let sitting_rem = SITTING_REMAINING_SECS.load(Ordering::Relaxed);
    let water_rem = WATER_REMAINING_SECS.load(Ordering::Relaxed);

    serde_json::json!({
        "sitting": {
            "enabled": SITTING_ENABLED.load(Ordering::Relaxed),
            "remaining_secs": if sitting_rem > 0 { sitting_rem } else { 0 },
            "alerting": sitting_rem == -1,
            "interval_secs": SITTING_INTERVAL_SECS.load(Ordering::Relaxed),
            "can_skip": SITTING_CAN_SKIP.load(Ordering::Relaxed),
        },
        "water": {
            "enabled": WATER_ENABLED.load(Ordering::Relaxed),
            "remaining_secs": if water_rem > 0 { water_rem } else { 0 },
            "alerting": water_rem == -1,
            "interval_secs": WATER_INTERVAL_SECS.load(Ordering::Relaxed),
            "can_skip": WATER_CAN_SKIP.load(Ordering::Relaxed),
        },
    })
}
