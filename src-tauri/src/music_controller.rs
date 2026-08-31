use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::command;
use once_cell::sync::Lazy;
use reqwest::Client;
use base64::Engine;

// --- 引入 SMTC 需要的模块 ---
use windows::core::Interface;
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

/// SessionManager 是否已在「具备 COM 套间的线程」上完成初始化并缓存。
/// 异步 command 跑在 tokio 工作线程上，那些线程从未 CoInitialize，
/// 在那里首次触发 RequestAsync 可能失败或长时间阻塞，并污染后续缓存。
/// 未就绪时兜底路径直接返回 None，等 binder 线程就绪后再取。
static MANAGER_READY: AtomicBool = AtomicBool::new(false);

/// 最近一次成功提取到的元数据缓存（AUMID 小写, 歌名, 歌手）。
///
/// 存在意义：事件回调必须「零阻塞」——回调线程不能发起
/// `TryGetMediaPropertiesAsync` 这类跨进程异步调用（播放器冷启动时会卡住主线程 STA）。
/// 因此回调里构造快照时，标题从这里补，播放态走同步调用。
/// 由 `extract_music_info` 成功时写入。
static LAST_INFO: Mutex<Option<(String, String, String)>> = Mutex::new(None);

/// 获取（必要时创建并缓存）SMTC SessionManager。缓存命中时零 WinRT 异步调用。
pub(crate) fn get_cached_session_manager() -> Option<GlobalSystemMediaTransportControlsSessionManager> {
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

/// 在具备 COM 套间的线程（session_binder 的常驻 MTA 线程）上预热 SessionManager 并置就绪标志。
/// 返回是否成功就绪。
pub(crate) fn prime_session_manager() -> bool {
    let ready = get_cached_session_manager().is_some();
    if ready {
        MANAGER_READY.store(true, Ordering::SeqCst);
    }
    ready
}

/// SessionManager 是否就绪（供兜底轮询路径判断是否可以直接取会话）
pub(crate) fn is_session_manager_ready() -> bool {
    MANAGER_READY.load(Ordering::SeqCst)
}

/// 读取该 AUMID 缓存到的（歌名, 歌手）。缓存缺失或标题为空时返回 None。
pub(crate) fn cached_title(aumid: &str) -> Option<(String, String)> {
    let guard = LAST_INFO.lock().ok()?;
    let (cached_aumid, song, artist) = guard.as_ref()?;
    if cached_aumid == aumid && !song.is_empty() {
        Some((song.clone(), artist.clone()))
    } else {
        None
    }
}

fn set_cached_info(aumid: &str, song: &str, artist: &str) {
    if let Ok(mut guard) = LAST_INFO.lock() {
        *guard = Some((aumid.to_string(), song.to_string(), artist.to_string()));
    }
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
    let changed = {
        let mut target = TARGET_PLAYER.lock().unwrap_or_else(|e| e.into_inner());
        let changed = *target != player;
        *target = player;
        changed
    };
    // 目标变化后让会话绑定管理器重新选择要监听的会话（事件驱动链路）
    if changed {
        crate::session_binder::rebind_on_target_changed();
    }
}

/// 同步判断会话是否正在播放（GetPlaybackInfo 是本地缓存的同步调用，开销可忽略）
pub(crate) fn is_session_playing(
    session: &GlobalSystemMediaTransportControlsSession,
) -> bool {
    session
        .GetPlaybackInfo()
        .ok()
        .and_then(|p| p.PlaybackStatus().ok())
        .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false)
}

/// 读取会话的 AUMID（小写）。取不到时返回空串。
pub(crate) fn session_aumid(session: &GlobalSystemMediaTransportControlsSession) -> String {
    session
        .SourceAppUserModelId()
        .map(|id| id.to_string().to_lowercase())
        .unwrap_or_default()
}

