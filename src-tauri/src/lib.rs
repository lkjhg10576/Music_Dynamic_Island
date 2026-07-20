mod audio_spectrum;
mod music_controller;
mod notification;
mod pomodoro;
mod countdown;
mod health_reminder;
mod system_events;

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TrySendError};
use tauri::{Manager, Emitter, WebviewWindowBuilder, WebviewUrl};
use sysinfo::{Networks, System};
use std::time::{Duration, Instant};
use tauri_plugin_autostart::MacosLauncher;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};
use winapi::shared::windef::RECT;

// 全功能灵动岛智能双模动画锁
static ANIMATION_ID: AtomicU32 = AtomicU32::new(0);

// B1 省内存模式：关闭主窗口时彻底销毁 WebView（默认 false，保持原 hide 行为）
static DESTROY_ON_CLOSE: AtomicBool = AtomicBool::new(false);

// B1 硬件统计缓存：后台线程每 1s 刷新，command 零阻塞读取
static HW_CPU_X100: AtomicU32 = AtomicU32::new(0);
static HW_MEM_USED: AtomicU64 = AtomicU64::new(0);
static HW_MEM_TOTAL: AtomicU64 = AtomicU64::new(0);

// 将分散的坐标合并为一个结构体，并附带所有权 ID 防止误删
struct AnchorState {
    center_x: i32,
    origin_y: i32,
    left_x: i32,
    bottom_y: i32,
    active_id: u32,
}
static ANIMATION_ANCHOR: Mutex<Option<AnchorState>> = Mutex::new(None);

// B3: 常驻动画线程的 channel，只保留最新一条动画参数（capacity=1，新任务覆盖旧任务）
static ANIMATION_CHANNEL: Mutex<Option<std::sync::mpsc::SyncSender<AnimationCommand>>> = Mutex::new(None);

// 一次动画的完整参数
struct AnimationCommand {
    id: u32,
    hwnd_raw: isize,
    window_clone: tauri::WebviewWindow,
    scale_factor: f64,
    anchor_cx: i32,
    anchor_cy: i32,
    anchor_lx: i32,
    anchor_by: i32,
    start_width: f64,
    start_height: f64,
    target_width: f64,
    target_height: f64,
    is_pinned: bool,
}

