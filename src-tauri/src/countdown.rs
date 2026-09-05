use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// ──────────────────────────────────────────────
// 倒计时原子状态
// ──────────────────────────────────────────────
static CD_ACTIVE: AtomicBool = AtomicBool::new(false);
static CD_PAUSED: AtomicBool = AtomicBool::new(false);
static CD_REMAINING_SECS: AtomicU32 = AtomicU32::new(0);
static CD_TOTAL_SECS: AtomicU32 = AtomicU32::new(0);
/// 结束响铃态：归零后置位，每 ALARM_INTERVAL_SECS 重复响一声，直到用户 stop_countdown_alarm 消音
static CD_ALARM_ACTIVE: AtomicBool = AtomicBool::new(false);
/// 响铃态已持续的秒数（驱动重复响铃间隔）
static CD_ALARM_TICK: AtomicU32 = AtomicU32::new(0);

/// 结束响铃的重复间隔（秒）
const ALARM_INTERVAL_SECS: u32 = 5;

/// 播放 Windows 感叹号音效
fn play_exclamation_sound() {
    thread::spawn(move || {
        // MB_ICONEXCLAMATION = 0x30，播放系统感叹号音效（收口到 win32_utils）
        crate::win32_utils::message_beep(0x30);
    });
}

/// 启动后台倒计时线程（每秒 tick，空闲时降低唤醒频率以节省 CPU）
pub fn start_countdown_thread(app_handle: AppHandle) {
    crate::thread_mgr::spawn_managed("countdown_tick", move |exit| {
        let mut was_idle = false; // 追踪空闲状态，避免重复 emit
        loop {
            let active = CD_ACTIVE.load(Ordering::Relaxed);
            let alarm = CD_ALARM_ACTIVE.load(Ordering::Relaxed);

            if !active && !alarm {
                // 空闲时仅在状态切换时发送一次 idle 事件，然后延长休眠
                if !was_idle {
                    let _ = app_handle.emit("countdown-tick", serde_json::json!({
                        "active": false,
                        "phase": "idle",
                    }));
                    was_idle = true;
                }
                // 空闲时延长休眠到 5 秒，大幅降低线程唤醒频率（可中断）
                if exit.sleep_interruptible(Duration::from_millis(5000)) {
                    return;
                }
                continue;
            }
            was_idle = false;
            // 可中断 1s 休眠：收到退出信号立即结束
            if exit.sleep_interruptible(Duration::from_millis(1000)) {
                return;
            }

            // 结束响铃态：不依赖前端窗口，由本线程每 5 秒重复响一声，
            // 直到用户经 stop_countdown_alarm / stop_countdown 消音
            // （tick 从 1 计数，%5==0 → 归零瞬间第一声后，第 5/10/… 秒重复）
            if alarm {
                let tick = CD_ALARM_TICK.fetch_add(1, Ordering::Relaxed) + 1;
                if tick % ALARM_INTERVAL_SECS == 0 {
                    play_exclamation_sound();
                }
                let _ = app_handle.emit("countdown-tick", serde_json::json!({
                    "active": true,
                    "paused": true,
                    "remaining_secs": 0,
                    "phase": "finished",
                    "total_secs": CD_TOTAL_SECS.load(Ordering::Relaxed),
                }));
                continue;
            }

            let paused = CD_PAUSED.load(Ordering::Relaxed);
            if paused {
                // 暂停时仍发送 tick 保持显示
                let _ = app_handle.emit("countdown-tick", serde_json::json!({
                    "active": true,
                    "paused": true,
                    "remaining_secs": CD_REMAINING_SECS.load(Ordering::Relaxed),
                    "phase": "countdown",
                    "total_secs": CD_TOTAL_SECS.load(Ordering::Relaxed),
                }));
                continue;
            }

            let remaining = CD_REMAINING_SECS.load(Ordering::Relaxed);
            if remaining <= 0 {
                // 倒计时结束 → 进入结束响铃态：立即响一声，之后由响铃分支每 5 秒重复
                CD_ALARM_ACTIVE.store(true, Ordering::Relaxed);
                CD_ALARM_TICK.store(0, Ordering::Relaxed);
                CD_ACTIVE.store(false, Ordering::Relaxed);
                play_exclamation_sound();
                let _ = app_handle.emit("countdown-complete", serde_json::json!({
                    "message": "倒计时结束",
                }));
                let _ = app_handle.emit("countdown-tick", serde_json::json!({
                    "active": true,
                    "paused": true,
                    "remaining_secs": 0,
                    "phase": "finished",
                    "total_secs": CD_TOTAL_SECS.load(Ordering::Relaxed),
                }));
                // 响铃持续到用户消音，不再自动复位；idle 事件由 stop_countdown_alarm 后的空闲分支发出
            } else {
                // 正常倒计时
                CD_REMAINING_SECS.store(remaining - 1, Ordering::Relaxed);
                let _ = app_handle.emit("countdown-tick", serde_json::json!({
                    "active": true,
                    "paused": false,
                    "remaining_secs": remaining - 1,
                    "phase": "countdown",
                    "total_secs": CD_TOTAL_SECS.load(Ordering::Relaxed),
                }));
            }
        }
    });
}

#[tauri::command]
pub fn start_countdown(total_secs: u32) {
    CD_TOTAL_SECS.store(total_secs, Ordering::Relaxed);
    CD_REMAINING_SECS.store(total_secs, Ordering::Relaxed);
    CD_ACTIVE.store(true, Ordering::Relaxed);
    CD_PAUSED.store(false, Ordering::Relaxed);
    CD_ALARM_ACTIVE.store(false, Ordering::Relaxed);
    CD_ALARM_TICK.store(0, Ordering::Relaxed);
}

#[tauri::command]
pub fn pause_countdown() {
    CD_PAUSED.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn resume_countdown() {
    CD_PAUSED.store(false, Ordering::Relaxed);
}

#[tauri::command]
pub fn stop_countdown() {
    CD_ACTIVE.store(false, Ordering::Relaxed);
    CD_PAUSED.store(false, Ordering::Relaxed);
    CD_ALARM_ACTIVE.store(false, Ordering::Relaxed);
    CD_ALARM_TICK.store(0, Ordering::Relaxed);
}

/// 消音：仅终止结束响铃（倒计时本就已完成），idle 事件由线程的空闲分支发出
#[tauri::command]
pub fn stop_countdown_alarm() {
    CD_ALARM_ACTIVE.store(false, Ordering::Relaxed);
    CD_ALARM_TICK.store(0, Ordering::Relaxed);
}

#[tauri::command]
pub fn get_countdown_state() -> serde_json::Value {
    let active = CD_ACTIVE.load(Ordering::Relaxed);
    let alarm = CD_ALARM_ACTIVE.load(Ordering::Relaxed);
    if !active && !alarm {
        return serde_json::json!({
            "active": false,
            "phase": "idle",
        });
    }
    let phase = if alarm {
        "finished"
    } else {
        "countdown"
    };
    serde_json::json!({
        "active": true,
        "paused": CD_PAUSED.load(Ordering::Relaxed),
        "remaining_secs": CD_REMAINING_SECS.load(Ordering::Relaxed),
        "phase": phase,
        "total_secs": CD_TOTAL_SECS.load(Ordering::Relaxed),
    })
}
