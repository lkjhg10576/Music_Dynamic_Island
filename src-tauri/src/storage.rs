//! 落盘基建：应用数据目录管理 + 原子写 + JSON 读取。
//! 供流量统计（traffic_stats）、设置单一数据源（config）、歌词缓存（lyrics）共用。

use serde::de::DeserializeOwned;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
// app.path() 来自 Manager trait，必须显式导入否则 E0599
use tauri::Manager;

/// 获取应用数据目录（不存在则创建）
pub fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("创建应用数据目录失败: {}", e))?;
    }
    Ok(dir)
}

/// 原子写：先写同目录临时文件，成功后 rename 覆盖目标，避免写一半崩溃损坏数据
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("创建临时文件失败: {}", e))?;
        f.write_all(data)
            .map_err(|e| format!("写入临时文件失败: {}", e))?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, path).map_err(|e| format!("替换文件失败: {}", e))?;
    Ok(())
}

/// 读取 JSON 文件；文件不存在或解析失败一律返回 None（数据 0 信任：损坏即弃用）
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let data = fs::read(path).ok()?;
    serde_json::from_slice(&data).ok()
}
