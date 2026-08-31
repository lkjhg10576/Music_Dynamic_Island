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
static CD_PLAYING_SOUND: AtomicBool = AtomicBool::new(false);

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
            let playing_sound = CD_PLAYING_SOUND.load(Ordering::Relaxed);

            if !active && !playing_sound {
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

            // 如果正在播放提示音，等待播放完成
            if playing_sound {
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
                // 倒计时结束 → 播放 Windows 原生感叹号音效
                CD_PLAYING_SOUND.store(true, Ordering::Relaxed);
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
                // 3秒后重置播放状态，允许用户关闭
                let app_handle_clone = app_handle.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(3));
                    CD_PLAYING_SOUND.store(false, Ordering::Relaxed);
                    let _ = app_handle_clone.emit("countdown-tick", serde_json::json!({
                        "active": false,
                        "phase": "idle",
                    }));
                });
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
    CD_PLAYING_SOUND.store(false, Ordering::Relaxed);
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
    CD_PLAYING_SOUND.store(false, Ordering::Relaxed);
}

#[tauri::command]
pub fn get_countdown_state() -> serde_json::Value {
    let active = CD_ACTIVE.load(Ordering::Relaxed);
    let playing_sound = CD_PLAYING_SOUND.load(Ordering::Relaxed);
    if !active && !playing_sound {
        return serde_json::json!({
            "active": false,
            "phase": "idle",
        });
    }
    let phase = if playing_sound {
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
