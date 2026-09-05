//! 番茄钟专注统计：按天累计已完成专注次数 / 专注秒数，落盘 `app_data_dir()/pomodoro_stats.json`。
//! 由 pomodoro.rs 在每次专注阶段完成时 record_focus()（完成事件分钟级稀疏，完成即落盘无需节流）；
//! 前端经 get_pomodoro_stats 读快照，或监听 pomodoro-stats-changed 事件增量刷新。
//! 总计 = 全部按天条目求和，不单独维护 total 字段（单一数据源不漂移），跨天自然不清零。

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::storage::{app_data_dir, atomic_write, local_date_string, read_json};

const FILE_NAME: &str = "pomodoro_stats.json";

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct DayFocus {
    pub count: u32,
    pub secs: u64,
}

static STATS: Lazy<Mutex<HashMap<String, DayFocus>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static INIT_LOCK: Mutex<()> = Mutex::new(());

fn file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(FILE_NAME))
}

/// 启动时从磁盘载入历史统计（lib.rs setup 调用一次）
pub fn init(app: &tauri::AppHandle) {
    let _guard = INIT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if INITIALIZED.load(Ordering::SeqCst) {
        return;
    }
    let loaded: Option<HashMap<String, DayFocus>> =
        file_path(app).ok().and_then(|p| read_json(&p));
    if let Some(map) = loaded {
        *STATS.lock().unwrap_or_else(|e| e.into_inner()) = map;
    }
    INITIALIZED.store(true, Ordering::SeqCst);
}

/// 确保统计已从磁盘载入。若前端查询早于 setup 初始化，也能立即读到历史数据。
pub fn ensure_loaded(app: &tauri::AppHandle) {
    if !INITIALIZED.load(Ordering::SeqCst) {
        init(app);
    }
}

/// 专注阶段完成：当日 +1 次、累计专注秒数，随后立即落盘
pub fn record_focus(app: &tauri::AppHandle, focus_secs: u32) {
    {
        let today = local_date_string();
        let mut map = STATS.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(today).or_default();
        entry.count = entry.count.saturating_add(1);
        entry.secs = entry.secs.saturating_add(focus_secs as u64);
    }
    crate::win32_utils::log_err(persist(app), "persist pomodoro_stats.json");
}

/// 快照：(当日日期键, 当日, 总计)。总计按"当前统计周期内所有按天条目"求和。
pub fn snapshot() -> (String, DayFocus, DayFocus) {
    let today = local_date_string();
    let map = STATS.lock().unwrap_or_else(|e| e.into_inner());
    let day = map.get(&today).copied().unwrap_or_default();
    let total = map.values().fold(DayFocus::default(), |acc, d| DayFocus {
        count: acc.count.saturating_add(d.count),
        secs: acc.secs.saturating_add(d.secs),
    });
    (today, day, total)
}

/// 立即落盘（原子写）
pub fn persist(app: &tauri::AppHandle) -> Result<(), String> {
    ensure_loaded(app);
    let path = file_path(app)?;
    let data = {
        let map = STATS.lock().unwrap_or_else(|e| e.into_inner());
        serde_json::to_vec_pretty(&*map).map_err(|e| format!("序列化番茄统计失败: {}", e))?
    };
    atomic_write(&path, &data)
}
