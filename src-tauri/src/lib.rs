use std::sync::Mutex;
use tauri::{State, Manager};
use sysinfo::Networks;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

// 引入托盘相关组件
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};

struct AppState {
    networks: Mutex<Networks>,
}

#[tauri::command]
fn get_network_stats(state: State<'_, AppState>) -> (u64, u64) {
    let mut networks = state.networks.lock().unwrap();
    networks.refresh_list();

    let mut total_rx = 0;
    let mut total_tx = 0;

    for (_interface_name, data) in networks.iter() {
        total_rx += data.total_received();
        total_tx += data.total_transmitted();
    }

    (total_rx, total_tx)
}

#[tauri::command]
fn get_network_latency() -> Result<u128, String> {
    let addr: SocketAddr = "223.5.5.5:53".parse().unwrap();
    let timeout = Duration::from_millis(1500);

    let start = Instant::now();
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => {
            let elapsed = start.elapsed().as_millis();
            Ok(elapsed)
        }
        Err(_) => Err("Timeout or disconnected".to_string()),
    }
}

#[tauri::command]
fn is_widget_visible(app: tauri::AppHandle) -> bool {
    match app.get_webview_window("widget") {
        Some(win) => win.is_visible().unwrap_or(false),
        None => false,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let networks = Networks::new_with_refreshed_list();

    tauri::Builder::default()
        // 1. 【唯一实例保证】初始化单实例插件。如果重复启动，直接退出进程
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            networks: Mutex::new(networks),
        })
        .invoke_handler(tauri::generate_handler![
            get_network_stats, 
            is_widget_visible, 
            get_network_latency
        ])
        .setup(|app| {
            // 2. 【系统托盘右键菜单】仅创建一个 "强制退出" 按钮
            let quit_item = MenuItem::with_id(app, "quit", "强制退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            // 3. 【构建系统托盘】
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone()) // 使用默认图标
                .menu(&tray_menu)
                .on_menu_event(move |_app_handle, event| {
                    if event.id == "quit" {
                        // 点击“强制退出”时：强杀进程完全退出
                        std::process::exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // 当点击系统托盘图标时
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                        let app_handle = tray.app_handle();
                        // 寻找主控制台窗口
                        if let Some(main_window) = app_handle.get_webview_window("main") {
                            let _ = main_window.show();     // 显示窗口
                            let _ = main_window.unminimize(); // 取消最小化
                            let _ = main_window.set_focus();  // 聚焦窗口
                        }
                    }
                })
                .build(app)?;

            // 4. 【拦截控制台关闭事件】使其点击关闭时隐藏而不是真的退出
            if let Some(main_window) = app.get_webview_window("main") {
                let w_clone = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // 阻止窗口真正关闭
                        api.prevent_close();
                        // 使用克隆的窗口句柄来将其隐藏
                        let _ = w_clone.hide();
                    }
                });
            }

            // 5. 灵动岛(Widget)的原有 Windows 样式裁剪逻辑
            if let Some(widget_window) = app.get_webview_window("widget") {
                #[cfg(target_os = "windows")]
                {
                    use windows_sys::Win32::Graphics::Dwm::{
                        DwmSetWindowAttribute,
                        DWMWA_WINDOW_CORNER_PREFERENCE,
                        DWMWA_BORDER_COLOR,
                        DWMWCP_DONOTROUND,
                    };
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        SetWindowLongPtrW,
                        GWL_STYLE,
                        WS_CAPTION,
                    };
                    use windows_sys::Win32::Foundation::HWND;

                    if let Ok(hwnd) = widget_window.hwnd() {
                        let hwnd_raw = hwnd.0 as HWND;
                        unsafe {
                            let current_style = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd_raw, GWL_STYLE);
                            SetWindowLongPtrW(hwnd_raw, GWL_STYLE, current_style & !(WS_CAPTION as isize));

                            let border_color: u32 = 0xFFFFFFFE;
                            let _ = DwmSetWindowAttribute(
                                hwnd_raw,
                                DWMWA_BORDER_COLOR as u32,
                                &border_color as *const _ as *const _,
                                std::mem::size_of::<u32>() as u32,
                            );

                            let corner_preference = DWMWCP_DONOTROUND;
                            let _ = DwmSetWindowAttribute(
                                hwnd_raw,
                                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                                &corner_preference as *const _ as *const _,
                                std::mem::size_of::<i32>() as u32,
                            );
                        }
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}