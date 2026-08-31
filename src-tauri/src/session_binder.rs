//! SMTC 会话绑定管理器：事件驱动的音乐元数据 / 播放态推送。
//! 替代前端 3s 轮询（fetch_netease_music_info 保留作为启动快照 + 45s 低频兜底）；
//! 进度条 1s 校准链路（get_music_timeline）不受影响。
//!
//! 核心结构：CURRENT_SESSION 长期持有绑定的会话——SMTC 事件挂在 Session 对象上，
//! 对象释放即停发，这是必须引入全局状态的根本原因。
//!
//! ## 线程模型（关键）
//! 全部 WinRT 调用都在一条**常驻 MTA 后台线程**（binder_thread）及其回调线程上完成，
//! 主线程 STA 完全不参与：
//! - `RequestAsync` / `TryGetMediaPropertiesAsync` 这类跨进程异步调用会阻塞调用线程，
//!   放在主线程会卡住消息泵，导致后续事件排队、首帧延迟。
//! - MTA 上注册的 WinRT 事件由 RPC 线程池投递，不需要消息泵，
//!   binder_thread 阻塞在 `recv()` 也不影响事件接收。
//!
//! ## 两段式推送
//! 事件回调只发**零阻塞快照**（`extract_quick_info`：同步的播放态 + 缓存标题），
//! 保证 UI 立刻响应；全量元数据（含阻塞的属性读取）随后补发。
//!
//! ## 重入安全（硬约束）
//! 所有回调锁内只取快照 / 比较身份，WinRT 调用与 emit 一律在锁外完成；
//! 事件挂载与解绑同样在锁外进行（`Drop` 会调 Remove*，必须保证锁已释放）。
//! `Mutex` 不可重入，锁内绝不能触发回调或 WinRT 调用。

use once_cell::sync::Lazy;
use std::sync::mpsc;
use std::sync::Mutex;
use tauri::Emitter;
use windows::core::Interface;
use windows::Foundation::{EventRegistrationToken, TypedEventHandler};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    MediaPropertiesChangedEventArgs, PlaybackInfoChangedEventArgs, SessionsChangedEventArgs,
};

use crate::music_controller::{
    extract_music_info, extract_quick_info, get_cached_session_manager, get_target_media_session,
    is_session_playing, prime_session_manager, session_aumid,
};

/// 推送给前端的完整载荷：Some((歌名, 歌手, 是否播放, AUMID)) / None 表示无可用音乐会话
type Payload = Option<(String, String, bool, String)>;

struct BoundSession {
    session: GlobalSystemMediaTransportControlsSession,
    media_token: EventRegistrationToken,
    playback_token: EventRegistrationToken,
}

/// 换绑时同步解绑旧会话上的事件。
/// 调用方必须保证此刻**没有持有** CURRENT_SESSION 锁（Remove* 是同步 COM 调用）。
impl Drop for BoundSession {
    fn drop(&mut self) {
        if let Err(e) = self.session.RemoveMediaPropertiesChanged(self.media_token.clone()) {
            eprintln!("[NSD][warn] RemoveMediaPropertiesChanged 失败: {}", e);
        }
        if let Err(e) = self.session.RemovePlaybackInfoChanged(self.playback_token.clone()) {
            eprintln!("[NSD][warn] RemovePlaybackInfoChanged 失败: {}", e);
        }
    }
}

static CURRENT_SESSION: Lazy<Mutex<Option<BoundSession>>> = Lazy::new(|| Mutex::new(None));
static APP_HANDLE: Lazy<Mutex<Option<tauri::AppHandle>>> = Lazy::new(|| Mutex::new(None));
static SESSIONS_TOKEN: Lazy<Mutex<Option<EventRegistrationToken>>> = Lazy::new(|| Mutex::new(None));
/// 发往 binder_thread 的命令通道
static CMD_TX: Lazy<Mutex<Option<mpsc::Sender<BinderCmd>>>> = Lazy::new(|| Mutex::new(None));
/// 上次 emit 的 music-info-changed 载荷，用于去重
/// （SMTC 常连发 MediaPropertiesChanged + PlaybackInfoChanged，避免前端重复拉封面/歌词）
static LAST_EMITTED: Lazy<Mutex<Option<Payload>>> = Lazy::new(|| Mutex::new(None));

