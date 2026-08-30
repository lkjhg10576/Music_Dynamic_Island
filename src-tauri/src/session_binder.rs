//! SMTC 会话绑定管理器：事件驱动的音乐元数据 / 播放态推送。
//! 替代前端 3s 轮询（fetch_netease_music_info 保留作为启动快照 + 低频兜底）；
//! 进度条 1s 校准链路（get_music_timeline）不受影响。
//!
//! 核心结构：CURRENT_SESSION 长期持有绑定的会话——SMTC 事件挂在 Session 对象上，
//! 对象释放即停发，这是必须引入全局状态的根本原因。
//!
//! 重入安全：所有回调锁内只取快照 / 比较指针，WinRT 调用与 emit 一律在锁外完成；
//! 事件挂载与解绑同样在锁外进行，锁不做任何 WinRT 调用。
//! rebind 在回调线程同步执行：其中所有 WinRT 调用均为缓存命中后的毫秒级同步调用，
//! 不会长时间占用 WinRT 事件回调线程。

use once_cell::sync::Lazy;
use std::sync::Mutex;
use tauri::Emitter;
use windows::core::Interface;
use windows::Foundation::{EventRegistrationToken, TypedEventHandler};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

use crate::music_controller::{
    extract_music_info, get_cached_session_manager, get_target_media_session,
};

struct BoundSession {
    session: GlobalSystemMediaTransportControlsSession,
    media_token: EventRegistrationToken,
    playback_token: EventRegistrationToken,
}

static CURRENT_SESSION: Lazy<Mutex<Option<BoundSession>>> = Lazy::new(|| Mutex::new(None));
static APP_HANDLE: Lazy<Mutex<Option<tauri::AppHandle>>> = Lazy::new(|| Mutex::new(None));
static SESSIONS_TOKEN: Lazy<Mutex<Option<EventRegistrationToken>>> = Lazy::new(|| Mutex::new(None));

type ManagerHandler = TypedEventHandler<
    GlobalSystemMediaTransportControlsSessionManager,
    windows::core::IInspectable,
>;
type SessionHandler =
    TypedEventHandler<GlobalSystemMediaTransportControlsSession, windows::core::IInspectable>;

fn app_handle() -> Option<tauri::AppHandle> {
    APP_HANDLE.lock().unwrap().clone()
}

/// 注册「会话列表变化」事件：播放器退出 / 新启动时触发 rebind
fn register_sessions_changed(
    manager: &GlobalSystemMediaTransportControlsSessionManager,
    handler: &ManagerHandler,
) -> windows::core::Result<EventRegistrationToken> {
    manager.SessionsChanged(handler)
}

/// 注册「媒体元数据变化」事件：歌名 / 歌手 / 专辑变化时触发
fn register_media_changed(
    session: &GlobalSystemMediaTransportControlsSession,
    handler: &SessionHandler,
) -> windows::core::Result<EventRegistrationToken> {
    session.MediaPropertiesChanged(handler)
}

/// 注册「播放信息变化」事件：播放 / 暂停状态变化时触发
fn register_playback_changed(
    session: &GlobalSystemMediaTransportControlsSession,
    handler: &SessionHandler,
) -> windows::core::Result<EventRegistrationToken> {
    session.PlaybackInfoChanged(handler)
}

/// setup 阶段调用：注册 manager 会话列表变化事件并绑定初始会话。
/// 此时 TARGET_PLAYER 可能尚未由前端下发，先按默认目标绑定；
/// 前端 set_target_player 到达后会触发一次 rebind 纠正。
pub fn init(app: tauri::AppHandle) {
    *APP_HANDLE.lock().unwrap() = Some(app.clone());

    if let Some(manager) = get_cached_session_manager() {
        let sessions_callback = move |_manager, _args| -> windows::core::Result<()> {
            if let Some(app) = app_handle() {
                rebind(app, true);
            }
            Ok(())
        };
        let handler = ManagerHandler::new(sessions_callback);
        let sessions_reg = register_sessions_changed(&manager, &handler);
        if let Ok(token) = sessions_reg {
            *SESSIONS_TOKEN.lock().unwrap() = Some(token);
        }
    }

    rebind(app, false);
}

/// 前端 set_target_player 变更后调用：目标变了，重新选择要绑定的会话
pub fn rebind_on_target_changed() {
    if let Some(app) = app_handle() {
        rebind(app, true);
    }
}

/// 重新选择目标会话并绑定。`force_emit=true` 时即使会话未变也推送一次全量信息。
fn rebind(app: tauri::AppHandle, force_emit: bool) {
    let Some(new_session) = get_target_media_session() else {
        // 无可用会话：解绑当前并通知前端「未在播放」
        let old = take_bound_session();
        drop(old);
        if force_emit {
            emit_music_info(app, None);
        }
        return;
    };

    // 与当前绑定是同一会话 → 不重绑（避免重复挂事件），按需补推快照
    {
        let guard = CURRENT_SESSION.lock().unwrap();
        if let Some(bound) = guard.as_ref() {
            if bound.session.as_raw() == new_session.as_raw() {
                drop(guard);
                if force_emit {
                    let info = extract_music_info(&new_session);
                    emit_music_info(app, info);
                }
                return;
            }
        }
    }

    bind_to(app, new_session);
}

