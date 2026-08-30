//! 歌词缓存：按「规范化歌名 + 规范化歌手 + 时长」唯一 key 落盘歌词。
//! - key 生成收敛为本模块唯一函数（播放匹配 / 导入建条目 / 保存绑定共用），是「保存后必命中」的根基
//! - 存储：`app_data_dir()/lyrics/{hash}.lrc`（一律 UTF-8，编码猜测只发生在导入入口）+ `index.json`
//! - 命中规则：key 精确匹配 → 同曲同歌手 ±2s 时长容差扫描

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::AppHandle;

use crate::storage::{app_data_dir, atomic_write, read_json};

/// 时长容差（毫秒）：SMTC 上报时长与源站时长存在轻微偏差
const DURATION_TOLERANCE_MS: i64 = 2000;

#[derive(Serialize, Deserialize, Clone)]
pub struct LyricEntry {
    /// 规范化 key：normalize(歌名)|normalize(歌手)|时长秒
    pub key: String,
    /// lrc 文件名（hash.lrc）
    pub file: String,
    /// auto = 播放链路自动缓存；user = 管理界面手动绑定/导入
    pub source: String,
    pub song: String,
    pub artist: String,
    pub duration_ms: i64,
    /// 保存时间（unix 秒）
    pub saved_at: u64,
}

static INDEX_CACHE: Lazy<Mutex<Option<Vec<LyricEntry>>>> = Lazy::new(|| Mutex::new(None));

fn lyrics_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app_data_dir(app)?.join("lyrics");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建歌词目录失败: {}", e))?;
    }
    Ok(dir)
}

fn index_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app_data_dir(app)?.join("lyrics").join("index.json"))
}

