mod audio_spectrum;
mod music_controller;
mod notification;
mod pomodoro;
mod pomodoro_stats;
mod countdown;
mod calendar;
mod health_reminder;
mod system_events;
mod print_queue;
mod storage;
mod traffic_stats;
mod config_store;
mod session_binder;
mod lyrics_cache;
mod print_utils;
mod thread_mgr;
mod win32_utils;
mod clipboard;

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TrySendError};
use tauri::{Manager, Emitter, WebviewWindowBuilder, WebviewUrl};
use sysinfo::{Disks, Networks, System};
use std::time::Duration;
use tauri_plugin_autostart::MacosLauncher;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};

// 全功能灵动岛智能双模动画锁
static ANIMATION_ID: AtomicU32 = AtomicU32::new(0);

// B1 省内存模式：关闭主窗口时彻底销毁 WebView（默认 false，保持原 hide 行为）
static DESTROY_ON_CLOSE: AtomicBool = AtomicBool::new(false);

// 网络延迟探测间隔（秒），默认 30，允许范围 1~60；由前端设置页实时下发
static NETWORK_LATENCY_INTERVAL_SECS: AtomicU64 = AtomicU64::new(30);

/// 供 system_events::NetworkMonitor 读取当前延迟探测间隔（已钳制到 1~60）
pub(crate) fn network_latency_interval_secs() -> u64 {
    NETWORK_LATENCY_INTERVAL_SECS
        .load(Ordering::Relaxed)
        .clamp(1, 60)
}

#[tauri::command]
fn set_network_latency_interval(secs: u64) {
    let clamped = secs.clamp(1, 60);
    NETWORK_LATENCY_INTERVAL_SECS.store(clamped, Ordering::Relaxed);
}

