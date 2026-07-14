use std::sync::Mutex;
use tauri::command;
use once_cell::sync::Lazy;
use reqwest::Client;
use base64::Engine;

// --- 引入 SMTC 需要的模块 ---
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    GlobalSystemMediaTransportControlsSession,
};
use windows::Foundation::TimeSpan;

// 全局记录当前选中的平台（默认空，由前端传来）
static TARGET_PLAYER: Mutex<String> = Mutex::new(String::new());

// 全局 HTTP 客户端单例，避免每次切歌都创建新的
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .expect("failed to build reqwest client")
});

// 给前端调用的切换接口
#[command]
pub fn set_target_player(player: String) {
    if let Ok(mut target) = TARGET_PLAYER.lock() {
        *target = player;
    }
}

// 自动匹配你选择的软件
fn get_target_media_session() -> Option<GlobalSystemMediaTransportControlsSession> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()?.get().ok()?;
    
    let sessions = manager.GetSessions().ok()?;

    // 获取当前的目标（前端如果还没传，默认用 netease）
    let target = {
        let guard = TARGET_PLAYER.lock().unwrap_or_else(|e| e.into_inner()); // 加入防中毒
        if guard.is_empty() { "netease".to_string() } else { guard.clone() }
    };

    // SMTC模式：返回第一个活动的媒体会话
    if target == "smtc" {
        // 收集为 Vec 以避免消耗迭代器后需要重新获取
        let sessions: Vec<_> = sessions.into_iter().collect();

        // 优先级1: 正在播放的会话
        for session in &sessions {
            if let Ok(playback_info) = session.GetPlaybackInfo() {
                if let Ok(status) = playback_info.PlaybackStatus() {
                    if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                        return Some(session.clone());
                    }
                }
            }
        }
        
        // 优先级2: 暂停但有媒体信息的会话
        for session in &sessions {
            if let Some(properties) = session.TryGetMediaPropertiesAsync().ok()?.get().ok() {
                if let Ok(title) = properties.Title() {
                    if !title.to_string().is_empty() {
                        return Some(session.clone());
                    }
                }
            }
        }
        
        return None;
    }

    // 非 SMTC 模式：按 AppUserModelId 匹配
    // 注意：如果上面 SMTC 分支已执行，sessions 已被 move，但那个分支必定 return，
    // 所以编译器知道只有非 SMTC 路径才会到达这里，sessions 未被消费。
    for session in sessions {
        if let Ok(app_id) = session.SourceAppUserModelId() {
            let app_id_str = app_id.to_string().to_lowercase();
            
            // 网易云特殊一点，包名可能叫 cloudmusic 或 netease
            if target == "netease" && (app_id_str.contains("cloudmusic") || app_id_str.contains("netease")) {
                return Some(session);
            } 
            // 其他软件直接用名字去系统进程列表里撞
            else if target != "netease" && app_id_str.contains(&target) {
                return Some(session);
            }
        }
    }
    None
}

#[command]
pub async fn fetch_netease_music_info() -> Result<Option<(String, String, bool)>, String> {
    let session = match get_target_media_session() {
        Some(s) => s,
        None => return Ok(None),
    };

    let is_playing = if let Ok(playback_info) = session.GetPlaybackInfo() {
        if let Ok(status) = playback_info.PlaybackStatus() {
            status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
        } else {
            false
        }
    } else {
        false
    };

    let properties = session.TryGetMediaPropertiesAsync()
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;

    let title = properties.Title().unwrap_or_default().to_string();
    let artist = properties.Artist().unwrap_or_default().to_string();

    if title.is_empty() {
        return Ok(None);
    }

    Ok(Some((title, artist, is_playing)))
}

#[command]
pub async fn control_system_media(action: String) -> Result<(), String> {
    if let Some(session) = get_target_media_session() {
        match action.as_str() {
            "play_pause" => { let _ = session.TryTogglePlayPauseAsync(); },
            "next" => { let _ = session.TrySkipNextAsync(); },
            "prev" => { let _ = session.TrySkipPreviousAsync(); },
            _ => {}
        }
    }
    Ok(())
}

// 利用微软官方 SMTC API 直接把网易云的本地封面榨出来
fn get_smtc_thumbnail() -> Option<String> {
    use windows::Storage::Streams::{Buffer, InputStreamOptions, DataReader};

    let session = get_target_media_session()?;
    let properties = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
    let thumbnail_ref = properties.Thumbnail().ok()?;
    let stream = thumbnail_ref.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 { return None; }

    let buffer = Buffer::Create(size).ok()?;
    stream.ReadAsync(&buffer, size, InputStreamOptions::None).ok()?.get().ok()?;
    let reader = DataReader::FromBuffer(&buffer).ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;

    Some(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)))
}