/// 规范化文本：小写 + 去除所有空白（含全角空格）。唯一 key 生成的一部分，勿在别处重写。
fn normalize_text(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// FNV-1a 64 位哈希：确定性强、跨版本稳定，用于把 key 映射为文件名
fn fnv1a(data: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in data.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// 唯一 key 生成（全项目仅此一处）：normalize(歌名) | normalize(歌手) | 时长秒
pub fn make_key(song: &str, artist: &str, duration_ms: i64) -> String {
    format!(
        "{}|{}|{}",
        normalize_text(song),
        normalize_text(artist),
        if duration_ms > 0 {
            duration_ms / 1000
        } else {
            0
        }
    )
}

fn load_index(app: &AppHandle) -> Vec<LyricEntry> {
    let mut cached = INDEX_CACHE.lock().unwrap();
    if let Some(list) = cached.as_ref() {
        return list.clone();
    }
    let loaded: Option<Vec<LyricEntry>> = index_path(app).ok().and_then(|p| read_json(&p));
    // 数据 0 信任：索引损坏一律视为空（重建），宁丢不坏
    let list = loaded.unwrap_or_default();
    *cached = Some(list.clone());
    list
}

fn persist_index(app: &AppHandle, list: &[LyricEntry]) -> Result<(), String> {
    let path = index_path(app)?;
    let data = serde_json::to_vec_pretty(list).map_err(|e| format!("序列化歌词索引失败: {}", e))?;
    atomic_write(&path, &data)?;
    *INDEX_CACHE.lock().unwrap() = Some(list.to_vec());
    Ok(())
}

fn read_lrc(dir: &std::path::Path, file: &str) -> Option<String> {
    let bytes = std::fs::read(dir.join(file)).ok()?;
    String::from_utf8(bytes).ok()
}

/// 查询本地缓存：先精确 key，再同曲同歌手 ±2s 时长容差
pub fn lookup(song: &str, artist: &str, duration_ms: i64, app: &AppHandle) -> Option<String> {
    let index = load_index(app);
    let norm_song = normalize_text(song);
    let norm_artist = normalize_text(artist);
    if norm_song.is_empty() {
        return None;
    }
    let dir = lyrics_dir(app).ok()?;

    // 1. 精确 key 命中
    let exact_key = make_key(&norm_song, &norm_artist, duration_ms);
    if let Some(entry) = index.iter().find(|e| e.key == exact_key) {
        if let Some(content) = read_lrc(&dir, &entry.file) {
            return Some(content);
        }
    }

    // 2. ±2s 容差：规范化后同曲同歌手，时长差最小者
    if duration_ms > 0 {
        let prefix = format!("{}|{}|", norm_song, norm_artist);
        let hit = index
            .iter()
            .filter(|e| e.key.starts_with(&prefix))
            .filter(|e| (e.duration_ms - duration_ms).abs() <= DURATION_TOLERANCE_MS)
            .min_by_key(|e| (e.duration_ms - duration_ms).abs());
        if let Some(entry) = hit {
            if let Some(content) = read_lrc(&dir, &entry.file) {
                return Some(content);
            }
        }
    }
    None
}

/// 保存歌词：写 lrc 文件（UTF-8）+ 更新索引（同 key 覆盖）。内容为空拒收。
pub fn save(
    app: &AppHandle,
    song: &str,
    artist: &str,
    duration_ms: i64,
    content: &str,
    source: &str,
) -> Result<LyricEntry, String> {
    if content.trim().is_empty() {
        return Err("歌词内容为空，已拒绝保存".into());
    }
    let dir = lyrics_dir(app)?;
    let key = make_key(song, artist, duration_ms);
    let file = format!("{}.lrc", fnv1a(&key));
    // 入库即归一 UTF-8：Rust String 本身即 UTF-8，编码识别只发生在导入入口
    atomic_write(&dir.join(&file), content.as_bytes())?;

    let mut index = load_index(app);
    index.retain(|e| e.key != key);
    let entry = LyricEntry {
        key: key.clone(),
        file,
        source: source.to_string(),
        song: song.to_string(),
        artist: artist.to_string(),
        duration_ms,
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    index.push(entry.clone());
    persist_index(app, &index)?;
    Ok(entry)
}

/// 一级列表：全部缓存条目（管理界面用）
pub fn list(app: &AppHandle) -> Vec<LyricEntry> {
    load_index(app)
}

/// 按 key 读取歌词原文
pub fn get_by_key(app: &AppHandle, key: &str) -> Option<String> {
    let index = load_index(app);
    let entry = index.iter().find(|e| e.key == key)?;
    let dir = lyrics_dir(app).ok()?;
    read_lrc(&dir, &entry.file)
}

/// 删除条目：移除 lrc 文件与索引记录
pub fn delete(app: &AppHandle, key: &str) -> Result<(), String> {
    let mut index = load_index(app);
    if let Some(pos) = index.iter().position(|e| e.key == key) {
        let entry = index.remove(pos);
        if let Ok(dir) = lyrics_dir(app) {
            let _ = std::fs::remove_file(dir.join(&entry.file));
        }
        persist_index(app, &index)?;
    }
    Ok(())
}

// ===== 歌词管理界面（6b）command：key 由本模块唯一函数生成 =====

/// 保存绑定（user 来源）：song/artist/duration 由后端生成唯一 key，保存成功才写索引
#[tauri::command]
pub fn save_lyrics_binding(
    app: AppHandle,
    song: String,
    artist: String,
    duration_ms: i64,
    content: String,
) -> Result<LyricEntry, String> {
    save(&app, &song, &artist, duration_ms, &content, "user")
}

/// 一级列表：全部缓存条目
#[tauri::command]
pub fn list_lyrics_cache(app: AppHandle) -> Vec<LyricEntry> {
    list(&app)
}

/// 按 key 读取歌词原文（二级界面进入时判断 dirty 用）
#[tauri::command]
pub fn get_lyrics_by_key(app: AppHandle, key: String) -> Option<String> {
    get_by_key(&app, &key)
}

/// 删除缓存条目
#[tauri::command]
pub fn delete_lyrics_entry(app: AppHandle, key: String) -> Result<(), String> {
    delete(&app, &key)
}