/// 测量到 223.5.5.5:53 的 TCP 连接延迟（毫秒），超时 1500ms 记 Err。
/// 由 system_events::network_monitor_loop 用 block_on 调用；不再作为前端命令暴露，
/// 故不注册进 generate_handler，仅 crate 内可见。
#[allow(dead_code)]
pub(crate) async fn get_network_latency() -> Result<u128, String> {
    let start = std::time::Instant::now();
    let connect_future = tokio::net::TcpStream::connect("223.5.5.5:53");
    match tokio::time::timeout(Duration::from_millis(1500), connect_future).await {
        Ok(Ok(_)) => Ok(start.elapsed().as_millis()),
        _ => Err("Timeout".to_string()),
    }
}

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
    let _ = ANIMATION_CHANNEL
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(tx);

    thread_mgr::spawn_managed("island_animation", move |exit| {
        let rx = Arc::new(rx);
        loop {
            if exit.is_exiting() { break; }
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
                        if exit.is_exiting() { return; }

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

                        win32_utils::set_window_pos_no_activate(cmd.hwnd_raw, final_x, final_y, phys_window_w, phys_window_h);
                    }

                    // 动画结束：设置最终目标尺寸，emit island-resize，清理锚点
                    let phys_target_w = (cmd.target_width * cmd.scale_factor).round() as i32;
                    let phys_target_h = (cmd.target_height * cmd.scale_factor).round() as i32;

                    let (final_x, final_y) = if cmd.is_pinned {
                        (cmd.anchor_lx, cmd.anchor_by - phys_target_h)
                    } else {
                        (cmd.anchor_cx - phys_target_w / 2, cmd.anchor_cy)
                    };

                    win32_utils::set_window_pos_no_activate(cmd.hwnd_raw, final_x, final_y, phys_target_w, phys_target_h);
                    win32_utils::log_err(
                        cmd.window_clone.emit("island-resize", vec![cmd.target_width, cmd.target_height]),
                        "emit island-resize",
                    );

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
    // 判断逻辑（菜单/外壳/全屏跳过）收口到 win32_utils，此处只做置顶动作
    if win32_utils::should_skip_topmost() {
        return;
    }
    if let Some(win) = app.get_webview_window("widget") {
        if let Ok(hwnd) = win.hwnd() {
            win32_utils::set_window_pos_topmost(hwnd.0 as isize);
        }
    }
}

// 新增：底层原子化窗口调整指令，彻底消除位移闪烁
#[tauri::command]
fn set_window_bounds(app: tauri::AppHandle, x: i32, y: i32, width: i32, height: i32) {
    if let Some(win) = app.get_webview_window("widget") {
        if let Ok(hwnd) = win.hwnd() {
            // SWP_NOACTIVATE | SWP_NOZORDER：不抢占用户焦点，不打乱窗口层级
            win32_utils::set_window_pos_no_activate(hwnd.0 as isize, x, y, width, height);
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
            // 获取失败时用零值矩形兜底（windows-sys RECT 未实现 Default，显式构造）
            let rect = win32_utils::get_window_rect(hwnd.0 as isize).unwrap_or(
                windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 },
            );

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

            let tx = ANIMATION_CHANNEL
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
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

// 网速原子缓存：硬件监控线程写入，monitor-stats 事件推送使用
// 消除 AppState Networks 的双份刷新，降低长时间运行后的 CPU 爬坡
static HW_LAST_RX: AtomicU64 = AtomicU64::new(0);
static HW_LAST_TX: AtomicU64 = AtomicU64::new(0);
static HW_TOTAL_RX: AtomicU64 = AtomicU64::new(0);
static HW_TOTAL_TX: AtomicU64 = AtomicU64::new(0);
static HW_LAST_SAMPLE_AT: AtomicU64 = AtomicU64::new(0);

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// B1 后台线程：默认每 1s 刷新 CPU / 内存统计，写入原子变量供 command 零阻塞读取；
// 同时每 2s emit "monitor-stats" 事件，推送网速差值 + CPU/内存。
// §6.1 动态间隔：仅当硬件实时活动开启或主窗口可见时全量轮询；
// 否则进入 30s 一次的流量统计保活模式（保证省内存/静默自启下统计不中断，同时大幅降低 CPU 消耗）。
fn is_hardware_realtime_needed(app: &tauri::AppHandle) -> bool {
    // 岛的网速/硬件依赖 monitor-stats 推送，保活路径不 emit；
    // widget 可见时必须全量轮询，否则省内存模式销毁 main 后灵动岛网速会冻结。
    if app
        .get_webview_window("widget")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false)
    {
        return true;
    }
    if config_store::get_bool("nsd_hw_enabled", false) {
        return true;
    }
    app.get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false)
}

fn start_hardware_monitor(app_handle: tauri::AppHandle) {
    thread_mgr::spawn_managed("hardware_monitor", move |exit| {
        let mut sys = System::new();
        let mut networks = Networks::new_with_refreshed_list();
        // E2：磁盘占用率数据源（全量轮询分支内 refresh，保活分支不推送无需刷新）
        let mut disks = Disks::new_with_refreshed_list();
        let mut last_emit = std::time::Instant::now();
        let mut tick_count: u64 = 0; // 计数器：定期重建 Networks 防止内部 hash 膨胀
        // 跨推送周期累计网速样本，推送时取均值，使显示更连贯（避免单 1s 快照跳变）
        let mut pending_rx: u64 = 0;
        let mut pending_tx: u64 = 0;
        let mut sample_count: u64 = 0;
        // 首次刷新建立 CPU 基线
        sys.refresh_cpu_usage();
        std::thread::sleep(Duration::from_millis(200));
        // 流量统计：载入历史数据后由本线程按天累计
        traffic_stats::init(&app_handle);
        loop {
            // 实时硬件监控未开启且主窗口不可见时，只做低频流量统计保活
            if !is_hardware_realtime_needed(&app_handle) {
                networks.refresh();
                tick_count += 1;
                // 每15分钟重建一次 Networks 对象（30s × 30），防止长期运行后虚拟网卡增删导致内部 hash 膨胀
                if tick_count % 30 == 0 {
                    networks = Networks::new_with_refreshed_list();
                    HW_LAST_RX.store(0, Ordering::Relaxed);
                    HW_LAST_TX.store(0, Ordering::Relaxed);
                }
                let mut total_rx: u64 = 0;
                let mut total_tx: u64 = 0;
                for (_name, data) in networks.iter() {
                    total_rx += data.total_received();
                    total_tx += data.total_transmitted();
                }
                let now_secs = unix_now_secs();
                let prev_rx = HW_LAST_RX.load(Ordering::Relaxed);
                let prev_tx = HW_LAST_TX.load(Ordering::Relaxed);
                let (rx_diff, tx_diff) = if prev_rx > 0 {
                    (
                        total_rx.saturating_sub(prev_rx),
                        total_tx.saturating_sub(prev_tx),
                    )
                } else {
                    (0, 0)
                };
                HW_LAST_RX.store(total_rx, Ordering::Relaxed);
                HW_LAST_TX.store(total_tx, Ordering::Relaxed);
                HW_TOTAL_RX.store(total_rx, Ordering::Relaxed);
                HW_TOTAL_TX.store(total_tx, Ordering::Relaxed);
                HW_LAST_SAMPLE_AT.store(now_secs, Ordering::Relaxed);
                traffic_stats::accumulate(tx_diff, rx_diff);
                traffic_stats::maybe_persist(&app_handle);
                // 清空实时推送缓存，避免恢复全量轮询时把空闲前的旧样本混入新均值
                pending_rx = 0;
                pending_tx = 0;
                sample_count = 0;
                if exit.sleep_interruptible(Duration::from_secs(30)) {
                    return;
                }
                continue;
            }

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

            // E2：电池电量 + 磁盘占用率（monitor-stats 扩展数据源）
            // 电池复用 win32_utils::power_status 共享封装（与 system_events 电量提醒同源）；
            // 无电池时 BatteryLifePercent 为 255，原样上报，由前端做"置灰不可选"兜底。
            let battery_pct: u8 = win32_utils::power_status()
                .map(|(_, pct)| pct)
                .unwrap_or(255);
            // 磁盘占用率：全盘汇总 已用/总容量（排除 available > total 的异常项防溢出）
            disks.refresh();
            let mut disk_total: u64 = 0;
            let mut disk_used: u64 = 0;
            for disk in disks.list() {
                let (total_space, available) = (disk.total_space(), disk.available_space());
                if total_space > 0 && available <= total_space {
                    disk_total += total_space;
                    disk_used += total_space - available;
                }
            }
            let disk_pct: f64 = if disk_total > 0 {
                (disk_used as f64 / disk_total as f64) * 100.0
            } else {
                0.0
            };

            // 刷新网络统计并计算差值
            networks.refresh();
            // 每15分钟重建一次 Networks 对象，防止长期运行后虚拟网卡增删导致内部 hash 膨胀
            tick_count += 1;
            if tick_count % 900 == 0 {
                networks = Networks::new_with_refreshed_list();
                // 同步刷新磁盘列表（U 盘等挂载点变化），容量数值仍每秒 refresh
                disks.refresh_list();
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
            // 计算瞬时速度 (bytes/s)，避免除零；跨采样间隔用实际秒数归一化，
            // 防止从 30s 保活切回 1s 全量轮询的第一帧产生约 30 倍的虚假峰值。
            let now_secs = unix_now_secs();
            let prev_sample_at = HW_LAST_SAMPLE_AT.load(Ordering::Relaxed);
            let elapsed_secs = now_secs.saturating_sub(prev_sample_at).max(1);
            let prev_rx = HW_LAST_RX.load(Ordering::Relaxed);
            let prev_tx = HW_LAST_TX.load(Ordering::Relaxed);
            let (rx_diff, tx_diff) = if prev_rx > 0 {
                (
                    total_rx.saturating_sub(prev_rx),
                    total_tx.saturating_sub(prev_tx),
                )
            } else {
                (0, 0)
            };
            let rx_speed = ((rx_diff as f64) / elapsed_secs as f64).round() as u64;
            let tx_speed = ((tx_diff as f64) / elapsed_secs as f64).round() as u64;
            HW_LAST_RX.store(total_rx, Ordering::Relaxed);
            HW_LAST_TX.store(total_tx, Ordering::Relaxed);
            HW_TOTAL_RX.store(total_rx, Ordering::Relaxed);
            HW_TOTAL_TX.store(total_tx, Ordering::Relaxed);
            HW_LAST_SAMPLE_AT.store(now_secs, Ordering::Relaxed);

            // 累计本推送周期内的网速样本（每 1s 一次）
            pending_rx += rx_speed;
            pending_tx += tx_speed;
            sample_count += 1;

            // 流量统计：按天累计实际字节增量（diff），节流落盘（主窗口关闭后仍持续统计）
            traffic_stats::accumulate(tx_diff, rx_diff);
            traffic_stats::maybe_persist(&app_handle);

            // 每 2s 推送 monitor-stats 事件（实时硬件监控开启时推送，控制台图表依赖此事件）
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
                    "battery_pct": battery_pct,
                    "disk_pct": disk_pct,
                    "upload_bytes": total_tx,
                    "download_bytes": total_rx,
                });
                win32_utils::log_err(app_handle.emit("monitor-stats", payload), "emit monitor-stats");
                last_emit = std::time::Instant::now();
                pending_rx = 0;
                pending_tx = 0;
                sample_count = 0;
            }

            // 可中断休眠：收到退出信号立即返回，避免最长 1s 的 join 延迟
            if exit.sleep_interruptible(Duration::from_secs(1)) {
                return;
            }
        }
    });
}