/// 启动常驻动画线程（单次创建，loop 监听 channel）
fn start_animation_thread() {
    let (tx, rx): (std::sync::mpsc::SyncSender<AnimationCommand>, Receiver<AnimationCommand>) = std::sync::mpsc::sync_channel(1);
    let _ = ANIMATION_CHANNEL.lock().unwrap().insert(tx);

    std::thread::spawn(move || {
        let rx = Arc::new(rx);
        loop {
            // try_recv 非阻塞：只在有新任务时执行，空闲时零 CPU
            let cmd = rx.recv();
            match cmd {
                Ok(mut cmd) => {
                    let mut start_time = std::time::Instant::now();
                    let duration = std::time::Duration::from_millis(400);
                    let freq = 2.4;
                    let decay = 12.0;

                    while start_time.elapsed() < duration {
                        // 在循环中继续检查 channel：如有新动画命令立即中断当前动画
                        if let Ok(new_cmd) = rx.try_recv() {
                            // 有更新，放弃当前动画，用新参数重新开始（覆盖旧命令）
                            cmd = new_cmd;
                            start_time = std::time::Instant::now();
                            continue;
                        }

                        std::thread::sleep(std::time::Duration::from_millis(8));

                        let elapsed = start_time.elapsed().as_secs_f64();
                        let progress = elapsed / 0.4;
                        if progress >= 1.0 { break; }

                        let spring = 1.0 - (freq * elapsed * 2.0 * std::f64::consts::PI).cos() * (-decay * elapsed).exp();
                        let current_w = cmd.start_width + (cmd.target_width - cmd.start_width) * spring;
                        let current_h = cmd.start_height + (cmd.target_height - cmd.start_height) * spring;

                        let phys_window_w = (current_w * cmd.scale_factor).round() as i32;
                        let phys_window_h = (current_h * cmd.scale_factor).round() as i32;

                        let (final_x, final_y) = if cmd.is_pinned {
                            (cmd.anchor_lx, cmd.anchor_by - phys_window_h)
                        } else {
                            (cmd.anchor_cx - phys_window_w / 2, cmd.anchor_cy)
                        };

                        unsafe {
                            winapi::um::winuser::SetWindowPos(
                                cmd.hwnd_raw as _,
                                std::ptr::null_mut(),
                                final_x, final_y, phys_window_w, phys_window_h, 0x0014,
                            );
                        }
                    }

                    // 动画结束：设置最终目标尺寸，emit island-resize，清理锚点
                    let phys_target_w = (cmd.target_width * cmd.scale_factor).round() as i32;
                    let phys_target_h = (cmd.target_height * cmd.scale_factor).round() as i32;

                    let (final_x, final_y) = if cmd.is_pinned {
                        (cmd.anchor_lx, cmd.anchor_by - phys_target_h)
                    } else {
                        (cmd.anchor_cx - phys_target_w / 2, cmd.anchor_cy)
                    };

                    unsafe {
                        winapi::um::winuser::SetWindowPos(
                            cmd.hwnd_raw as _,
                            std::ptr::null_mut(),
                            final_x, final_y, phys_target_w, phys_target_h, 0x0014,
                        );
                    }
                    let _ = cmd.window_clone.emit("island-resize", vec![cmd.target_width, cmd.target_height]);

                    // 仅当当前动画仍是锚点持有者时才清理，防止误删新一轮动画的锁
                    if let Ok(mut guard) = ANIMATION_ANCHOR.lock() {
                        if let Some(anchor) = guard.as_ref() {
                            if anchor.active_id == cmd.id {
                                *guard = None;
                            }
                        }
                    }
                }
                Err(_) => {
                    // channel 关闭（应用退出），线程退出
                    break;
                }
            }
        }
    });
}

#[tauri::command]
fn force_window_topmost(app: tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            let fg_hwnd = winapi::um::winuser::GetForegroundWindow();
            if !fg_hwnd.is_null() {
                let mut class_name = [0u16; 256];
                let len = winapi::um::winuser::GetClassNameW(fg_hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
                let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
                
                if class_str == "#32768" { return; }

                let mut rect: RECT = std::mem::zeroed();
                winapi::um::winuser::GetWindowRect(fg_hwnd, &mut rect);

                let monitor = winapi::um::winuser::MonitorFromWindow(fg_hwnd, winapi::um::winuser::MONITOR_DEFAULTTONEAREST);
                let mut mi: winapi::um::winuser::MONITORINFO = std::mem::zeroed();
                mi.cbSize = std::mem::size_of::<winapi::um::winuser::MONITORINFO>() as u32;
                winapi::um::winuser::GetMonitorInfoW(monitor, &mut mi);

                if rect.left == mi.rcMonitor.left && rect.top == mi.rcMonitor.top && rect.right == mi.rcMonitor.right && rect.bottom == mi.rcMonitor.bottom {
                    if class_str != "Progman" && class_str != "WorkerW" {
                        return; 
                    }
                }
            }

            if let Some(win) = app.get_webview_window("widget") {
                if let Ok(hwnd) = win.hwnd() {
                    winapi::um::winuser::SetWindowPos(hwnd.0 as _, -1isize as _, 0, 0, 0, 0, 19);
                }
            }
        }
    }
}

// 新增：底层原子化窗口调整指令，彻底消除位移闪烁
#[tauri::command]
fn set_window_bounds(app: tauri::AppHandle, x: i32, y: i32, width: i32, height: i32) {
    #[cfg(target_os = "windows")]
    {
        if let Some(win) = app.get_webview_window("widget") {
            if let Ok(hwnd) = win.hwnd() {
                unsafe {
                    // 0x0014 = SWP_NOACTIVATE (0x0010) | SWP_NOZORDER (0x0004)
                    // 确保同时修改坐标和尺寸时，不抢占用户焦点，不打乱窗口层级
                    winapi::um::winuser::SetWindowPos(
                        hwnd.0 as _,
                        std::ptr::null_mut(),
                        x, y, width, height,
                        0x0014,
                    );
                }
            }
        }
    }
}

