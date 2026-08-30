//! 设置单一数据源：所有前端设置项持久化到 `app_data_dir()/config.json`。
//! - 内存 HashMap 为读写热点，落盘由专用线程 500ms 节流（atomic_write）
//! - 每次写入向所有窗口广播 `config-changed`，双窗口不再依赖手动 emit 同步设置值
//! - 前端只通过统一存取层（src/utils/settings.ts）访问，不再直读 localStorage

use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::Emitter;

use crate::storage::{app_data_dir, atomic_write, read_json};

const FILE_NAME: &str = "config.json";

static CONFIG: Lazy<Mutex<HashMap<String, Value>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static DIRTY: AtomicBool = AtomicBool::new(false);

fn file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(FILE_NAME))
}

/// 启动时载入 config.json 并启动 500ms 节流落盘线程（setup 阶段调用一次）
pub fn init(app: &tauri::AppHandle) {
    if let Some(map) = file_path(app)
        .ok()
        .and_then(|p| read_json::<HashMap<String, Value>>(&p))
    {
        *CONFIG.lock().unwrap() = map;
    }
    let handle = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if DIRTY.swap(false, Ordering::Relaxed) {
            let _ = persist(&handle);
        }
    });
}

pub fn persist(app: &tauri::AppHandle) -> Result<(), String> {
    let path = file_path(app)?;
    let data = {
        let map = CONFIG.lock().unwrap();
        serde_json::to_vec_pretty(&*map).map_err(|e| format!("序列化配置失败: {}", e))?
    };
    atomic_write(&path, &data)
}

pub fn get(key: &str) -> Option<Value> {
    CONFIG.lock().unwrap().get(key).cloned()
}

pub fn get_all() -> HashMap<String, Value> {
    CONFIG.lock().unwrap().clone()
}

pub fn set(app: &tauri::AppHandle, key: String, value: Value) -> Result<(), String> {
    {
        let mut map = CONFIG.lock().unwrap();
        // 值未变化则跳过，避免重复落盘与广播
        if map.get(&key) == Some(&value) {
            return Ok(());
        }
        map.insert(key.clone(), value.clone());
    }
    DIRTY.store(true, Ordering::Relaxed);
    let _ = app.emit(
        "config-changed",
        serde_json::json!({ "key": key, "value": value }),
    );
    Ok(())
}

pub fn remove(app: &tauri::AppHandle, key: String) -> Result<(), String> {
    let removed = CONFIG.lock().unwrap().remove(&key).is_some();
    if removed {
        DIRTY.store(true, Ordering::Relaxed);
        let _ = app.emit(
            "config-changed",
            serde_json::json!({ "key": key, "value": Value::Null }),
        );
    }
    Ok(())
}

/// 一次性迁移：接收窗口 localStorage 的旧键值。后端已有值不覆盖（先启动的窗口迁移生效，
/// 双窗口 localStorage 本就靠 emit 保持一致，先到先得即可），迁移后立即落盘。
pub fn merge_legacy(app: &tauri::AppHandle, legacy: HashMap<String, Value>) -> Result<(), String> {
    let mut changed = false;
    {
        let mut map = CONFIG.lock().unwrap();
        for (k, v) in legacy {
            if !map.contains_key(&k) {
                map.insert(k, v);
                changed = true;
            }
        }
    }
    if changed {
        DIRTY.store(true, Ordering::Relaxed);
    }
    persist(app)
}