#[tauri::command]
fn is_widget_visible(app: tauri::AppHandle) -> bool {
    match app.get_webview_window("widget") {
        Some(win) => win.is_visible().unwrap_or(false),
        None => false,
    }
}

#[tauri::command]
fn get_traffic_stats(app: tauri::AppHandle) -> serde_json::Value {
    traffic_stats::ensure_loaded(&app);
    serde_json::to_value(traffic_stats::snapshot()).unwrap_or(serde_json::Value::Null)
}

/// 番茄钟专注统计快照：当日 + 历史总计（总计不自动清零）
#[tauri::command]
fn get_pomodoro_stats(app: tauri::AppHandle) -> serde_json::Value {
    pomodoro_stats::ensure_loaded(&app);
    let (date, today, total) = pomodoro_stats::snapshot();
    serde_json::json!({
        "date": date,
        "today": today,
        "total": total,
    })
}

/// 一次性迁移：前端把 localStorage 里的历史流量数据合并到后端落盘
#[tauri::command]
fn merge_legacy_traffic(
    app: tauri::AppHandle,
    legacy: std::collections::HashMap<String, traffic_stats::DayTraffic>,
) -> Result<(), String> {
    traffic_stats::merge_legacy(&app, legacy)
}

// ===== 设置单一数据源（config.json + config-changed 广播） =====