// 自动匹配你选择的软件
pub(crate) fn get_target_media_session() -> Option<GlobalSystemMediaTransportControlsSession> {
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

        // 优先级0：已绑定的会话若仍在会话列表中且仍在播放，直接沿用。
        // 保证灵动岛 / 进度条(1Hz) / 封面 / 歌词指向同一目标，避免多会话时来回跳。
        if let Some(bound) = crate::session_binder::current_bound_session() {
            let still_listed = sessions.iter().any(|s| s.as_raw() == bound.as_raw());
            if still_listed && is_session_playing(&bound) {
                return Some(bound);
            }
        }

        // 优先级1: 正在播放的会话
        for session in &sessions {
            if is_douyin(session) {
                continue;
            }
            if is_session_playing(session) {
                return Some(session.clone());
            }
        }

        // 优先级2: 有缓存标题的会话（零阻塞，不再发起 TryGetMediaPropertiesAsync）
        for session in &sessions {
            if is_douyin(session) {
                continue;
            }
            if cached_title(&session_aumid(session)).is_some() {
                return Some(session.clone());
            }
        }

        // 优先级3（关键兜底）：绑定第一个非抖音会话。
        //
        // 播放器刚启动时：状态还不是 Playing（优先级1 落空）、标题也还是空（优先级2 落空）。
        // 旧实现此时 return None，导致 session_binder 完全不绑定任何会话——
        // 之后播放器切到 Playing 时我们没挂事件、收不到 PlaybackInfoChanged，
        // 若 SessionsChanged 不再触发就只能等 45s 兜底。这就是「延迟数秒」的根因。
        // 这里只要把事件挂上，等 MediaPropertiesChanged / PlaybackInfoChanged 唤醒即可。
        for session in &sessions {
            if is_douyin(session) {
                continue;
            }
            return Some(session.clone());
        }

        return None;
    }

    // 非 SMTC 模式：按 AppUserModelId 匹配
    // 注意：如果上面 SMTC 分支已执行，sessions 已被 move，但那个分支必定 return，
    // 所以编译器知道只有非 SMTC 路径才会到达这里，sessions 未被消费。
    for session in sessions {
        if let Ok(app_id) = session.SourceAppUserModelId() {
            let app_id_str = app_id.to_string().to_lowercase();

            // 抖音会话不算音乐，跳过本条继续找后面的（不能整体 return None：
            // 抖音排在会话列表前面时，后面的网易云等播放器会被一起放弃）
            if app_id_str.contains("douyin") {
                continue;
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

/// 从会话提取 (歌名, 歌手, 是否播放, 来源AUMID)，并把成功的标题写入 LAST_INFO 缓存。
/// None 表示该会话不产生有效音乐信息；空标题仍返回 Some（前端显示「已连接的应用名」）。
/// 供 fetch_netease_music_info（快照/兜底轮询）与 session_binder（事件推送）共用。
///
/// 注意：内部含阻塞的 `TryGetMediaPropertiesAsync().get()`（跨进程），
/// 只能在「可以阻塞的线程」上调用：
/// - 兜底轮询所在的 tokio 线程；
/// - session_binder 的常驻 MTA 线程（bind_to 第 2 步补发全量元数据）；
/// - SMTC 事件回调线程（MTA 的 RPC 线程池）：各回调在独立线程上投递，
///   阻塞单个回调既不会卡消息泵，也不会延迟其他事件的投递，
///   且 MediaPropertiesChanged 触发时属性刚更新（「热」），读取通常毫秒级。
///   切歌时新标题必须在这里读进 LAST_INFO 缓存，否则缓存永远是旧歌名。
/// **主线程（STA）绝不能调用本函数**；STA / 高频路径请用零阻塞的 `extract_quick_info`。
pub(crate) fn extract_music_info(
    session: &GlobalSystemMediaTransportControlsSession,
) -> Option<(String, String, bool, String)> {
    let result = extract_music_info_uncached(session);
    if let Some((song, artist, _playing, app_id)) = &result {
        if !song.is_empty() {
            set_cached_info(app_id, song, artist);
        }
    }
    result
}

/// 零阻塞快照：只做同步的 `GetPlaybackInfo` / `SourceAppUserModelId`，
/// 标题从 LAST_INFO 缓存补（缓存缺失时返回空标题，前端显示「已连接的应用名」）。
///
/// 供 SMTC 事件回调使用——回调跑在 WinRT 事件线程上，
/// 发起 `TryGetMediaPropertiesAsync` 会在播放器冷启动时卡住该线程数秒，
/// 导致后续事件排队、首帧延迟。
pub(crate) fn extract_quick_info(
    session: &GlobalSystemMediaTransportControlsSession,
) -> Option<(String, String, bool, String)> {
    let is_playing = is_session_playing(session);
    let app_id_str = session_aumid(session);
    let (song, artist) = cached_title(&app_id_str).unwrap_or((String::new(), String::new()));
    Some((song, artist, is_playing, app_id_str))
}

/// extract_music_info 的实际实现（不带缓存写入）
fn extract_music_info_uncached(
    session: &GlobalSystemMediaTransportControlsSession,
) -> Option<(String, String, bool, String)> {
    let is_playing = if let Ok(playback_info) = session.GetPlaybackInfo() {
        if let Ok(status) = playback_info.PlaybackStatus() {
            status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
        } else {
            false
        }
    } else {
        false
    };

    let properties = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;

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
        return Some((String::new(), String::new(), is_playing, app_id_str));
    }

    // 识别到抖音：直接判非音乐（会话选择层已按 AUMID 过滤，此处是双保险）
    if title.contains("抖音") || title.contains("douyin") {
        return None;
    }

    if app_id_str.contains("bilibili") { // 识别到哔哩哔哩
        return Some((title, "bilibili".to_string(), is_playing, app_id_str));
    }

    if app_id_str.contains("edge") { // 识别到 Edge 浏览器
        if artist.is_empty() {
            return Some((title, "edge".to_string(), is_playing, app_id_str));
        }
        return Some((title, artist, is_playing, app_id_str));
    }

    if app_id_str.contains("chrome") { // 识别到 Chrome 浏览器
        if artist.is_empty() {
            return Some((title, "chrome".to_string(), is_playing, app_id_str));
        }
        return Some((title, artist, is_playing, app_id_str));
    }

    if app_id_str.contains("potplayer") { // 识别到 PotPlayer
        if artist.is_empty() {
            return Some((title, "potplayer".to_string(), is_playing, app_id_str));
        }
        return Some((title, artist, is_playing, app_id_str));
    }

    Some((title, artist, is_playing, app_id_str))
}

#[command]
pub async fn fetch_netease_music_info() -> Result<Option<(String, String, bool, String)>, String> {
    // SessionManager 未就绪时不要在这里触发 RequestAsync：
    // 异步 command 跑在 tokio 工作线程上，那些线程没有 COM 套间，
    // 首次 RequestAsync 可能失败或长时间阻塞。返回 None 交给下一次轮询重试。
    if !is_session_manager_ready() {
        return Ok(None);
    }

    let session = match get_target_media_session() {
        Some(s) => s,
        None => return Ok(None),
    };

    // 取属性失败 ≠ 无音乐（播放器刚启动时常见）。会话还在，退回零阻塞快照，
    // 与事件驱动链路语义一致，避免把 UI 误清空成「未在播放歌曲」。
    Ok(extract_music_info(&session).or_else(|| extract_quick_info(&session)))
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

/// QQ 歌词 API 返回的 HTML 实体解码
fn decode_qq_lyric(lyric_text: &str) -> String {
    lyric_text
        .replace("&#10;", "\n")
        .replace("&#13;", "\r")
        .replace("&#32;", " ")
        .replace("&#45;", "-")
        .replace("&#40;", "(")
        .replace("&#41;", ")")
}

/// 拉取歌词：本地缓存 → QQ 音乐 → 网易云兜底。由 lib.rs generate_handler 注册为命令。
#[command]
pub async fn fetch_netease_lyrics(
    app: tauri::AppHandle,
    song_name: String,
    artist_name: String,
    duration_ms: i64,
) -> Result<String, String> {
    // 6a：本地缓存优先（key = 规范化(歌名)+规范化(歌手)+时长，±2s 容差），命中直接用
    if let Some(cached) = crate::lyrics_cache::lookup(&song_name, &artist_name, duration_ms, &app) {
        return Ok(cached);
    }

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
                                let decoded = decode_qq_lyric(&lyric_text);
                                if !decoded.is_empty() {
                                    println!("[网络歌词调试] 命中 QQ音乐 API (已通过双重校验)");
                                    // 6a：命中即落盘（自动缓存）
                                    let _ = crate::lyrics_cache::save(&app, &song_name, &artist_name, duration_ms, &decoded, "auto");
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
                                // 6a：命中即落盘（自动缓存）
                                let _ = crate::lyrics_cache::save(&app, &song_name, &artist_name, duration_ms, lyric_text, "auto");
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

// ===== 歌词管理界面（6b）后端 command =====

/// 当前播放曲目聚合信息（SMTC），未检测到播放返回 None
#[derive(serde::Serialize)]
pub struct CurrentTrack {
    pub song: String,
    pub artist: String,
    pub duration_ms: i64,
    pub app_id: String,
}

#[command]
pub async fn import_current_track() -> Result<Option<CurrentTrack>, String> {
    let Some(session) = get_target_media_session() else {
        return Ok(None);
    };
    let Some((song, artist, _playing, app_id)) = extract_music_info(&session) else {
        return Ok(None);
    };
    if song.is_empty() {
        return Ok(None);
    }
    let duration_ms = read_timeline_bounds(&session)
        .ok()
        .flatten()
        .map(|b| (b.end_ms - b.start_ms) as i64)
        .unwrap_or(0);
    Ok(Some(CurrentTrack {
        song,
        artist,
        duration_ms,
        app_id,
    }))
}

/// 歌词搜索候选（双源聚合：QQ 10 条 + 网易 10 条，无自动匹配，由用户选择）
#[derive(serde::Serialize)]
pub struct LyricCandidate {
    /// "qq" | "netease"
    pub source: String,
    /// QQ songmid 或 网易 song id
    pub id: String,
    pub song: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: i64,
}

#[command]
pub async fn search_lyrics_candidates(
    song_name: String,
    artist_name: String,
) -> Result<Vec<LyricCandidate>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?;
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
    let query = if artist_name.is_empty() {
        song_name.clone()
    } else {
        format!("{} {}", song_name, artist_name)
    };
    let mut candidates: Vec<LyricCandidate> = Vec::new();

    // QQ 音乐搜索（10 条）
    let qq_search_url = format!(
        "https://c.y.qq.com/soso/fcgi-bin/client_search_cp?w={}&n=10&format=json",
        urlencoding::encode(&query)
    );
    if let Ok(resp) = client.get(&qq_search_url).header("User-Agent", ua).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(songs) = json.pointer("/data/song/list").and_then(|v| v.as_array()) {
                for song in songs {
                    let Some(id) = song.get("songmid").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let name = song.get("songname").and_then(|v| v.as_str()).unwrap_or("");
                    let album = song
                        .get("albumname")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let artist = song
                        .get("singer")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| s.get("name").and_then(|v| v.as_str()))
                                .collect::<Vec<_>>()
                                .join("/")
                        })
                        .unwrap_or_default();
                    let duration_ms = song
                        .get("interval")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0)
                        * 1000;
                    candidates.push(LyricCandidate {
                        source: "qq".into(),
                        id: id.to_string(),
                        song: name.to_string(),
                        artist,
                        album,
                        duration_ms,
                    });
                }
            }
        }
    }

    // 网易云搜索（10 条，伪造随机 IP 避免风控，与播放链路一致）
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
            ("limit", "10"),
            ("offset", "0"),
        ])
        .send()
        .await
    {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(songs) = json.pointer("/result/songs").and_then(|v| v.as_array()) {
                for song in songs {
                    let Some(id) = song.get("id").and_then(|v| v.as_i64()) else {
                        continue;
                    };
                    let name = song.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let artist = song
                        .get("artists")
                        .or(song.get("ar"))
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| s.get("name").and_then(|v| v.as_str()))
                                .collect::<Vec<_>>()
                                .join("/")
                        })
                        .unwrap_or_default();
                    let album = song
                        .get("album")
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let duration_ms = song
                        .get("duration")
                        .or(song.get("dt"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    candidates.push(LyricCandidate {
                        source: "netease".into(),
                        id: id.to_string(),
                        song: name.to_string(),
                        artist,
                        album,
                        duration_ms,
                    });
                }
            }
        }
    }

    Ok(candidates)
}

/// 按候选拉取完整 LRC 原文（时间轴全保留）
#[command]
pub async fn get_lyrics_by_candidate(source: String, id: String) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|e| e.to_string())?;
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";

    if source == "qq" {
        let url = format!(
            "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg?songmid={}&format=json&nobase64=1",
            urlencoding::encode(&id)
        );
        let resp = client
            .get(&url)
            .header("Referer", "https://y.qq.com/")
            .header("User-Agent", ua)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let lyric = json
            .get("lyric")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let decoded = decode_qq_lyric(lyric);
        if decoded.trim().is_empty() {
            return Err("该候选未提供歌词".into());
        }
        return Ok(decoded);
    }

    if source == "netease" {
        let url = format!(
            "https://music.163.com/api/song/lyric?id={}&lv=-1&kv=-1&tv=-1",
            urlencoding::encode(&id)
        );
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
        let resp = client
            .get(&url)
            .header("User-Agent", ua)
            .header("X-Real-IP", &fake_ip)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let lyric = json
            .pointer("/lrc/lyric")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if lyric.trim().is_empty() {
            return Err("该候选未提供歌词".into());
        }
        return Ok(lyric.to_string());
    }

    Err("未知歌词来源".into())
}