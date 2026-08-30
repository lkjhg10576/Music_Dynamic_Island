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

// 全局记录当前选中的平台（默认空，由前端传来）
static TARGET_PLAYER: Mutex<String> = Mutex::new(String::new());

// 缓存 SMTC SessionManager：RequestAsync 是较重的 WinRT 异步，fetchTimeline(1Hz)/syncMusicStatus(0.33Hz)
// 每次都重建会累积 COM 对象分配开销。首次请求后复用同一实例；GetSessions 每次仍返回最新会话列表，
// 无需重复 RequestAsync。SessionManager 为系统级单例，长期有效。
static SESSION_MANAGER: Lazy<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>> =
    Lazy::new(|| Mutex::new(None));

/// 获取（必要时创建并缓存）SMTC SessionManager。缓存命中时零 WinRT 异步调用。
fn get_cached_session_manager() -> Option<GlobalSystemMediaTransportControlsSessionManager> {
    {
        let guard = SESSION_MANAGER.lock().ok()?;
        if let Some(m) = guard.as_ref() {
            return Some(m.clone());
        }
    }
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()?
        .get()
        .ok()?;
    if let Ok(mut guard) = SESSION_MANAGER.lock() {
        *guard = Some(manager.clone());
    }
    Some(manager)
}

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
    let manager = match get_cached_session_manager() {
        Some(m) => m,
        None => return None,
    };
    
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

        // 抖音会话不算音乐：刷抖音视频时不把抖音当音乐显示
        let is_douyin = |session: &GlobalSystemMediaTransportControlsSession| {
            session
                .SourceAppUserModelId()
                .map(|id| id.to_string().to_lowercase().contains("douyin"))
                .unwrap_or(false)
        };

        // 优先级1: 正在播放的会话
        for session in &sessions {
            if is_douyin(session) {
                continue;
            }
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
            if is_douyin(session) {
                continue;
            }
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

            // 抖音会话不算音乐，直接放弃匹配（对齐上游）
            if app_id_str.contains("douyin") {
                return None;
            }

            // 网易云特殊一点，包名可能叫 cloudmusic 或 netease
            if target == "netease" && (app_id_str.contains("cloudmusic") || app_id_str.contains("netease")) {
                return Some(session);
            }
            // 洛雪音乐：官方包名叫 cn.toside.music.desktop，用 lx-music 作为备用包名
            else if target == "lx-music"
                && (app_id_str.contains("cn.toside.music.desktop") || app_id_str.contains("lx-music"))
            {
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
pub async fn fetch_netease_music_info() -> Result<Option<(String, String, bool, String)>, String> {
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

    // 当前会话来源应用的 AUMID（小写），供前端区分浏览器/视频类来源与应用 logo
    let app_id_str = session
        .SourceAppUserModelId()
        .map(|id| id.to_string().to_lowercase())
        .unwrap_or_default();

    if title.is_empty() {
        // SMTC 已连上应用但尚未提供有效标题：仍返回会话信息（空标题 + 应用包名），
        // 让前端把单行展示改为显示"已连接的应用名"，而不是"未在播放"
        return Ok(Some((String::new(), String::new(), is_playing, app_id_str)));
    }

    // 识别到抖音：直接判非音乐（会话选择层已按 AUMID 过滤，此处是双保险）
    if title.contains("抖音") || title.contains("douyin") {
        return Ok(None);
    }

    if app_id_str.contains("bilibili") { // 识别到哔哩哔哩
        return Ok(Some((title, "bilibili".to_string(), is_playing, app_id_str)));
    }

    if app_id_str.contains("edge") { // 识别到 Edge 浏览器
        if artist.is_empty() {
            return Ok(Some((title, "edge".to_string(), is_playing, app_id_str)));
        }
        return Ok(Some((title, artist, is_playing, app_id_str)));
    }

    if app_id_str.contains("chrome") { // 识别到 Chrome 浏览器
        if artist.is_empty() {
            return Ok(Some((title, "chrome".to_string(), is_playing, app_id_str)));
        }
        return Ok(Some((title, artist, is_playing, app_id_str)));
    }

    if app_id_str.contains("potplayer") { // 识别到 PotPlayer
        if artist.is_empty() {
            return Ok(Some((title, "potplayer".to_string(), is_playing, app_id_str)));
        }
        return Ok(Some((title, artist, is_playing, app_id_str)));
    }

    Ok(Some((title, artist, is_playing, app_id_str)))
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

// 仅尝试读取 SMTC 本地封面，不联网兜底（浏览器/视频类应用专用，避免视频标题在网络搜图时串错封面）
#[command]
pub async fn get_smtc_cover() -> Result<Option<String>, String> {
    Ok(get_smtc_thumbnail())
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
    /// 相对于有效播放区间起点的位置（毫秒）
    pub position_ms: u64,
    /// 有效播放区间总时长（毫秒）
    pub end_ms: u64,
    pub can_seek: bool,
}

struct TimelineBounds {
    start_ms: u64,
    end_ms: u64,
    position_ms: u64,
    can_seek: bool,
}

fn timespan_to_ms(duration: i64) -> Option<u64> {
    if duration < 0 {
        None
    } else {
        Some((duration as u64) / 10_000)
    }
}

fn read_timeline_bounds(
    session: &GlobalSystemMediaTransportControlsSession,
) -> Result<Option<TimelineBounds>, String> {
    let timeline = session.GetTimelineProperties().map_err(|e| e.to_string())?;

    let start_ms = timeline.StartTime().ok()
        .and_then(|value| timespan_to_ms(value.Duration))
        .unwrap_or(0);
    let end_ms = timeline.EndTime().ok()
        .and_then(|value| timespan_to_ms(value.Duration))
        .unwrap_or(0);
    let min_seek_ms = timeline.MinSeekTime().ok()
        .and_then(|value| timespan_to_ms(value.Duration))
        .unwrap_or(start_ms);
    let max_seek_ms = timeline.MaxSeekTime().ok()
        .and_then(|value| timespan_to_ms(value.Duration))
        .unwrap_or(end_ms);
    let position_ms = timeline.Position().ok()
        .and_then(|value| timespan_to_ms(value.Duration))
        .unwrap_or(start_ms);

    // 部分播放器只上报 seek 范围，另一些只上报 StartTime/EndTime。
    let has_seek_range = max_seek_ms > min_seek_ms;
    let effective_start = if has_seek_range { min_seek_ms } else { start_ms };
    let effective_end = if has_seek_range { max_seek_ms } else { end_ms };
    if effective_end <= effective_start {
        return Ok(None);
    }

    let can_seek = session.GetPlaybackInfo().ok()
        .and_then(|info| info.Controls().ok())
        .and_then(|controls| controls.IsPlaybackPositionEnabled().ok())
        .unwrap_or(has_seek_range);

    Ok(Some(TimelineBounds {
        start_ms: effective_start,
        end_ms: effective_end,
        position_ms: position_ms.clamp(effective_start, effective_end),
        can_seek,
    }))
}

/// 读取当前媒体会话的归一化播放进度与总时长（毫秒）
#[command]
pub async fn get_music_timeline() -> Result<Option<MusicTimeline>, String> {
    let session = match get_target_media_session() {
        Some(session) => session,
        None => return Ok(None),
    };
    let bounds = match read_timeline_bounds(&session)? {
        Some(bounds) => bounds,
        None => return Ok(None),
    };

    Ok(Some(MusicTimeline {
        position_ms: bounds.position_ms.saturating_sub(bounds.start_ms),
        end_ms: bounds.end_ms.saturating_sub(bounds.start_ms),
        can_seek: bounds.can_seek,
    }))
}

/// 拖动定位：跳转到相对于有效播放区间起点的位置（毫秒）
#[command]
pub async fn seek_music(position_ms: u64) -> Result<(), String> {
    let session = get_target_media_session().ok_or_else(|| "无活动媒体会话".to_string())?;
    let bounds = read_timeline_bounds(&session)?
        .ok_or_else(|| "当前媒体未提供有效播放进度".to_string())?;
    if !bounds.can_seek {
        return Err("当前媒体不支持拖动定位".to_string());
    }

    let duration_ms = bounds.end_ms.saturating_sub(bounds.start_ms);
    let absolute_ms = bounds.start_ms.saturating_add(position_ms.min(duration_ms));
    let position_ticks = absolute_ms.saturating_mul(10_000).min(i64::MAX as u64) as i64;
    let changed = session.TryChangePlaybackPositionAsync(position_ticks)
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;
    if !changed {
        return Err("播放器拒绝了定位请求".to_string());
    }

    Ok(())
}

// ===== 网络歌词：QQ 音乐引擎优先，网易云兜底 =====

#[command]
pub async fn fetch_netease_lyrics(
    song_name: String,
    artist_name: String,
    duration_ms: i64,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?;

    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
    let query = format!("{} {}", song_name, artist_name);
    let query_name_lower = song_name.to_lowercase();
    let query_artist_lower = artist_name.to_lowercase(); // 歌手小写比对

    // ENGINE 1: QQ MUSIC (极速国内优选源)
    let qq_search_url = format!(
        "https://c.y.qq.com/soso/fcgi-bin/client_search_cp?w={}&n=5&format=json",
        urlencoding::encode(&query)
    );

    if let Ok(resp) = client
        .get(&qq_search_url)
        .header("User-Agent", ua)
        .send()
        .await
    {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(songs) = json.pointer("/data/song/list").and_then(|v| v.as_array()) {
                let mut best_songmid = None;

                for song in songs {
                    let songmid = song.get("songmid").and_then(|v| v.as_str());
                    let interval = song.get("interval").and_then(|v| v.as_i64()).unwrap_or(0);
                    let name = song
                        .get("songname")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();

                    // 提取 QQ 音乐歌手名
                    let mut singer_name = String::new();
                    if let Some(singers) = song.get("singer").and_then(|v| v.as_array()) {
                        for s in singers {
                            if let Some(sname) = s.get("name").and_then(|v| v.as_str()) {
                                singer_name.push_str(&sname.to_lowercase());
                            }
                        }
                    }

                    let name_match =
                        name.contains(&query_name_lower) || query_name_lower.contains(&name);
                    let artist_match = singer_name.contains(&query_artist_lower)
                        || query_artist_lower.contains(&singer_name)
                        || query_artist_lower.is_empty();

                    if let Some(mid) = songmid {
                        if duration_ms > 0 {
                            let diff = (interval * 1000 - duration_ms).abs();
                            // 必须名字匹配，且 (歌手匹配 或 时间误差极小)
                            if name_match && (artist_match || diff <= 3000) {
                                best_songmid = Some(mid.to_string());
                                break;
                            }
                        } else if name_match && artist_match {
                            best_songmid = Some(mid.to_string());
                            break;
                        }
                    }
                }

                if let Some(songmid) = best_songmid {
                    let qq_lyric_url = format!("https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg?songmid={}&format=json&nobase64=1", songmid);
                    if let Ok(lyric_resp) = client
                        .get(&qq_lyric_url)
                        .header("Referer", "https://y.qq.com/")
                        .header("User-Agent", ua)
                        .send()
                        .await
                    {
                        if let Ok(lyric_json) = lyric_resp.json::<serde_json::Value>().await {
                            if let Some(lyric_text) =
                                lyric_json.get("lyric").and_then(|v| v.as_str())
                            {
                                let decoded = lyric_text
                                    .replace("&#10;", "\n")
                                    .replace("&#13;", "\r")
                                    .replace("&#32;", " ")
                                    .replace("&#45;", "-")
                                    .replace("&#40;", "(")
                                    .replace("&#41;", ")");
                                if !decoded.is_empty() {
                                    println!("[网络歌词调试] 命中 QQ音乐 API (已通过双重校验)");
                                    return Ok(decoded);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ENGINE 2: NETEASE FALLBACK (网易云兜底，伪造随机 IP 避免风控)
    let fake_ip = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!(
            "{}.{}.{}.{}",
            rng.gen_range(11..250),
            rng.gen_range(11..250),
            rng.gen_range(11..250),
            rng.gen_range(11..250)
        )
    };

    if let Ok(resp) = client
        .post("https://music.163.com/api/search/get/web")
        .header("Referer", "https://music.163.com")
        .header("User-Agent", ua)
        .header("X-Real-IP", &fake_ip)
        .form(&[
            ("s", query.as_str()),
            ("type", "1"),
            ("limit", "8"),
            ("offset", "0"),
        ])
        .send()
        .await
    {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(songs) = json.pointer("/result/songs").and_then(|v| v.as_array()) {
                let mut best_song_id = None;
                let mut min_diff = i64::MAX;

                for song in songs {
                    let song_duration = song
                        .get("duration")
                        .or(song.get("dt"))
                        .and_then(|v| v.as_i64());
                    let id = song.get("id").and_then(|v| v.as_i64());
                    let name = song
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();

                    // 提取网易云歌手名进行比对
                    let mut singer_name = String::new();
                    if let Some(artists) = song
                        .get("artists")
                        .or(song.get("ar"))
                        .and_then(|v| v.as_array())
                    {
                        for a in artists {
                            if let Some(aname) = a.get("name").and_then(|v| v.as_str()) {
                                singer_name.push_str(&aname.to_lowercase());
                            }
                        }
                    }

                    let name_match =
                        name.contains(&query_name_lower) || query_name_lower.contains(&name);
                    let artist_match = singer_name.contains(&query_artist_lower)
                        || query_artist_lower.contains(&singer_name)
                        || query_artist_lower.is_empty();

                    if let (Some(id), Some(song_dur)) = (id, song_duration) {
                        if duration_ms > 0 {
                            let diff = (song_dur - duration_ms).abs();
                            // 必须名字匹配，且 (歌手匹配 或 时间误差极小) 才算命中
                            if name_match && (artist_match || diff <= 3000) {
                                if diff < min_diff {
                                    min_diff = diff;
                                    best_song_id = Some(id);
                                }
                            }
                        } else if name_match && artist_match {
                            best_song_id = Some(id);
                            break;
                        }
                    }
                }

                if let Some(song_id) = best_song_id {
                    let lyric_url = format!(
                        "https://music.163.com/api/song/lyric?id={}&lv=-1&kv=-1&tv=-1",
                        song_id
                    );
                    if let Ok(lyric_resp) = client
                        .get(&lyric_url)
                        .header("User-Agent", ua)
                        .header("X-Real-IP", &fake_ip)
                        .send()
                        .await
                    {
                        if let Ok(lyric_json) = lyric_resp.json::<serde_json::Value>().await {
                            if let Some(lyric_text) =
                                lyric_json.pointer("/lrc/lyric").and_then(|v| v.as_str())
                            {
                                println!("[网络歌词调试] 命中网易云 API 兜底 (已通过双重校验)");
                                return Ok(lyric_text.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    println!("[网络歌词调试] 失败：所有网络接口均未找到匹配歌词，或未通过双重校验");
    Ok("".to_string())
}