#[tauri::command]
fn config_get(app: tauri::AppHandle, key: String) -> Option<serde_json::Value> {
    config_store::ensure_loaded(&app);
    config_store::get(&key)
}

#[tauri::command]
fn config_get_all(app: tauri::AppHandle) -> std::collections::HashMap<String, serde_json::Value> {
    config_store::ensure_loaded(&app);
    config_store::get_all()
}

#[tauri::command]
fn config_set(app: tauri::AppHandle, key: String, value: serde_json::Value) -> Result<(), String> {
    config_store::set(&app, key, value)
}

#[tauri::command]
fn config_remove(app: tauri::AppHandle, key: String) -> Result<(), String> {
    config_store::remove(&app, key)
}

#[tauri::command]
fn config_migrate_legacy(
    app: tauri::AppHandle,
    legacy: std::collections::HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    config_store::merge_legacy(&app, legacy)
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
            win32_utils::log_err(win_clone.hide(), "hide main window on close");
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
                win32_utils::log_err(win.destroy(), "destroy main window");
            }
        });
    } else if let Some(win) = app.get_webview_window("main") {
        win32_utils::log_err(win.hide(), "hide main window");
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
            win32_utils::log_err(new_window.set_focus(), "focus recreated main window");
        }
        Err(e) => {
            eprintln!("[NSD] 重建主窗口失败: {}", e);
        }
    }
}