/// binder_thread 可处理的命令
enum BinderCmd {
    /// 目标播放器变更 / 需要重新选择会话
    Rebind,
}

// TypedEventHandler 是泛型委托：TResult 必须与事件的 EventArgs 类型「精确一致」——
// windows 0.58 用 Param/CanInto 做约束，写 IInspectable 会因缺少 CanInto 而 E0277。
// 回调签名固定为 (&Option<TSender>, &Option<TResult>)，写值类型会 E0631。
type ManagerHandler = TypedEventHandler<
    GlobalSystemMediaTransportControlsSessionManager,
    SessionsChangedEventArgs,
>;
type MediaHandler =
    TypedEventHandler<GlobalSystemMediaTransportControlsSession, MediaPropertiesChangedEventArgs>;
type PlaybackHandler =
    TypedEventHandler<GlobalSystemMediaTransportControlsSession, PlaybackInfoChangedEventArgs>;

fn app_handle() -> Option<tauri::AppHandle> {
    APP_HANDLE.lock().ok()?.clone()
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
    handler: &MediaHandler,
) -> windows::core::Result<EventRegistrationToken> {
    session.MediaPropertiesChanged(handler)
}

/// 注册「播放信息变化」事件：播放 / 暂停状态变化时触发
fn register_playback_changed(
    session: &GlobalSystemMediaTransportControlsSession,
    handler: &PlaybackHandler,
) -> windows::core::Result<EventRegistrationToken> {
    session.PlaybackInfoChanged(handler)
}

/// setup 阶段调用：启动常驻 binder 线程，**立即返回，不阻塞 setup**。
///
/// 旧的同步实现会在主线程上执行 `RequestAsync().get()`（跨进程、可达数秒），
/// 把整个 setup 卡住。这里只 spawn 线程，实际初始化挪到 binder_thread。
pub fn init(app: tauri::AppHandle) {
    *APP_HANDLE.lock().unwrap_or_else(|e| e.into_inner()) = Some(app.clone());

    let (tx, rx) = mpsc::channel::<BinderCmd>();
    *CMD_TX.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);

    crate::thread_mgr::spawn_managed("smtc_binder", move |exit| binder_thread(app, rx, exit));
}

/// binder 常驻线程：MTA 套间 + SessionManager 预热 + 事件注册 + 命令循环
fn binder_thread(app: tauri::AppHandle, rx: mpsc::Receiver<BinderCmd>, exit: crate::thread_mgr::ExitFlag) {
    // COM 套间：MTA；ComGuard RAII 保证 CoUninitialize 严格配对
    let _com_guard = crate::win32_utils::ComGuard::new();

    // 在本线程（有 COM 套间）预热 SessionManager。
    // 启动极早期（SMTC/RPC 服务尚未就绪）RequestAsync 可能失败：
    // 失败绝不能放弃——这里若死掉，SessionsChanged 永远注册不上，
    // MANAGER_READY 恒为 false，兜底轮询也拿不到数据，事件链永久失效。
    // 以 1s 间隔无限重试，直到成功（可中断：退出信号到达时放弃）
    while !prime_session_manager() {
        if exit.sleep_interruptible(std::time::Duration::from_secs(1)) {
            return;
        }
    }

    if let Some(manager) = get_cached_session_manager() {
        // 形参类型必须显式标注：交给推导会得到「非 HRTB」的闭包，报 FnMut not general enough
        let sessions_callback = move |_manager: &Option<
            GlobalSystemMediaTransportControlsSessionManager,
        >,
              _args: &Option<SessionsChangedEventArgs>|
              -> windows::core::Result<()> {
            if let Some(app) = app_handle() {
                rebind(app, true);
            }
            Ok(())
        };
        let handler = ManagerHandler::new(sessions_callback);
        if let Ok(token) = register_sessions_changed(&manager, &handler) {
            // SESSIONS_TOKEN 不解绑：本线程与应用同生命周期，进程退出即释放；
            // 强行在退出路径解绑反而要额外同步，收益为零。
            *SESSIONS_TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(token);
        }
    }

    rebind(app.clone(), false);

    // 常驻命令循环。MTA 下事件由 RPC 线程池投递，这里阻塞 recv 不影响事件接收。
    // 线程不退出，保证 MTA 存活、回调可达。
    for cmd in rx {
        if exit.is_exiting() {
            break;
        }
        match cmd {
            BinderCmd::Rebind => rebind(app.clone(), true),
        }
    }
}

