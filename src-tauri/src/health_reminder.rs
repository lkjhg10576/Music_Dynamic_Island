use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::thread;
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

/// 在 Windows 上播放系统"感叹号"音效
fn play_exclamation_sound() {
    #[cfg(target_os = "windows")]
    unsafe {
        // MB_ICONEXCLAMATION (0x30) 播放系统感叹号音效
        winapi::um::winuser::MessageBeep(0x30);
    }
}

/// 处理单个提醒的 tick 逻辑，返回是否需要播放音效
fn process_reminder_tick(
    enabled: &AtomicBool,
    remaining: &AtomicI32,
    alert_tick: &AtomicU32,
    play_sound: &mut dyn FnMut(),
) -> bool {
    if !enabled.load(Ordering::Relaxed) {
        return false;
    }

    let rem = remaining.load(Ordering::Relaxed);
    if rem > 0 {
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
        // 提醒中，每 5 秒播放一次音效
        let tick = alert_tick.fetch_add(1, Ordering::Relaxed) + 1;
        if tick % 5 == 0 {
            play_sound();
            return true;
        }
    }
    false
}

/// 启动健康提醒后台线程（每秒 tick）
pub fn start_health_reminder_thread(app_handle: AppHandle) {
    thread::spawn(move || {
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
                thread::sleep(Duration::from_millis(5000));
                continue;
            }
            was_inactive = false;

            thread::sleep(Duration::from_millis(1000));

            // 处理久坐提醒
            process_reminder_tick(
                &SITTING_ENABLED,
                &SITTING_REMAINING_SECS,
                &SITTING_ALERT_TICK,
                &mut || play_exclamation_sound(),
            );

            // 处理喝水提醒
            process_reminder_tick(
                &WATER_ENABLED,
                &WATER_REMAINING_SECS,
                &WATER_ALERT_TICK,
                &mut || play_exclamation_sound(),
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
// Tauri 命令
// ──────────────────────────────────────────────

#[tauri::command]
pub fn start_sitting_reminder(interval_secs: u32) {
    SITTING_INTERVAL_SECS.store(interval_secs, Ordering::Relaxed);
    // Clamp to i32::MAX to prevent truncation overflow
    SITTING_REMAINING_SECS.store(interval_secs.min(i32::MAX as u32) as i32, Ordering::Relaxed);
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
        let new_remaining = remaining.saturating_add(interval.min(i32::MAX as u32) as i32);
        SITTING_REMAINING_SECS.store(new_remaining, Ordering::Relaxed);
        SITTING_CAN_SKIP.store(false, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn start_water_reminder(interval_secs: u32) {
    WATER_INTERVAL_SECS.store(interval_secs, Ordering::Relaxed);
    WATER_REMAINING_SECS.store(interval_secs.min(i32::MAX as u32) as i32, Ordering::Relaxed);
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
    WATER_REMAINING_SECS.store(interval.min(i32::MAX as u32) as i32, Ordering::Relaxed);
    WATER_ALERT_TICK.store(0, Ordering::Relaxed);
    WATER_CAN_SKIP.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn skip_water_reminder() {
    let remaining = WATER_REMAINING_SECS.load(Ordering::Relaxed);
    if remaining > 0 && WATER_CAN_SKIP.load(Ordering::Relaxed) {
        let interval = WATER_INTERVAL_SECS.load(Ordering::Relaxed);
        let new_remaining = remaining.saturating_add(interval.min(i32::MAX as u32) as i32);
        WATER_REMAINING_SECS.store(new_remaining, Ordering::Relaxed);
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