#[tauri::command]
async fn start_island_animation(
    window: tauri::WebviewWindow,
    start_width: f64,
    start_height: f64,
    target_width: f64,
    target_height: f64,
    is_pinned: bool,
) -> Result<(), String> {
    let id = ANIMATION_ID.fetch_add(1, Ordering::SeqCst) + 1;
    let scale_factor = window.scale_factor().unwrap_or(1.0);

    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = window.hwnd() {
            use winapi::um::winuser::GetWindowRect;
            use winapi::shared::windef::RECT;

            let mut rect: RECT = unsafe { std::mem::zeroed() };
            unsafe { GetWindowRect(hwnd.0 as _, &mut rect); }

            // 获取并克隆锚点值，与之前逻辑一致
            let (anchor_cx, anchor_cy, anchor_lx, anchor_by) = {
                let mut anchor_guard = ANIMATION_ANCHOR.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(anchor) = anchor_guard.as_mut() {
                    // 已经有动画锚点，说明正在连续打断动画，继承坐标并刷新所有权 ID
                    anchor.active_id = id;
                    (anchor.center_x, anchor.origin_y, anchor.left_x, anchor.bottom_y)
                } else {
                    // 首次触发，设定新的物理锚点
                    let cx = rect.left + (rect.right - rect.left) / 2;
                    let cy = rect.top;
                    let lx = rect.left;
                    let by = rect.bottom;
                    *anchor_guard = Some(AnchorState {
                        center_x: cx,
                        origin_y: cy,
                        left_x: lx,
                        bottom_y: by,
                        active_id: id,
                    });
                    (cx, cy, lx, by)
                }
            };

            let hwnd_raw = hwnd.0 as isize;

            // B3: 不再每次 spawn 新线程，改为向常驻线程发送命令
            let cmd = AnimationCommand {
                id,
                hwnd_raw,
                window_clone: window.clone(),
                scale_factor,
                anchor_cx,
                anchor_cy,
                anchor_lx,
                anchor_by,
                start_width,
                start_height,
                target_width,
                target_height,
                is_pinned,
            };

            let tx = ANIMATION_CHANNEL.lock().unwrap().clone();
            if let Some(tx) = tx {
                // try_send 非阻塞：channel 满（capacity=1）说明动画线程还在跑 400ms 动画
                // 直接丢新不阻塞 UI 线程；接收方 try_recv 在动画期间会持续检测
                // 真正"新命令覆盖旧命令"的语义在接收方（第 78 行 try_recv 处）实现
                if let Err(TrySendError::Disconnected(_)) = tx.try_send(cmd) {
                    return Err("动画线程已关闭".into());
                }
            }
        }
    }
    Ok(())
}

// 网速原子缓存：硬件监控线程写入，get_network_stats / get_hardware_stats 零阻塞读取
// 消除 AppState Networks 的双份刷新，降低长时间运行后的 CPU 爬坡
static HW_LAST_RX: AtomicU64 = AtomicU64::new(0);
static HW_LAST_TX: AtomicU64 = AtomicU64::new(0);
static HW_TOTAL_RX: AtomicU64 = AtomicU64::new(0);
static HW_TOTAL_TX: AtomicU64 = AtomicU64::new(0);
static HW_EMIT_ENABLED: AtomicBool = AtomicBool::new(true);

