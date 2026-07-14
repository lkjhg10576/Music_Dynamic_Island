use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

// 静态状态迁移至此
static LAST_NOTIFICATION_ID: AtomicU32 = AtomicU32::new(0);
static IS_NOTIF_INIT: AtomicBool = AtomicBool::new(false);
static ACCESS_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(serde::Serialize, Clone)]
pub struct ToastData {
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub aumid: String,
}

#[tauri::command]
pub async fn fetch_latest_notification() -> Result<Option<ToastData>, String> {
    use windows::UI::Notifications::Management::UserNotificationListener;
    use windows::UI::Notifications::NotificationKinds;

    let listener = match UserNotificationListener::Current() {
        Ok(l) => l,
        Err(_) => return Ok(None),
    };

    // 仅在首次调用时请求权限，避免每次轮询都触发系统权限对话框
    if !ACCESS_REQUESTED.load(Ordering::SeqCst) {
        let _ = listener.RequestAccessAsync();
        ACCESS_REQUESTED.store(true, Ordering::SeqCst);
    }

    let notifications = match listener.GetNotificationsAsync(NotificationKinds::Toast) {
        Ok(op) => match op.get() {
            Ok(ns) => ns,
            Err(_) => return Ok(None),
        },
        Err(_) => return Ok(None),
    };

    let mut latest_notif = None;
    let mut max_id = 0u32;

    for notif in notifications {
        if let Ok(id) = notif.Id() {
            if id > max_id {
                max_id = id;
                latest_notif = Some(notif);
            }
        }
    }

    if max_id == 0 { return Ok(None); }

    let last_processed_id = LAST_NOTIFICATION_ID.load(Ordering::SeqCst);

    if !IS_NOTIF_INIT.load(Ordering::SeqCst) {
        LAST_NOTIFICATION_ID.store(max_id, Ordering::SeqCst);
        IS_NOTIF_INIT.store(true, Ordering::SeqCst);
        return Ok(None);
    }

    if max_id > last_processed_id {
        LAST_NOTIFICATION_ID.store(max_id, Ordering::SeqCst);

        if let Some(notif) = latest_notif {
            let app_name = notif.AppInfo()
                .and_then(|info| info.DisplayInfo())
                .and_then(|dinfo| dinfo.DisplayName())
                .map(|name| name.to_string())
                .unwrap_or_else(|_| "系统通知".to_string());

            let aumid = notif.AppInfo()
                .and_then(|info| info.AppUserModelId())
                .map(|id| id.to_string())
                .unwrap_or_default();

            if let Ok(toast_binding) = notif
                .Notification()
                .and_then(|n| n.Visual())
                .and_then(|v| v.GetBinding(&windows::core::HSTRING::from("ToastGeneric")))
            {
                if let Ok(text_elements) = toast_binding.GetTextElements() {
                    let mut text_list = Vec::new();
                    for elem in text_elements {
                        if let Ok(text) = elem.Text() {
                            text_list.push(text.to_string());
                        }
                    }

                    if !text_list.is_empty() {
                        let title = text_list.first().cloned().unwrap_or_default();
                        let body = if text_list.len() > 1 {
                            text_list[1..].join(" ")
                        } else {
                            String::new()
                        };

                        // 过滤微信通知
                        if title.contains("微信") || title.contains("WeChat") || body.contains("微信") || body.contains("WeChat") {
                            return Ok(None);
                        }

                        return Ok(Some(ToastData { app_name, title, body, aumid }));
                    }
                }
            }
        }
    }

    Ok(None)
}

// ===== F11 通知点击打开：根据 aumid 解析并启动来源应用 =====

/// 查询注册表 `Applications\<aumid>\shell\open\command` 的默认值
fn query_registry_command(root: windows_sys::Win32::System::Registry::HKEY, subkey: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{
        RegOpenKeyExW, RegQueryValueExW, RegCloseKey,
        KEY_READ, REG_SZ, REG_EXPAND_SZ, HKEY,
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