/// 导出 CSV 文件：直接保存到系统「下载」目录，无需弹出任何对话框。
/// 若同名文件已存在，则自动追加 (1)/(2)… 序号避免覆盖。
/// 返回 Ok(保存的完整路径) 供前端提示展示，Err 表示获取目录或写入失败。
#[tauri::command]
fn save_csv_file(app: tauri::AppHandle, default_name: String, content: String) -> Result<String, String> {
    // 获取系统下载目录（自动跟随用户在系统中设置的实际下载位置）
    let dir = app
        .path()
        .download_dir()
        .map_err(|e| format!("无法获取下载目录: {}", e))?;

    // 拆分文件名主干与扩展名，用于同名冲突时插入序号
    let (stem, ext) = match default_name.rfind('.') {
        Some(pos) => (&default_name[..pos], &default_name[pos..]),
        None => (default_name.as_str(), ""),
    };

    // 计算最终不冲突的路径
    let mut target = dir.join(&default_name);
    let mut idx = 1;
    while target.exists() {
        target = dir.join(format!("{}({}){}", stem, idx, ext));
        idx += 1;
    }

    // 写入文件（带 UTF-8 BOM，便于 Excel 正确识别中文）
    let bom = "\u{FEFF}";
    let full_content = format!("{}{}", bom, content);
    std::fs::write(&target, full_content.as_bytes())
        .map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(target.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .invoke_handler(tauri::generate_handler![
            is_widget_visible,
            set_destroy_on_close,
            close_main_window,
            notification::check_notification_access,
            notification::open_notification_settings,
            notification::set_notification_listening,
            notification::launch_app_by_aumid,
            force_window_topmost,
            set_window_bounds,
            start_island_animation,
            audio_spectrum::set_spectrum_active,
            audio_spectrum::start_audio_spectrum,
            audio_spectrum::stop_audio_spectrum,
            music_controller::set_target_player,
            music_controller::fetch_netease_music_info,
            music_controller::control_system_media,
            music_controller::get_random_cover_url,
            music_controller::get_smtc_cover,
            music_controller::get_music_timeline,
            music_controller::seek_music,
            music_controller::fetch_netease_lyrics,
            music_controller::import_current_track,
            music_controller::search_lyrics_candidates,
            music_controller::get_lyrics_by_candidate,
            lyrics_cache::save_lyrics_binding,
            lyrics_cache::list_lyrics_cache,
            lyrics_cache::get_lyrics_by_key,
            lyrics_cache::delete_lyrics_entry,
            pomodoro::start_pomodoro,
            pomodoro::pause_pomodoro,
            pomodoro::resume_pomodoro,
            pomodoro::stop_pomodoro,
            pomodoro::get_pomodoro_state,
            countdown::start_countdown,
            countdown::pause_countdown,
            countdown::resume_countdown,
            countdown::stop_countdown,
            countdown::stop_countdown_alarm,
            countdown::get_countdown_state,
            calendar::calendar_add_manual_event,
            calendar::calendar_remove_manual_event,
            calendar::calendar_get_state,
            health_reminder::start_sitting_reminder,
            health_reminder::stop_sitting_reminder,
            health_reminder::dismiss_sitting_alert,
            health_reminder::skip_sitting_reminder,
            health_reminder::start_water_reminder,
            health_reminder::stop_water_reminder,
            health_reminder::dismiss_water_alert,
            health_reminder::skip_water_reminder,
            health_reminder::get_health_reminder_state,
            system_events::set_system_event_filter,
            set_network_latency_interval,
            save_csv_file,
            get_traffic_stats,
            merge_legacy_traffic,
            get_pomodoro_stats,
            config_get,
            config_get_all,
            config_set,
            config_remove,
            config_migrate_legacy,
            print_queue::set_printer_monitor_enabled,
            print_queue::get_printer_state,
            clipboard::clipboard_set_enabled,
            clipboard::clipboard_get_history,
            clipboard::clipboard_copy_item,
            clipboard::clipboard_toggle_pin,
            clipboard::clipboard_delete_item,
            clipboard::clipboard_clear,
        ])
        .setup(|app| {
            // 设置单一数据源：载入 config.json + 落盘线程
            config_store::init(app.handle());
            // 流量统计：尽早从磁盘载入历史数据，避免前端首屏查询时读到空快照
            traffic_stats::init(app.handle());
            // 番茄钟专注统计：同上，尽早载入历史按天数据
            pomodoro_stats::init(app.handle());
            // B8: 注册 AppHandle 到 audio_spectrum 模块，支持 emit 频谱事件
            audio_spectrum::set_app_handle(Arc::new(app.handle().clone()));
            // B3: 启动常驻动画线程（单次创建）
            start_animation_thread();
            // 音频频谱改为按需启停：由前端 set_spectrum_active / start|stop_audio_spectrum 控制
            system_events::start_monitor(app.handle().clone());
            // 独立线程：NLM 连通性 + 延迟探测（不与 start_monitor 混用 COM 套间）
            system_events::start_network_monitor(app.handle().clone());
            pomodoro::start_pomodoro_thread(app.handle().clone());
            countdown::start_countdown_thread(app.handle().clone());
            // 日程同步：系统日历（WinRT 只读）+ 手动提醒，每 5 分钟重查 / 30 秒推送
            calendar::start_calendar_thread(app.handle().clone());
            health_reminder::start_health_reminder_thread(app.handle().clone());
            print_queue::start_print_queue_monitor(app.handle().clone());
            start_hardware_monitor(app.handle().clone());
            // SMTC 会话绑定管理器：事件驱动音乐信息推送（替代前端 3s 轮询）
            session_binder::init(app.handle().clone());

            // 全屏应用检测线程：每 2s 轮询，发射 fullscreen-changed 事件供前端做自动隐藏
            // Win32 判定逻辑收口到 win32_utils::is_foreground_fullscreen
            let app_handle_for_fs = app.handle().clone();
            thread_mgr::spawn_managed("fullscreen_detector", move |exit| {
                // 本线程 emit 事件用到 COM 相关类型，按 MTA 初始化，ComGuard RAII 配对释放
                let _com = win32_utils::ComGuard::new();

                let mut was_fullscreen = false;
                // P1：前台窗口句柄缓存 — 句柄未变时跳过完整的 Win32 API 检测，空闲开销趋近于零
                let mut last_fg_hwnd: isize = 0;
                loop {
                    if exit.is_exiting() { break; }
                    #[cfg(target_os = "windows")]
                    {
                        let fg_hwnd = win32_utils::foreground_window();
                        // 快速路径：前台窗口句柄与上次相同，说明窗口未切换，延长休眠减少 CPU 唤醒
                        if fg_hwnd == last_fg_hwnd {
                            if exit.sleep_interruptible(std::time::Duration::from_millis(2000)) {
                                break;
                            }
                            continue;
                        }
                        last_fg_hwnd = fg_hwnd;

                        let is_fullscreen = win32_utils::is_foreground_fullscreen();
                        if is_fullscreen != was_fullscreen {
                            win32_utils::log_err(
                                app_handle_for_fs.emit("fullscreen-changed", is_fullscreen),
                                "emit fullscreen-changed",
                            );
                            was_fullscreen = is_fullscreen;
                        }
                    }
                    // 正常检测路径：休眠 2s（可中断），收到退出信号立即返回
                    if exit.sleep_interruptible(std::time::Duration::from_millis(2000)) {
                        break;
                    }
                }
            });

            let args: Vec<String> = std::env::args().collect();
            let is_autostart = args.iter().any(|arg| arg == "--autostart");

            if let Some(main_window) = app.get_webview_window("main") {
                if !is_autostart {
                    win32_utils::log_err(main_window.show(), "show main window at startup");
                    win32_utils::log_err(main_window.set_focus(), "focus main window at startup");
                }
            }

            let quit_item = MenuItem::with_id(app, "quit", "强制退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Music Dynamic Island")
                .menu(&tray_menu)
                .on_menu_event(move |app_handle, event| {
                    if event.id == "quit" {
                        // 退出前把尚未落盘的配置和流量统计立即写入磁盘，避免“改了设置/跑了流量但重启丢失”
                        config_store::persist(app_handle).ok();
                        traffic_stats::persist(app_handle).ok();
                        std::process::exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        match app.get_webview_window("main") {
                            Some(main_window) => {
                                win32_utils::log_err(main_window.show(), "show main window from tray");
                                win32_utils::log_err(main_window.unminimize(), "unminimize main window from tray");
                                win32_utils::log_err(main_window.set_focus(), "focus main window from tray");
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
                            let hr = DwmSetWindowAttribute(hwnd_raw, DWMWA_BORDER_COLOR as u32, &border_color as *const _ as *const _, 4);
                            if hr != 0 {
                                eprintln!("[NSD][warn] DwmSetWindowAttribute(BORDER_COLOR) failed, hr=0x{:08X}", hr);
                            }

                            let corner_preference = DWMWCP_DONOTROUND;
                            let hr = DwmSetWindowAttribute(hwnd_raw, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &corner_preference as *const _ as *const _, 4);
                            if hr != 0 {
                                eprintln!("[NSD][warn] DwmSetWindowAttribute(CORNER_PREFERENCE) failed, hr=0x{:08X}", hr);
                            }
                        }
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}