// B1 后台线程：每 1s 刷新 CPU / 内存统计，写入原子变量供 command 零阻塞读取
// 同时每 2s emit "monitor-stats" 事件，推送网速差值 + CPU/内存
fn start_hardware_monitor(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut sys = System::new();
        let mut networks = Networks::new_with_refreshed_list();
        let mut last_emit = std::time::Instant::now();
        let mut tick_count: u64 = 0; // 计数器：定期重建 Networks 防止内部 hash 膨胀
        // 跨推送周期累计网速样本，推送时取均值，使显示更连贯（避免单 1s 快照跳变）
        let mut pending_rx: u64 = 0;
        let mut pending_tx: u64 = 0;
        let mut sample_count: u64 = 0;
        // 首次刷新建立 CPU 基线
        sys.refresh_cpu_usage();
        std::thread::sleep(Duration::from_millis(200));
        loop {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            // sysinfo 0.30 中 global_cpu_info() 行为变化，改用 cpus() 遍历求平均
            let cpus = sys.cpus();
            let cpu_pct: f32 = if !cpus.is_empty() {
                cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
            } else {
                0.0
            };
            let used_mem = sys.used_memory();
            let total_mem = sys.total_memory();
            let mem_pct = if total_mem > 0 { (used_mem as f64 / total_mem as f64) * 100.0 } else { 0.0 };
            HW_CPU_X100.store((cpu_pct * 100.0) as u32, Ordering::Relaxed);
            HW_MEM_USED.store(used_mem, Ordering::Relaxed);
            HW_MEM_TOTAL.store(total_mem, Ordering::Relaxed);

            // 刷新网络统计并计算差值
            networks.refresh();
            // 每15分钟重建一次 Networks 对象，防止长期运行后虚拟网卡增删导致内部 hash 膨胀
            tick_count += 1;
            if tick_count % 900 == 0 {
                networks = Networks::new_with_refreshed_list();
                // 重置累计缓存避免重建后首次速度计算出现异常负值
                HW_LAST_RX.store(0, Ordering::Relaxed);
                HW_LAST_TX.store(0, Ordering::Relaxed);
            }
            let mut total_rx: u64 = 0;
            let mut total_tx: u64 = 0;
            for (_name, data) in networks.iter() {
                total_rx += data.total_received();
                total_tx += data.total_transmitted();
            }
            // 计算瞬时速度 (bytes/s)，避免除零
            let prev_rx = HW_LAST_RX.load(Ordering::Relaxed);
            let prev_tx = HW_LAST_TX.load(Ordering::Relaxed);
            let (rx_speed, tx_speed) = if prev_rx > 0 {
                let rx_diff = total_rx.saturating_sub(prev_rx);
                let tx_diff = total_tx.saturating_sub(prev_tx);
                ((rx_diff as f64).round() as u64, (tx_diff as f64).round() as u64)
            } else {
                (0, 0)
            };
            HW_LAST_RX.store(total_rx, Ordering::Relaxed);
            HW_LAST_TX.store(total_tx, Ordering::Relaxed);
            HW_TOTAL_RX.store(total_rx, Ordering::Relaxed);
            HW_TOTAL_TX.store(total_tx, Ordering::Relaxed);

            // 累计本推送周期内的网速样本（每 1s 一次）
            pending_rx += rx_speed;
            pending_tx += tx_speed;
            sample_count += 1;

            // 每 2s 推送 monitor-stats 事件（始终推送，控制台图表依赖此事件）
            if last_emit.elapsed() >= Duration::from_secs(2) {
                // 取推送周期内的平均速度（字节/秒），使网速显示更连贯、更具代表性
                let avg_rx = if sample_count > 0 { pending_rx / sample_count } else { 0 };
                let avg_tx = if sample_count > 0 { pending_tx / sample_count } else { 0 };
                let payload = serde_json::json!({
                    "upload_speed": avg_tx,
                    "download_speed": avg_rx,
                    "cpu_pct": cpu_pct,
                    "mem_pct": mem_pct,
                    "used_mem": used_mem,
                    "total_mem": total_mem,
                    "upload_bytes": total_tx,
                    "download_bytes": total_rx,
                });
                let _ = app_handle.emit("monitor-stats", payload);
                last_emit = std::time::Instant::now();
                pending_rx = 0;
                pending_tx = 0;
                sample_count = 0;
            }

            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

// 控制硬件监控事件推送开关（保留命令以维持 API 兼容，但当前始终推送）
#[tauri::command]
fn set_hardware_emit(enabled: bool) {
    HW_EMIT_ENABLED.store(enabled, Ordering::Relaxed);
}

#[tauri::command]
fn get_hardware_stats() -> (f32, u64, u64) {
    (
        HW_CPU_X100.load(Ordering::Relaxed) as f32 / 100.0,
        HW_MEM_USED.load(Ordering::Relaxed),
        HW_MEM_TOTAL.load(Ordering::Relaxed),
    )
}

#[tauri::command]
fn get_network_stats() -> (u64, u64) {
    // 直接从硬件监控线程的原子缓存读取，消除双份 Networks 刷新
    (
        HW_TOTAL_RX.load(Ordering::Relaxed),
        HW_TOTAL_TX.load(Ordering::Relaxed),
    )
}

#[tauri::command]
async fn get_network_latency() -> Result<u128, String> {
    let start = Instant::now();
    let connect_future = tokio::net::TcpStream::connect("223.5.5.5:53");
    match tokio::time::timeout(Duration::from_millis(1500), connect_future).await {
        Ok(Ok(_)) => Ok(start.elapsed().as_millis()),
        _ => Err("Timeout".to_string()),
    }
}

#[tauri::command]
fn is_widget_visible(app: tauri::AppHandle) -> bool {
    match app.get_webview_window("widget") {
        Some(win) => win.is_visible().unwrap_or(false),
        None => false,
    }
}

#[tauri::command]
fn set_destroy_on_close(enabled: bool) {
    DESTROY_ON_CLOSE.store(enabled, Ordering::Relaxed);
}

/// 为主窗口绑定关闭事件处理（兜底）：
/// 任何真实 CloseRequested 一律 prevent_close + hide，避免误关整个 App。
/// 注意：destroy() 在 CloseRequested 事件回调里（包括丢到线程）在 Tauri 2 / Windows
/// 上不可靠，会导致窗口失去响应。因此「彻底销毁」逻辑移到命令 close_main_window 里执行。
fn bind_main_window_close_event(window: &tauri::WebviewWindow) {
    let win_clone = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_clone.hide();
        }
    });
}

