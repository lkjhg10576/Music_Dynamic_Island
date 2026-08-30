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

/// 返回 id > 游标 的全部通知（升序）；逐条过滤微信；游标推进到已处理的最大 id。
/// first_run=true 时仅推进游标（基线捕获），不推送，避免关/开通知后回放存量积压。
fn fetch_incremental(first_run: bool) -> Result<NotificationBatch, String> {
    use windows::UI::Notifications::Management::UserNotificationListener;
    use windows::UI::Notifications::NotificationKinds;

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

    let mut batch: Vec<ToastItem> = Vec::new();
    let mut max_id = 0u32;

    for notif in notifications {
        let id = match notif.Id() {
            Ok(id) => id,
            Err(_) => continue,
        };
        if id <= max_id {
            continue; // 保序
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

        if !first_run {
            batch.push(ToastItem {
                id,
                app_name,
                title,
                body,
                aumid,
            });
        }
    }

    // 推进游标到本次已处理的最大 id（无论是否被微信过滤）
    if max_id > 0 {
        LAST_NOTIFICATION_ID.store(max_id, Ordering::SeqCst);
    }
    Ok(NotificationBatch { items: batch })
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
        *NOTIF_CTRL.lock().unwrap() = Some(tx);
        NOTIF_RUNNING.store(true, Ordering::SeqCst);
        EVENT_CONFIRMED.store(false, Ordering::SeqCst);
        std::thread::spawn(move || start_listener_thread(rx, handler_tx, app));
    } else {
        if let Some(tx) = NOTIF_CTRL.lock().unwrap().take() {
            let _ = tx.send(Ctrl::Stop);
        }
        NOTIF_RUNNING.store(false, Ordering::SeqCst);
    }
    Ok(())
}

// ===== 常驻监听线程 + 状态机 + COM + 事件注册 + 轮询兜底 =====

fn start_listener_thread(rx: Receiver<Ctrl>, ctrl_tx: SyncSender<Ctrl>, app: tauri::AppHandle) {
    use windows::UI::Notifications::Management::UserNotificationListener;
    use windows::UI::Notifications::Management::UserNotificationListenerAccessStatus;

    // COM 套间：MTA（与 lib.rs 全屏线程一致，WinRT 调用前置条件）
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
    }

    let listener = match UserNotificationListener::Current() {
        Ok(l) => l,
        Err(_) => {
            let _ = app.emit("notification-status", AccessStatus::Unavailable);
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
        *EVENT_TOKEN.lock().unwrap() = Some(token);
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
        let _ = app.emit("notification-status", status);
    }

    let mut first_run = true;
    loop {
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
                let _ = app.emit("notification-event", batch);
            }
            _ => {}
        }
        first_run = false;
    }

    // 清理：注销事件
    if event_registered {
        if let Some(token) = EVENT_TOKEN.lock().unwrap().take() {
            let _ = listener.RemoveNotificationChanged(token);
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
    use winapi::um::shellapi::ShellExecuteW;

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