/// 绑定指定会话：先挂事件（锁外），再解绑旧会话（锁外），最后锁内置入并立即推送一次快照
fn bind_to(app: tauri::AppHandle, new_session: GlobalSystemMediaTransportControlsSession) {
    // —— 锁外：挂元数据变化事件（→ 全量推送） ——
    let app_for_media = app.clone();
    let media_callback = move |session, _args| -> windows::core::Result<()> {
        let Some(session) = session else {
            return Ok(());
        };
        handle_media_properties_changed(app_for_media.clone(), session);
        Ok(())
    };
    let media_handler = SessionHandler::new(media_callback);
    let media_token = match register_media_changed(&new_session, &media_handler) {
        Ok(t) => t,
        Err(_) => return, // 挂事件失败：不换绑，保留旧绑定继续工作
    };

    // —— 锁外：挂播放态变化事件（→ 轻量推送 + 自动切应用） ——
    let app_for_playback = app.clone();
    let playback_callback = move |session, _args| -> windows::core::Result<()> {
        let Some(session) = session else {
            return Ok(());
        };
        handle_playback_info_changed(app_for_playback.clone(), session);
        Ok(())
    };
    let playback_handler = SessionHandler::new(playback_callback);
    let playback_token = match register_playback_changed(&new_session, &playback_handler) {
        Ok(t) => t,
        Err(_) => {
            let _ = new_session.RemoveMediaPropertiesChanged(media_token);
            return;
        }
    };

    // —— 锁外：解绑旧会话 ——
    let old = take_bound_session();
    drop(old);

    // —— 锁内置入 ——
    *CURRENT_SESSION.lock().unwrap() = Some(BoundSession {
        session: new_session.clone(),
        media_token,
        playback_token,
    });

    // 换绑后立即推送一次全量信息，UI 无需等待下一次事件
    let info = extract_music_info(&new_session);
    emit_music_info(app, info);
}

/// 锁内取出当前绑定（返回后调用方 drop 以在锁外执行 Remove* 解绑）
fn take_bound_session() -> Option<BoundSession> {
    CURRENT_SESSION.lock().unwrap().take()
}

/// 元数据变化回调处理：推送全量音乐信息
fn handle_media_properties_changed(
    app: tauri::AppHandle,
    session: GlobalSystemMediaTransportControlsSession,
) {
    // 确认该会话仍是当前绑定（防旧会话事件串扰）；锁内只做指针比较
    let still_bound = is_currently_bound(&session);
    if !still_bound {
        return;
    }
    let info = extract_music_info(&session);
    emit_music_info(app, info);
}

/// 播放态变化回调处理：轻量推送；
/// 语义对齐现状（3s 轮询的自动切应用）：当前会话转暂停时，
/// 若存在其他正在播放的音乐会话则切换绑定。
fn handle_playback_info_changed(
    app: tauri::AppHandle,
    session: GlobalSystemMediaTransportControlsSession,
) {
    if !is_currently_bound(&session) {
        return;
    }

    let playing = is_session_playing(&session);
    if !playing {
        // 当前会话暂停：优先切到其他正在播放的会话（换绑内部会 emit 全量信息）
        if let Some(alternative) = find_playing_session_excluding(&session) {
            bind_to(app, alternative);
            return;
        }
    }

    let _ = app.emit(
        "music-playback-changed",
        serde_json::json!({ "playing": playing }),
    );
}

/// 锁内只做指针比较，判断会话是否仍是当前绑定
fn is_currently_bound(session: &GlobalSystemMediaTransportControlsSession) -> bool {
    let guard = CURRENT_SESSION.lock().unwrap();
    guard
        .as_ref()
        .map(|b| b.session.as_raw() == session.as_raw())
        .unwrap_or(false)
}

fn is_session_playing(session: &GlobalSystemMediaTransportControlsSession) -> bool {
    session
        .GetPlaybackInfo()
        .ok()
        .and_then(|p| p.PlaybackStatus().ok())
        .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false)
}

/// 在其余会话中寻找正在播放的音乐会话（排除抖音，与轮询选择逻辑一致）
fn find_playing_session_excluding(
    current: &GlobalSystemMediaTransportControlsSession,
) -> Option<GlobalSystemMediaTransportControlsSession> {
    let manager = get_cached_session_manager()?;
    for session in manager.GetSessions().ok()? {
        if session.as_raw() == current.as_raw() {
            continue;
        }
        let is_douyin = session
            .SourceAppUserModelId()
            .map(|id| id.to_string().to_lowercase().contains("douyin"))
            .unwrap_or(false);
        if is_douyin {
            continue;
        }
        if is_session_playing(&session) {
            return Some(session);
        }
    }
    None
}

/// 推送全量音乐信息；info=None 表示当前无可用音乐会话（前端清空展示）
fn emit_music_info(app: tauri::AppHandle, info: Option<(String, String, bool, String)>) {
    let payload = match info {
        Some((song, artist, playing, app_id)) => serde_json::json!({
            "song": song,
            "artist": artist,
            "playing": playing,
            "appId": app_id,
        }),
        None => serde_json::Value::Null,
    };
    let _ = app.emit("music-info-changed", payload);
}
