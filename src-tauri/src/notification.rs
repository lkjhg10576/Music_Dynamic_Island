use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Mutex;

use tauri::Emitter;

// 增量游标（进程内持久，跨启停保留，避免重复推送）
static LAST_NOTIFICATION_ID: AtomicU32 = AtomicU32::new(0);

// 监听线程控制
enum Ctrl {
    Wake,
    Stop,
}
static NOTIF_CTRL: Mutex<Option<SyncSender<Ctrl>>> = Mutex::new(None);
static NOTIF_RUNNING: AtomicBool = AtomicBool::new(false);
static EVENT_CONFIRMED: AtomicBool = AtomicBool::new(false); // 事件是否真实触发过
static EVENT_TOKEN: Mutex<Option<windows::Foundation::EventRegistrationToken>> = Mutex::new(None);

// ===== 数据结构 =====

#[derive(serde::Serialize, Clone)]
pub struct ToastItem {
    pub id: u32,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub aumid: String,
    // 来源应用 logo（data URI base64，解码自 DisplayInfo::GetLogo）；
    // None 时前端回退本地白名单/默认图标
    pub icon: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct NotificationBatch {
    pub items: Vec<ToastItem>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AccessStatus {
    Ok,
    Denied,
    Unavailable,
}

// ===== 增量拉取 =====

/// 返回 id > 游标 的全部通知（按 id 升序处理）；逐条过滤微信；游标推进到已处理的最大 id。
/// first_run=true 时仅推进游标（基线捕获），不推送，避免关/开通知后回放存量积压。
fn fetch_incremental(first_run: bool) -> Result<NotificationBatch, String> {
    use windows::UI::Notifications::NotificationKinds;
    use windows::UI::Notifications::UserNotification;
    use windows::UI::Notifications::Management::UserNotificationListener;

    let listener = match UserNotificationListener::Current() {
        Ok(l) => l,
        Err(_) => return Err("listener_unavailable".into()),
    };

    let notifications = match listener.GetNotificationsAsync(NotificationKinds::Toast) {
        Ok(op) => match op.get() {
            Ok(ns) => ns,
            Err(_) => return Ok(NotificationBatch { items: vec![] }),
        },
        Err(_) => return Ok(NotificationBatch { items: vec![] }),
    };

    // 游标初值取自上次推进位置（此前每批从 0 重算、全局游标只写不读，增量过滤完全失效）
    let mut max_id = LAST_NOTIFICATION_ID.load(Ordering::SeqCst);

    // GetNotificationsAsync 的返回顺序无保证：先按 id 升序整理再游标过滤，
    // 否则乱序批次配合运行游标会漏掉中间 id 的新通知
    let mut ordered: Vec<(u32, UserNotification)> = Vec::new();
    for notif in notifications {
        if let Ok(id) = notif.Id() {
            ordered.push((id, notif));
        }
    }
    ordered.sort_by_key(|(id, _)| *id);

    let mut batch: Vec<ToastItem> = Vec::new();

    for (id, notif) in ordered {
        // 游标过滤：仅处理上次已推进位置之后的新通知
        if id <= max_id {
            continue;
        }
        max_id = id;

        let app_name = notif
            .AppInfo()
            .and_then(|i| i.DisplayInfo())
            .and_then(|d| d.DisplayName())
            .map(|n| n.to_string())
            .unwrap_or_else(|_| "系统通知".to_string());

        let aumid = notif
            .AppInfo()
            .and_then(|i| i.AppUserModelId())
            .map(|i| i.to_string())
            .unwrap_or_default();

        let (title, body) = match notif
            .Notification()
            .and_then(|n| n.Visual())
            .and_then(|v| v.GetBinding(&windows::core::HSTRING::from("ToastGeneric")))
            .and_then(|b| b.GetTextElements())
        {
            Ok(texts) => {
                let mut list = Vec::new();
                for t in texts {
                    if let Ok(s) = t.Text() {
                        list.push(s.to_string());
                    }
                }
                if list.is_empty() {
                    continue;
                }
                let title = list.first().cloned().unwrap_or_default();
                let body = if list.len() > 1 {
                    list[1..].join(" ")
                } else {
                    String::new()
                };
                (title, body)
            }
            Err(_) => continue,
        };

        // 微信逐条过滤：命中仅跳过本条，游标照常推进，不影响同批其他通知
        let is_wechat = title.contains("微信")
            || title.contains("WeChat")
            || body.contains("微信")
            || body.contains("WeChat");
        if is_wechat {
            continue;
        }

        // 番茄钟专注期免打扰：与微信过滤同构，仅跳过本条；
        // 循环末尾的 max_id 推进不受 continue 影响 → 通知永久丢弃，专注结束后不回放
        if crate::pomodoro::is_focus_phase_active()
            && crate::config_store::get_bool("nsd_pomodoro_dnd", false)
        {
            continue;
        }

        if !first_run {
            batch.push(ToastItem {
                id,
                app_name,
                title,
                body,
                aumid,
                icon: extract_logo_data_uri(&notif),
            });
        }
    }

    // 推进游标到本次已处理的最大 id（无论条目是否被过滤）；fetch_max 防并发回退
    if max_id > 0 {
        LAST_NOTIFICATION_ID.fetch_max(max_id, Ordering::SeqCst);
    }
    Ok(NotificationBatch { items: batch })
}

/// 读取来源应用 DisplayInfo 的 logo 并解码为 data URI（base64）。
/// 仅在监听线程（已建 COM MTA 套间）内调用；任一步失败返回 None，不影响通知本体推送。
fn extract_logo_data_uri(notif: &windows::UI::Notifications::UserNotification) -> Option<String> {
    use base64::Engine as _;
    use windows::Storage::Streams::DataReader;

    let logo = notif
        .AppInfo()
        .and_then(|i| i.DisplayInfo())
        .and_then(|d| d.GetLogo())
        .ok()?;

    let stream = logo.OpenReadAsync().ok()?.get().ok()?;
    let content_type = stream.ContentType().map(|t| t.to_string()).unwrap_or_default();
    let mime = if content_type.is_empty() { "image/png" } else { content_type.as_str() };

    // 通知 logo 均为小图（典型几 KB），512KB 上限拦截异常数据
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    let loaded = reader.LoadAsync(512 * 1024).ok()?.get().ok()? as usize;
    let mut bytes = vec![0u8; loaded];
    reader.ReadBytes(&mut bytes).ok()?;

    Some(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

// ===== 命令 =====

// (A) 权限检测（无感，不弹系统对话框）
#[tauri::command]
pub fn check_notification_access() -> Result<AccessStatus, String> {
    use windows::UI::Notifications::Management::UserNotificationListener;
    use windows::UI::Notifications::Management::UserNotificationListenerAccessStatus;

    let listener = match UserNotificationListener::Current() {
        Ok(l) => l,
        Err(_) => return Ok(AccessStatus::Unavailable),
    };
    match listener.GetAccessStatus() {
        Ok(s) => match s {
            UserNotificationListenerAccessStatus::Allowed => Ok(AccessStatus::Ok),
            UserNotificationListenerAccessStatus::Denied => Ok(AccessStatus::Denied),
            // Unspecified（非 MSIX Win32 常见）→ 视为 Denied，引导用户去设置页开启
            _ => Ok(AccessStatus::Denied),
        },
        Err(_) => Ok(AccessStatus::Unavailable),
    }
}

// (B) 打开系统设置（复用 shell_execute 辅助）
#[tauri::command]
pub fn open_notification_settings() -> Result<(), String> {
    shell_execute("ms-settings:privacy-notifications", "");
    Ok(())
}

// (C) 启停事件监听（前端开关 / 启动消息通知时调用）
#[tauri::command]
pub fn set_notification_listening(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        if NOTIF_RUNNING.load(Ordering::SeqCst) {
            return Ok(()); // 幂等
        }
        let (tx, rx) = sync_channel::<Ctrl>(16);
        let handler_tx = tx.clone();
        *NOTIF_CTRL.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
        NOTIF_RUNNING.store(true, Ordering::SeqCst);
        EVENT_CONFIRMED.store(false, Ordering::SeqCst);
        crate::thread_mgr::spawn_managed("notification_listener", move |exit| {
            start_listener_thread(rx, handler_tx, app, exit)
        });
    } else {
        if let Some(tx) = NOTIF_CTRL.lock().unwrap_or_else(|e| e.into_inner()).take() {
            if let Err(e) = tx.send(Ctrl::Stop) {
                eprintln!("[NSD][warn] 发送通知停止信号失败: {}", e);
            }
        }
        NOTIF_RUNNING.store(false, Ordering::SeqCst);
    }
    Ok(())
}

// ===== 常驻监听线程 + 状态机 + COM + 事件注册 + 轮询兜底 =====

fn start_listener_thread(
    rx: Receiver<Ctrl>,
    ctrl_tx: SyncSender<Ctrl>,
    app: tauri::AppHandle,
    exit: crate::thread_mgr::ExitFlag,
) {
    use windows::UI::Notifications::Management::UserNotificationListener;
    use windows::UI::Notifications::Management::UserNotificationListenerAccessStatus;

    // COM 套间：MTA（WinRT 调用前置条件）；ComGuard RAII 保证 CoUninitialize 严格配对
    let _com_guard = crate::win32_utils::ComGuard::new();

    let listener = match UserNotificationListener::Current() {
        Ok(l) => l,
        Err(_) => {
            crate::win32_utils::log_err(app.emit("notification-status", AccessStatus::Unavailable), "emit notification-status (unavailable)");
            NOTIF_RUNNING.store(false, Ordering::SeqCst);
            return;
        }
    };

    // 注册 NotificationChanged：handler 仅「置位 + 唤醒主循环」，不在事件线程做阻塞 WinRT 调用
    let handler = windows::Foundation::TypedEventHandler::<
        UserNotificationListener,
        windows::UI::Notifications::UserNotificationChangedEventArgs,
    >::new(move |_sender, _args| {
        EVENT_CONFIRMED.store(true, Ordering::SeqCst); // 事件真实触发 → 升级为低频安全网
        let _ = ctrl_tx.send(Ctrl::Wake);
        Ok(())
    });

    let mut event_registered = false;
    if let Ok(token) = listener.NotificationChanged(&handler) {
        *EVENT_TOKEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(token);
        event_registered = true;
    }
    // 注册失败（非 MSIX Win32 已知限制）→ event_registered=false，纯 5s 轮询

    // 启动即上报一次权限状态（前端据此显示警示条 / 灵岛 toast）
    if let Ok(s) = listener.GetAccessStatus() {
        let status = match s {
            UserNotificationListenerAccessStatus::Allowed => AccessStatus::Ok,
            UserNotificationListenerAccessStatus::Denied => AccessStatus::Denied,
            _ => AccessStatus::Denied,
        };
        crate::win32_utils::log_err(app.emit("notification-status", status), "emit notification-status");
    }

    let mut first_run = true;
    loop {
        if exit.is_exiting() {
            break;
        }
        // 状态机：事件已确认 → 60s 低频安全网；否则 5s 轮询（覆盖「事件永不触发」兜底）
        let interval = if EVENT_CONFIRMED.load(Ordering::SeqCst) {
            std::time::Duration::from_secs(60)
        } else {
            std::time::Duration::from_secs(5)
        };

        let woke = match rx.recv_timeout(interval) {
            Ok(Ctrl::Wake) => true,
            Ok(Ctrl::Stop) => break,
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => true, // 超时 → 兜底轮询
        };
        if !NOTIF_RUNNING.load(Ordering::SeqCst) {
            break;
        }
        if !woke {
            continue;
        }

        match fetch_incremental(first_run) {
            Ok(batch) if !batch.items.is_empty() => {
                crate::win32_utils::log_err(app.emit("notification-event", batch), "emit notification-event");
            }
            _ => {}
        }
        first_run = false;
    }

    // 清理：注销事件（防泄漏：EVENT_TOKEN 与 RemoveNotificationChanged 严格配对）
    if event_registered {
        if let Some(token) = EVENT_TOKEN.lock().unwrap_or_else(|e| e.into_inner()).take() {
            if let Err(e) = listener.RemoveNotificationChanged(token) {
                eprintln!("[NSD][warn] 注销通知事件失败: {}", e);
            }
        }
    }
    NOTIF_RUNNING.store(false, Ordering::SeqCst);
}

// ===== F11 通知点击打开：根据 aumid 解析并启动来源应用 =====

/// 查询注册表 `Applications\<aumid>\shell\open\command` 的默认值
fn query_registry_command(
    root: windows_sys::Win32::System::Registry::HKEY,
    subkey: &str,
) -> Option<String> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_EXPAND_SZ, REG_SZ,
    };

    unsafe {
        let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(root, subkey_wide.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return None;
        }

        let mut buf = [0u16; 1024];
        let mut len = (buf.len() * 2) as u32;
        let mut typ: u32 = 0;
        let res = RegQueryValueExW(
            hkey,
            std::ptr::null(),
            std::ptr::null(),
            &mut typ,
            buf.as_mut_ptr() as *mut u8,
            &mut len,
        );
        RegCloseKey(hkey);

        if res != 0 || (typ != REG_SZ && typ != REG_EXPAND_SZ) {
            return None;
        }

        let count = (len / 2) as usize;
        let s = String::from_utf16_lossy(&buf[..count]);
        Some(s.trim_end_matches('\0').to_string())
    }
}

/// 解析 command 字符串，分离出 exe 路径与参数（去掉 %1 文件占位符）
fn parse_command(cmd: &str) -> (String, String) {
    let cmd = cmd.trim();
    // 应用启动不需要传文件参数，移除 %1 占位符
    let cmd = cmd.replace("\"%1\"", "").replace("%1", "");
    let cmd = cmd.trim();

    if cmd.starts_with('"') {
        if let Some(end) = cmd[1..].find('"') {
            let exe = cmd[1..=end].to_string();
            let args = cmd[end + 2..].trim().to_string();
            return (exe, args);
        }
    }

    if let Some(pos) = cmd.find(' ') {
        return (cmd[..pos].to_string(), cmd[pos + 1..].trim().to_string());
    }

    (cmd.to_string(), String::new())
}

/// 调用 ShellExecuteW 启动可执行文件
fn shell_execute(exe: &str, args: &str) {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    let exe_wide: Vec<u16> = exe.encode_utf16().chain(std::iter::once(0)).collect();
    let args_wide: Vec<u16> = if args.is_empty() {
        Vec::new()
    } else {
        args.encode_utf16().chain(std::iter::once(0)).collect()
    };
    let op_wide: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op_wide.as_ptr(),
            exe_wide.as_ptr(),
            if args_wide.is_empty() {
                std::ptr::null()
            } else {
                args_wide.as_ptr()
            },
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        );
    }
}

/// 点击灵动岛通知后，根据 aumid 启动来源应用
#[tauri::command]
pub fn launch_app_by_aumid(aumid: String) -> Result<(), String> {
    use windows_sys::Win32::System::Registry::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER};

    if aumid.is_empty() {
        return Err("通知无来源应用标识".to_string());
    }

    let subkey = format!("Applications\\{}\\shell\\open\\command", aumid);

    // 依次尝试 HKCR 和 HKCU 两个根键
    for &root in &[HKEY_CLASSES_ROOT, HKEY_CURRENT_USER] {
        if let Some(cmd) = query_registry_command(root, &subkey) {
            let (exe, args) = parse_command(&cmd);
            if !exe.is_empty() {
                shell_execute(&exe, &args);
                return Ok(());
            }
        }
    }

    // 兜底：直接用 aumid 尝试 ShellExecute（对部分注册了协议的应用有效）
    shell_execute(&aumid, "");
    Ok(())
}