/// 关闭主窗口：由前端「关闭」按钮调用（而非依赖 CloseRequested 事件）。
/// - 省内存模式关闭（默认）：hide（隐藏到后台，原行为）
/// - 省内存模式开启：destroy（彻底销毁 WebView 释放内存）
/// 在命令上下文（而非事件回调）里执行 destroy 是 Tauri 2 的可靠写法。
/// 用独立线程执行，命令立即返回，避免阻塞调用方。
#[tauri::command]
fn close_main_window(app: tauri::AppHandle) {
    if DESTROY_ON_CLOSE.load(Ordering::Relaxed) {
        std::thread::spawn(move || {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.destroy();
            }
        });
    } else if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

/// 省内存模式下窗口已被销毁时，从托盘重建主窗口
fn recreate_main_window(app: &tauri::AppHandle) {
    let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("/".into()))
        .title("MDI 控制台")
        .inner_size(700.0, 550.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .center();

    match builder.build() {
        Ok(new_window) => {
            bind_main_window_close_event(&new_window);
            let _ = new_window.set_focus();
        }
        Err(e) => {
            eprintln!("[NSD] 重建主窗口失败: {}", e);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .invoke_handler(tauri::generate_handler![
            get_network_stats,
            is_widget_visible,
            set_destroy_on_close,
            close_main_window,
            set_hardware_emit,
            get_network_latency,
            notification::fetch_latest_notification,
            notification::launch_app_by_aumid,
            get_hardware_stats,
            force_window_topmost,
            set_window_bounds,
            start_island_animation,
            audio_spectrum::get_audio_spectrum,
            audio_spectrum::set_spectrum_active,
            music_controller::set_target_player,
            music_controller::fetch_netease_music_info,
            music_controller::control_system_media,
            music_controller::get_random_cover_url,
            music_controller::get_music_timeline,
            music_controller::seek_music,
            pomodoro::start_pomodoro,
            pomodoro::pause_pomodoro,
            pomodoro::resume_pomodoro,
            pomodoro::stop_pomodoro,
            pomodoro::get_pomodoro_state,
            countdown::start_countdown,
            countdown::pause_countdown,
            countdown::resume_countdown,
            countdown::stop_countdown,
            countdown::get_countdown_state,
            health_reminder::start_sitting_reminder,
            health_reminder::stop_sitting_reminder,
            health_reminder::dismiss_sitting_alert,
            health_reminder::skip_sitting_reminder,
            health_reminder::start_water_reminder,
            health_reminder::stop_water_reminder,
            health_reminder::dismiss_water_alert,
            health_reminder::skip_water_reminder,
            health_reminder::get_health_reminder_state,
        ])
        .setup(|app| {
            // B8: 注册 AppHandle 到 audio_spectrum 模块，支持 emit 频谱事件
            audio_spectrum::set_app_handle(Arc::new(app.handle().clone()));
            // B3: 启动常驻动画线程（单次创建）
            start_animation_thread();
            audio_spectrum::start_monitor();
            system_events::start_monitor(app.handle().clone());
            pomodoro::start_pomodoro_thread(app.handle().clone());
            countdown::start_countdown_thread(app.handle().clone());
            health_reminder::start_health_reminder_thread(app.handle().clone());
            start_hardware_monitor(app.handle().clone());

            // 全屏应用检测线程

            // 全屏应用检测线程：每 600ms 轮询，发射 fullscreen-changed 事件供前端做自动隐藏
            let app_handle_for_fs = app.handle().clone();
            std::thread::spawn(move || {
                unsafe { let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED); }
                
                let mut was_fullscreen = false;
                // P1：前台窗口句柄缓存 — 句柄未变时跳过完整的 Win32 API 检测，空闲开销趋近于零
                let mut last_fg_hwnd = std::ptr::null_mut();
                loop {
                    #[cfg(target_os = "windows")]
                    {
                        unsafe {
                            let mut is_fullscreen = false;
                            let fg_hwnd = winapi::um::winuser::GetForegroundWindow();
                            
                            // 快速路径：前台窗口句柄与上次相同，说明窗口未切换，跳过完整检测
                            if fg_hwnd == last_fg_hwnd {
                                // 窗口未切换时延长休眠间隔（2000ms），减少不必要的 CPU 唤醒
                                std::thread::sleep(std::time::Duration::from_millis(2000));
                                continue;
                            }
                            last_fg_hwnd = fg_hwnd;
                            
                            let shell_hwnd = winapi::um::winuser::GetShellWindow();
                            
                            if !fg_hwnd.is_null() 
                                && fg_hwnd != winapi::um::winuser::GetDesktopWindow() 
                                && fg_hwnd != shell_hwnd 
                            {
                                let mut shell_pid = 0;
                                if !shell_hwnd.is_null() {
                                    winapi::um::winuser::GetWindowThreadProcessId(shell_hwnd, &mut shell_pid);
                                }

                                let mut fg_pid = 0;
                                winapi::um::winuser::GetWindowThreadProcessId(fg_hwnd, &mut fg_pid);

                                if shell_pid != 0 && fg_pid == shell_pid {
                                    // 属于系统外壳组件，忽略
                                } else {
                                    let style = winapi::um::winuser::GetWindowLongPtrW(fg_hwnd, winapi::um::winuser::GWL_STYLE) as u32;
                                    let ex_style = winapi::um::winuser::GetWindowLongPtrW(fg_hwnd, winapi::um::winuser::GWL_EXSTYLE) as u32;
                                    
                                    if (style & winapi::um::winuser::WS_CHILD) == 0 && (ex_style & winapi::um::winuser::WS_EX_TRANSPARENT) == 0 {
                                        let mut class_name = [0u16; 256];
                                        let len = winapi::um::winuser::GetClassNameW(fg_hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
                                        let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
                                        
                                        let is_blacklisted = class_str.contains("Windows.UI.Core.CoreWindow") 
                                            || class_str.contains("Xaml_WindowedPopupClass")
                                            || class_str.contains("SearchApp")
                                            || class_str.contains("NotifyIconOverflowWindow");

                                        if !is_blacklisted {
                                            let mut rect: winapi::shared::windef::RECT = std::mem::zeroed();
                                            winapi::um::winuser::GetWindowRect(fg_hwnd, &mut rect);

                                            let monitor = winapi::um::winuser::MonitorFromWindow(fg_hwnd, winapi::um::winuser::MONITOR_DEFAULTTONEAREST);
                                            let mut mi: winapi::um::winuser::MONITORINFO = std::mem::zeroed();
                                            mi.cbSize = std::mem::size_of::<winapi::um::winuser::MONITORINFO>() as u32;
                                            winapi::um::winuser::GetMonitorInfoW(monitor, &mut mi);

                                            if rect.left <= mi.rcMonitor.left 
                                                && rect.top <= mi.rcMonitor.top 
                                                && rect.right >= mi.rcMonitor.right 
                                                && rect.bottom >= mi.rcMonitor.bottom 
                                            {
                                                is_fullscreen = true;
                                            }
                                        }
                                    }
                                }
                            }

                            if is_fullscreen != was_fullscreen {
                                let _ = app_handle_for_fs.emit("fullscreen-changed", is_fullscreen);
                                was_fullscreen = is_fullscreen;
                            }
                        }
                    }
                    // 正常检测路径：休眠 600ms 后继续轮询
                    std::thread::sleep(std::time::Duration::from_millis(600));
                }
            });

            let args: Vec<String> = std::env::args().collect();
            let is_autostart = args.iter().any(|arg| arg == "--autostart");

            if let Some(main_window) = app.get_webview_window("main") {
                if !is_autostart {
                    let _ = main_window.show();
                    let _ = main_window.set_focus();
                }
            }

            let quit_item = MenuItem::with_id(app, "quit", "强制退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("NetSpeed Dynamic Pro")
                .menu(&tray_menu)
                .on_menu_event(move |_app_handle, event| {
                    if event.id == "quit" { std::process::exit(0); }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        match app.get_webview_window("main") {
                            Some(main_window) => {
                                let _ = main_window.show();
                                let _ = main_window.unminimize();
                                let _ = main_window.set_focus();
                            }
                            None => {
                                // 省内存模式下窗口已被销毁，重建主窗口
                                recreate_main_window(app);
                            }
                        }
                    }
                })
                .build(app)?;

            if let Some(main_window) = app.get_webview_window("main") {
                bind_main_window_close_event(&main_window);
            }

            if let Some(widget_window) = app.get_webview_window("widget") {
                #[cfg(target_os = "windows")]
                {
                    use windows_sys::Win32::Graphics::Dwm::{
                        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWA_BORDER_COLOR, DWMWCP_DONOTROUND,
                    };
                    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowLongPtrW, GWL_STYLE, WS_CAPTION};
                    use windows_sys::Win32::Foundation::HWND;

                    if let Ok(hwnd) = widget_window.hwnd() {
                        let hwnd_raw = hwnd.0 as HWND;
                        unsafe {
                            let current_style = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd_raw, GWL_STYLE);
                            SetWindowLongPtrW(hwnd_raw, GWL_STYLE, current_style & !(WS_CAPTION as isize));

                            let border_color: u32 = 0xFFFFFFFE;
                            let _ = DwmSetWindowAttribute(hwnd_raw, DWMWA_BORDER_COLOR as u32, &border_color as *const _ as *const _, 4);

                            let corner_preference = DWMWCP_DONOTROUND;
                            let _ = DwmSetWindowAttribute(hwnd_raw, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &corner_preference as *const _ as *const _, 4);
                        }
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}