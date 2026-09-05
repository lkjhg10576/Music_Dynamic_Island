use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// ──────────────────────────────────────────────
// 番茄钟原子状态
// ──────────────────────────────────────────────
static POMO_ACTIVE: AtomicBool = AtomicBool::new(false);
static POMO_PAUSED: AtomicBool = AtomicBool::new(false);
static POMO_REMAINING_SECS: AtomicU32 = AtomicU32::new(0);
static POMO_FOCUS_SECS: AtomicU32 = AtomicU32::new(1500);  // 默认 25 min
static POMO_BREAK_SECS: AtomicU32 = AtomicU32::new(300);   // 默认 5 min
static POMO_REMAINING_CYCLES: AtomicU32 = AtomicU32::new(0);
static POMO_TOTAL_CYCLES: AtomicU32 = AtomicU32::new(0);
static POMO_PHASE: AtomicU32 = AtomicU32::new(0);           // 0=focus, 1=break
static POMO_PHASE_TOTAL_SECS: AtomicU32 = AtomicU32::new(0);

/// 启动后台番茄钟线程（每秒 tick，空闲时降低唤醒频率以节省 CPU）
pub fn start_pomodoro_thread(app_handle: AppHandle) {
    crate::thread_mgr::spawn_managed("pomodoro_tick", move |exit| {
        let mut was_inactive = false; // 追踪空闲状态，避免重复 emit
        loop {
            let active = POMO_ACTIVE.load(Ordering::Relaxed);
            if !active {
                // 空闲时仅在状态切换时发送一次 idle 事件，然后延长休眠
                if !was_inactive {
                    let _ = app_handle.emit("pomodoro-tick", serde_json::json!({
                        "active": false,
                        "phase": "idle",
                    }));
                    was_inactive = true;
                }
                // 空闲时延长休眠到 5 秒，大幅降低线程唤醒频率（可中断）
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

            let paused = POMO_PAUSED.load(Ordering::Relaxed);
            if paused {
                // 暂停时仍发送 tick 保持显示
                let _ = app_handle.emit("pomodoro-tick", serde_json::json!({
                    "active": true,
                    "paused": true,
                    "remaining_secs": POMO_REMAINING_SECS.load(Ordering::Relaxed),
                    "phase": if POMO_PHASE.load(Ordering::Relaxed) == 0 { "focus" } else { "break" },
                    "remaining_cycles": POMO_REMAINING_CYCLES.load(Ordering::Relaxed),
                    "total_secs": POMO_PHASE_TOTAL_SECS.load(Ordering::Relaxed),
                }));
                continue;
            }

            let remaining = POMO_REMAINING_SECS.load(Ordering::Relaxed);
            if remaining <= 0 {
                // ── 阶段切换 ──
                let phase = POMO_PHASE.load(Ordering::Relaxed);
                if phase == 0 {
                    // 专注结束 → 切换到休息
                    POMO_PHASE.store(1, Ordering::Relaxed);
                    let break_secs = POMO_BREAK_SECS.load(Ordering::Relaxed);
                    POMO_REMAINING_SECS.store(break_secs, Ordering::Relaxed);
                    POMO_PHASE_TOTAL_SECS.store(break_secs, Ordering::Relaxed);

                    let remaining_cycles = POMO_REMAINING_CYCLES.load(Ordering::Relaxed);
                    let _ = app_handle.emit("pomodoro-tick", serde_json::json!({
                        "active": true,
                        "paused": false,
                        "remaining_secs": break_secs,
                        "phase": "break",
                        "remaining_cycles": remaining_cycles,
                        "total_secs": break_secs,
                    }));
                    // REMAINING_CYCLES 在休息结束时才递减，所以要 +1 才是“刚完成的第几次专注”
                    let completed_focus = POMO_TOTAL_CYCLES.load(Ordering::Relaxed)
                        .saturating_sub(remaining_cycles) + 1;
                    let message = if crate::health_reminder::is_water_enabled() {
                        "专注结束，休息一下吧！记得喝口水~"
                    } else {
                        "专注结束，休息一下吧！"
                    };
                    let _ = app_handle.emit("pomodoro-phase-change", serde_json::json!({
                        "phase": "break",
                        "message": message,
                    }));
                    if crate::health_reminder::is_sitting_enabled() && completed_focus % 2 == 0 {
                        crate::health_reminder::trigger_sitting_alert();
                    }
                } else {
                    // 休息结束 → 下一轮或完成
                    let remaining_cycles = POMO_REMAINING_CYCLES.load(Ordering::Relaxed);
                    if remaining_cycles <= 1 {
                        // 所有轮次完成
                        POMO_ACTIVE.store(false, Ordering::Relaxed);
                        POMO_PAUSED.store(false, Ordering::Relaxed);
                        // 退出接管：健康提醒计时器恢复完整间隔
                        crate::health_reminder::reset_timers();
                        let _ = app_handle.emit("pomodoro-complete", serde_json::json!({
                            "message": "所有番茄钟已完成！",
                        }));
                        // 发送 idle 事件
                        let _ = app_handle.emit("pomodoro-tick", serde_json::json!({
                            "active": false,
                            "phase": "idle",
                        }));
                    } else {
                        POMO_REMAINING_CYCLES.store(remaining_cycles - 1, Ordering::Relaxed);
                        POMO_PHASE.store(0, Ordering::Relaxed);
                        let focus_secs = POMO_FOCUS_SECS.load(Ordering::Relaxed);
                        POMO_REMAINING_SECS.store(focus_secs, Ordering::Relaxed);
                        POMO_PHASE_TOTAL_SECS.store(focus_secs, Ordering::Relaxed);

                        let _ = app_handle.emit("pomodoro-tick", serde_json::json!({
                            "active": true,
                            "paused": false,
                            "remaining_secs": focus_secs,
                            "phase": "focus",
                            "remaining_cycles": POMO_REMAINING_CYCLES.load(Ordering::Relaxed),
                            "total_secs": focus_secs,
                        }));
                        let _ = app_handle.emit("pomodoro-phase-change", serde_json::json!({
                            "phase": "focus",
                            "message": "休息结束，继续专注！",
                        }));
                    }
                }
            } else {
                // ── 正常倒计时 ──
                POMO_REMAINING_SECS.store(remaining - 1, Ordering::Relaxed);
                let phase = POMO_PHASE.load(Ordering::Relaxed);
                let _ = app_handle.emit("pomodoro-tick", serde_json::json!({
                    "active": true,
                    "paused": false,
                    "remaining_secs": remaining - 1,
                    "phase": if phase == 0 { "focus" } else { "break" },
                    "remaining_cycles": POMO_REMAINING_CYCLES.load(Ordering::Relaxed),
                    "total_secs": POMO_PHASE_TOTAL_SECS.load(Ordering::Relaxed),
                }));
            }
        }
    });
}

#[tauri::command]
pub fn start_pomodoro(focus_secs: u32, break_secs: u32, cycles: u32) {
    POMO_FOCUS_SECS.store(focus_secs, Ordering::Relaxed);
    POMO_BREAK_SECS.store(break_secs, Ordering::Relaxed);
    POMO_REMAINING_CYCLES.store(cycles, Ordering::Relaxed);
    POMO_TOTAL_CYCLES.store(cycles, Ordering::Relaxed);
    POMO_PHASE.store(0, Ordering::Relaxed); // 从专注开始
    POMO_REMAINING_SECS.store(focus_secs, Ordering::Relaxed);
    POMO_PHASE_TOTAL_SECS.store(focus_secs, Ordering::Relaxed);
    POMO_ACTIVE.store(true, Ordering::Relaxed);
    POMO_PAUSED.store(false, Ordering::Relaxed);
}

#[tauri::command]
pub fn pause_pomodoro() {
    POMO_PAUSED.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn resume_pomodoro() {
    POMO_PAUSED.store(false, Ordering::Relaxed);
}

#[tauri::command]
pub fn stop_pomodoro() {
    POMO_ACTIVE.store(false, Ordering::Relaxed);
    POMO_PAUSED.store(false, Ordering::Relaxed);
    // 手动停止同样退出接管：健康提醒计时器恢复完整间隔
    crate::health_reminder::reset_timers();
}

pub fn is_active() -> bool {
    POMO_ACTIVE.load(Ordering::Relaxed)
}

pub fn is_paused() -> bool {
    POMO_PAUSED.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn get_pomodoro_state() -> serde_json::Value {
    let active = POMO_ACTIVE.load(Ordering::Relaxed);
    if !active {
        return serde_json::json!({
            "active": false,
            "phase": "idle",
        });
    }
    serde_json::json!({
        "active": true,
        "paused": POMO_PAUSED.load(Ordering::Relaxed),
        "remaining_secs": POMO_REMAINING_SECS.load(Ordering::Relaxed),
        "phase": if POMO_PHASE.load(Ordering::Relaxed) == 0 { "focus" } else { "break" },
        "remaining_cycles": POMO_REMAINING_CYCLES.load(Ordering::Relaxed),
        "total_cycles": POMO_TOTAL_CYCLES.load(Ordering::Relaxed),
        "focus_secs": POMO_FOCUS_SECS.load(Ordering::Relaxed),
        "break_secs": POMO_BREAK_SECS.load(Ordering::Relaxed),
        "total_secs": POMO_PHASE_TOTAL_SECS.load(Ordering::Relaxed),
    })
}