/// 前端 set_target_player 变更后调用：目标变了，重新选择要绑定的会话。
/// 只投递命令，不在调用线程（tokio / 主线程）做任何 WinRT 调用。
pub fn rebind_on_target_changed() {
    let tx = CMD_TX.lock().ok().and_then(|g| g.as_ref().cloned());
    if let Some(tx) = tx {
        let _ = tx.send(BinderCmd::Rebind);
    }
}

/// 供 music_controller 的会话选择使用：返回当前绑定的会话（若已绑定）。
/// 让进度条 / 封面 / 歌词与灵动岛指向同一目标，避免多会话时各自选到不同的会话。
pub(crate) fn current_bound_session() -> Option<GlobalSystemMediaTransportControlsSession> {
    CURRENT_SESSION
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|b| b.session.clone()))
}

/// 重新选择目标会话并绑定。`force_emit=true` 时即使会话未变也推送一次快照。
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
        let guard = CURRENT_SESSION.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(bound) = guard.as_ref() {
            if bound.session.as_raw() == new_session.as_raw() {
                drop(guard);
                if force_emit {
                    // 零阻塞：只推同步播放态 + 缓存标题
                    let info = extract_quick_info(&new_session);
                    emit_music_info(app, info);
                }
                return;
            }
        }
    }

    bind_to(app, new_session);
}

/// 绑定指定会话：先挂事件（锁外），再解绑旧会话（锁外），最后锁内置入并推送快照。
///
/// 推送分两步：
/// 1. 零阻塞 quick 快照 → UI 立刻响应（播放器刚启动时这一步就能把灵动岛弹出来）
/// 2. 全量元数据（含阻塞的 TryGetMediaPropertiesAsync）→ 补上真实歌名
fn bind_to(app: tauri::AppHandle, new_session: GlobalSystemMediaTransportControlsSession) {
    // —— 锁外：挂元数据变化事件（→ 全量推送） ——
    let app_for_media = app.clone();
    let media_callback = move |session: &Option<GlobalSystemMediaTransportControlsSession>,
                               _args: &Option<MediaPropertiesChangedEventArgs>|
          -> windows::core::Result<()> {
        if let Some(session) = session.as_ref() {
            handle_media_properties_changed(app_for_media.clone(), session.clone());
        }
        Ok(())
    };
    let media_handler = MediaHandler::new(media_callback);
    let media_token = match register_media_changed(&new_session, &media_handler) {
        Ok(t) => t,
        Err(_) => return, // 挂事件失败：不换绑，保留旧绑定继续工作
    };

    // —— 锁外：挂播放态变化事件（→ 轻量推送 + 自动切应用） ——
    let app_for_playback = app.clone();
    let playback_callback = move |session: &Option<GlobalSystemMediaTransportControlsSession>,
                                  _args: &Option<PlaybackInfoChangedEventArgs>|
          -> windows::core::Result<()> {
        if let Some(session) = session.as_ref() {
            handle_playback_info_changed(app_for_playback.clone(), session.clone());
        }
        Ok(())
    };
    let playback_handler = PlaybackHandler::new(playback_callback);
    let playback_token = match register_playback_changed(&new_session, &playback_handler) {
        Ok(t) => t,
        Err(_) => {
            // 回滚已挂上的 media 事件；失败只打 warn（会话即将弃用，无泄漏后果）
            if let Err(e) = new_session.RemoveMediaPropertiesChanged(media_token) {
                eprintln!("[NSD][warn] 回滚 RemoveMediaPropertiesChanged 失败: {}", e);
            }
            return;
        }
    };

    // —— 锁外：解绑旧会话（Drop 会同步调 Remove*） ——
    let old = take_bound_session();
    drop(old);

    // —— 锁内置入 ——
    *CURRENT_SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(BoundSession {
        session: new_session.clone(),
        media_token,
        playback_token,
    });

    // 第 1 步：零阻塞快照，UI 无需等待属性读取完成
    emit_music_info(app.clone(), extract_quick_info(&new_session));

    // 第 2 步：全量元数据。取属性失败 ≠ 无音乐（播放器刚启动时很常见），
    // 此时保留第 1 步的快照，绝不 emit None，否则前端会误清空 UI。
    if let Some(full) = extract_music_info(&new_session) {
        emit_music_info(app, Some(full));
    }
}