#[command]
pub async fn get_random_cover_url(song_name: String, artist_name: String) -> Result<String, String> {
    if let Some(base64_cover) = get_smtc_thumbnail() {
        return Ok(base64_cover);
    }

    let client = &*HTTP_CLIENT;

    let (tx, mut rx) = tokio::sync::mpsc::channel(3);

    // 1号赛道：Apple Music
    let tx_itunes = tx.clone();
    let client_itunes = client.clone();
    let query_itunes = format!("{} {}", song_name, artist_name);
    tokio::spawn(async move {
        let encoded_query = urlencoding::encode(&query_itunes).into_owned();
        let itunes_url = format!("https://itunes.apple.com/search?term={}&media=music&limit=1", encoded_query);
        if let Ok(resp) = client_itunes.get(&itunes_url).send().await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(artwork) = json.pointer("/results/0/artworkUrl100").and_then(|v| v.as_str()) {
                    let _ = tx_itunes.send(artwork.replace("100x100bb", "300x300bb")).await;
                }
            }
        }
    });

    // 2号赛道：网易云 API
    let tx_netease = tx.clone();
    let client_netease = client.clone();
    let song_netease = song_name.clone();
    let artist_netease = artist_name.clone();
    tokio::spawn(async move {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
        let query = format!("{} {}", song_netease, artist_netease);
        if let Ok(resp) = client_netease.post("https://music.163.com/api/search/get/web")
            .header("Referer", "https://music.163.com")
            .header("User-Agent", ua)
            .form(&[("s", query.as_str()), ("type", "1"), ("limit", "1"), ("offset", "0")])
            .send().await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(pic) = json.pointer("/result/songs/0/al/picUrl").and_then(|v| v.as_str()) {
                    if !pic.is_empty() && pic != "http://p4.music.126.net/UeTuwE7pvjBpypWLudqukQ==/3135032972947607.jpg" {
                        let _ = tx_netease.send(pic.replace("http://", "https://") + "?param=300y300").await;
                    }
                }
            }
        }
    });

    // 3号赛道：Deezer API
    let tx_deezer = tx.clone();
    let client_deezer = client.clone();
    let song_deezer = song_name.clone();
    let artist_deezer = artist_name.clone();
    tokio::spawn(async move {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
        let deezer_url = format!(
            "https://api.deezer.com/search?q=track:\"{}\" artist:\"{}\"&limit=1",
            urlencoding::encode(&song_deezer).into_owned(),
            urlencoding::encode(&artist_deezer).into_owned()
        );
        if let Ok(resp) = client_deezer.get(&deezer_url).header("User-Agent", ua).send().await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(cover) = json.pointer("/data/0/album/cover_medium").and_then(|v| v.as_str()) {
                    if !cover.is_empty() { let _ = tx_deezer.send(cover.to_string()).await; }
                } else if let Some(cover) = json.pointer("/data/0/album/cover_big").and_then(|v| v.as_str()) {
                    if !cover.is_empty() { let _ = tx_deezer.send(cover.to_string()).await; }
                }
            }
        }
    });

    match tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv()).await {
        Ok(Some(url)) => Ok(url),
        _ => Ok("data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTUwIiBoZWlnaHQ9IjE1MCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48ZGVmcz48bGluZWFyR3JhZGllbnQgaWQ9ImciIHgxPSIwJSIgeTE9IjAlIiB4Mj0iMTAwJSIgeTI9IjEwMCUiPjxzdG9wIG9mZnNldD0iMCUiIHN0b3AtY29sb3I9IiNhOGVkZWEiLz48c3RvcCBvZmZzZXQ9IjEwMCUiIHN0b3AtY29sb3I9IiNmZWQ2ZTMiLz48L2xpbmVhckdyYWRpZW50PjwvZGVmcz48cmVjdCB3aWR0aD0iMTUwIiBoZWlnaHQ9IjE1MCIgcng9Ijc1IiBmaWxsPSJ1cmwoI2cpIi8+PC9zdmc+".to_string()),
    }
}

// ===== F6 音乐进度条：读取 SMTC Timeline 并支持拖动定位 =====

#[derive(serde::Serialize)]
pub struct MusicTimeline {
    pub position_ms: u64,
    pub end_ms: u64,
}

/// 读取当前媒体会话的播放进度与总时长（毫秒）
#[command]
pub async fn get_music_timeline() -> Result<Option<MusicTimeline>, String> {
    let session = match get_target_media_session() {
        Some(s) => s,
        None => return Ok(None),
    };

    let timeline = session.GetTimelinePropertiesAsync()
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;

    // TimeSpan.Duration 单位为 100ns，除以 10000 得到毫秒
    let position_ms = timeline.Position()
        .map(|ts| (ts.Duration / 10000) as u64)
        .unwrap_or(0);
    let end_ms = timeline.EndTime()
        .map(|ts| (ts.Duration / 10000) as u64)
        .unwrap_or(0);

    // 总时长为 0 视为无有效进度信息（如直播流）
    if end_ms == 0 {
        return Ok(None);
    }

    Ok(Some(MusicTimeline { position_ms, end_ms }))
}

/// 拖动定位：跳转到指定播放位置（毫秒）
#[command]
pub async fn seek_music(position_ms: u64) -> Result<(), String> {
    let session = match get_target_media_session() {
        Some(s) => s,
        None => return Err("无活动媒体会话".to_string()),
    };

    // 毫秒转回 100ns 单位的 TimeSpan
    let ts = TimeSpan { Duration: (position_ms * 10000) as i64 };
    session.TrySeekAsync(ts)
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;

    Ok(())
}