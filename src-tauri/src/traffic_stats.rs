//! 流量统计：按天累计上传/下载字节量，落盘 `app_data_dir()/traffic_stats.json`。
//! 由 lib.rs 硬件监控线程（1s 采样）驱动累计，主窗口关闭 / 省内存销毁 / 静默自启下统计不断档；
//! 前端只读快照（get_traffic_stats），不再自行差值累计。

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::storage::{app_data_dir, atomic_write, read_json};

const FILE_NAME: &str = "traffic_stats.json";
/// 落盘节流间隔：异常退出最多丢 30s 数据
const PERSIST_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct DayTraffic {
    pub up: u64,
    pub down: u64,
}

static TRAFFIC: Lazy<Mutex<HashMap<String, DayTraffic>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static LAST_PERSIST: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));

/// 本地日期（YYYY-MM-DD）。仅 Windows 下为真本地时区，其余平台用 UTC 近似。
fn local_date_string() -> String {
    #[cfg(target_os = "windows")]
    unsafe {
        // SYSTEMTIME 实际定义在 minwinbase，sysinfoapi 只是私有重导出（直接引会 E0603）
        use winapi::um::minwinbase::SYSTEMTIME;
        use winapi::um::sysinfoapi::GetLocalTime;
        let mut st: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut st);
        format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        civil_from_days((secs / 86400) as i64)
    }
}

/// 天数（自 1970-01-01）转 YYYY-MM-DD（Howard Hinnant civil_from_days 算法）
#[cfg(not(target_os = "windows"))]
fn civil_from_days(days: i64) -> String {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(FILE_NAME))
}

/// 启动时从磁盘载入历史统计（硬件监控线程开头调用一次）
pub fn init(app: &tauri::AppHandle) {
    let loaded: Option<HashMap<String, DayTraffic>> =
        file_path(app).ok().and_then(|p| read_json(&p));
    if let Some(map) = loaded {
        *TRAFFIC.lock().unwrap() = map;
    }
    *LAST_PERSIST.lock().unwrap() = Some(Instant::now());
}

/// 硬件监控线程每秒调用：把 1s 内的上/下行字节数累计到当日
pub fn accumulate(up_bytes: u64, down_bytes: u64) {
    if up_bytes == 0 && down_bytes == 0 {
        return;
    }
    let today = local_date_string();
    let mut map = TRAFFIC.lock().unwrap();
    let entry = map.entry(today).or_default();
    entry.up += up_bytes;
    entry.down += down_bytes;
}

/// 落盘节流：距上次落盘超过 PERSIST_INTERVAL 才真正写盘
pub fn maybe_persist(app: &tauri::AppHandle) {
    let due = {
        let mut last = LAST_PERSIST.lock().unwrap();
        match *last {
            Some(t) if t.elapsed() < PERSIST_INTERVAL => false,
            _ => {
                *last = Some(Instant::now());
                true
            }
        }
    };
    if due {
        let _ = persist(app);
    }
}

/// 立即落盘（原子写）
pub fn persist(app: &tauri::AppHandle) -> Result<(), String> {
    let path = file_path(app)?;
    let data = {
        let map = TRAFFIC.lock().unwrap();
        serde_json::to_vec_pretty(&*map).map_err(|e| format!("序列化流量统计失败: {}", e))?
    };
    atomic_write(&path, &data)
}

/// 前端只读快照
pub fn snapshot() -> HashMap<String, DayTraffic> {
    TRAFFIC.lock().unwrap().clone()
}

/// 迁移：合并前端 localStorage 的历史数据。按天取 max（后端已累计与旧数据取大者，避免双计），合并后立即落盘。
pub fn merge_legacy(
    app: &tauri::AppHandle,
    legacy: HashMap<String, DayTraffic>,
) -> Result<(), String> {
    {
        let mut map = TRAFFIC.lock().unwrap();
        for (day, traffic) in legacy {
            let entry = map.entry(day).or_default();
            entry.up = entry.up.max(traffic.up);
            entry.down = entry.down.max(traffic.down);
        }
    }
    persist(app)
}