/// 锁内取出当前绑定（返回后调用方 drop 以在锁外执行 Remove* 解绑）
fn take_bound_session() -> Option<BoundSession> {
    CURRENT_SESSION.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// 元数据变化回调处理：推送全量音乐信息。
/// 此时属性刚刚变化，读取是「热」的；万一失败则退回零阻塞快照而不是清空 UI。
fn handle_media_properties_changed(
    app: tauri::AppHandle,
    session: GlobalSystemMediaTransportControlsSession,
) {
    // 确认该会话仍是当前绑定（防旧会话事件串扰）；锁内只取快照，比较在锁外做
    if !is_currently_bound(&session) {
        return;
    }
    let info = extract_music_info(&session).or_else(|| extract_quick_info(&session));
    emit_music_info(app, info);
}

/// 播放态变化回调处理：
/// 语义对齐现状（3s 轮询的自动切应用）：当前会话转暂停时，
/// 若存在其他正在播放的音乐会话则切换绑定。
///
/// 只做零阻塞调用，保证播放/暂停这类高频状态切换的响应是即时的。
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

    // 轻量事件：让前端立即处理播放态与窗口显隐（无网络请求，响应最快）
    crate::win32_utils::log_err(
        app.emit(
            "music-playback-changed",
            serde_json::json!({ "playing": playing }),
        ),
        "emit music-playback-changed",
    );
    // 同时推一次全量快照（零阻塞，标题来自缓存），保证歌名与播放态同步
    emit_music_info(app.clone(), extract_quick_info(&session));
}

/// 判断会话是否仍是当前绑定。
/// 主判据是 COM 指针身份；指针不一致时再用 AUMID 兜底——
/// 只比指针在个别情况下会静默吞掉事件，导致只能等 45s 兜底。
fn is_currently_bound(session: &GlobalSystemMediaTransportControlsSession) -> bool {
    let bound = current_bound_session();
    let Some(bound) = bound else {
        return false;
    };
    // 锁外比较：session_aumid 是 WinRT 调用，不能在锁内做
    if bound.as_raw() == session.as_raw() {
        return true;
    }
    let aumid = session_aumid(&bound);
    !aumid.is_empty() && aumid == session_aumid(session)
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

/// 推送全量音乐信息；info=None 表示当前无可用音乐会话（前端清空展示）。
/// 与上次载荷完全相同时跳过，避免前端重复触发封面/歌词网络请求。
fn emit_music_info(app: tauri::AppHandle, info: Payload) {
    {
        let Ok(mut guard) = LAST_EMITTED.lock() else {
            return;
        };
        if guard.as_ref() == Some(&info) {
            return;
        }
        *guard = Some(info.clone());
    }

    let payload = match info {
        Some((song, artist, playing, app_id)) => serde_json::json!({
            "song": song,
            "artist": artist,
            "playing": playing,
            "appId": app_id,
        }),
        None => serde_json::Value::Null,
    };
    crate::win32_utils::log_err(app.emit("music-info-changed", payload), "emit music-info-changed");
}