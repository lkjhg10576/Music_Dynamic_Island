use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use serde::Serialize;
use serde::Deserialize;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
// 锁屏检测改用 winapi 0.3：其 OpenInputDesktop/CloseDesktop/GENERIC_READ API 在
// Win32_UI_WindowsAndMessaging / Win32_Foundation 下稳定且无需特殊处理（windows 0.58 的
// 桌面函数在该特性组合下未导出，会导致编译失败）。
use winapi::um::winuser::{CloseDesktop, OpenInputDesktop};
use winapi::um::winnt::GENERIC_READ;

// 结构化系统事件载荷（取代原纯文本 system-event / 结构化 battery-event）
// kind ∈ {volume, power, battery, network, network_latency, network_disconnect,
//          network_recovery, lock, unlock}
// level ∈ {info, success, warn}
#[derive(Clone, Serialize)]
struct SysEventPayload {
    kind: String,
    level: String,
    text: String,
    ts: u64,
}

#[derive(Clone, Serialize)]
struct NetworkStatusPayload {
    status: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemEventFilter {
    pub enabled: bool,
    pub volume: bool,
    pub power: bool,
    pub battery: bool,
    pub unlock: bool,
    #[serde(default)]
    pub network_latency: bool,
    #[serde(default)]
    pub network_disconnect: bool,
    #[serde(default)]
    pub network_recovery: bool,
}

// 分类门控原子：总闸 + 七个子类别
static SYS_EVT_ENABLED: AtomicBool = AtomicBool::new(true);
static SYS_EVT_VOLUME: AtomicBool = AtomicBool::new(true);
static SYS_EVT_POWER: AtomicBool = AtomicBool::new(true);
static SYS_EVT_BATTERY: AtomicBool = AtomicBool::new(true);
static SYS_EVT_UNLOCK: AtomicBool = AtomicBool::new(true);
static SYS_EVT_NETWORK_LATENCY: AtomicBool = AtomicBool::new(true);
static SYS_EVT_NETWORK_DISCONNECT: AtomicBool = AtomicBool::new(true);
static SYS_EVT_NETWORK_RECOVERY: AtomicBool = AtomicBool::new(true);

#[tauri::command]
pub fn set_system_event_filter(filter: SystemEventFilter) {
    SYS_EVT_ENABLED.store(filter.enabled, Ordering::Relaxed);
    SYS_EVT_VOLUME.store(filter.volume, Ordering::Relaxed);
    SYS_EVT_POWER.store(filter.power, Ordering::Relaxed);
    SYS_EVT_BATTERY.store(filter.battery, Ordering::Relaxed);
    SYS_EVT_UNLOCK.store(filter.unlock, Ordering::Relaxed);
    SYS_EVT_NETWORK_LATENCY.store(filter.network_latency, Ordering::Relaxed);
    SYS_EVT_NETWORK_DISCONNECT.store(filter.network_disconnect, Ordering::Relaxed);
    SYS_EVT_NETWORK_RECOVERY.store(filter.network_recovery, Ordering::Relaxed);
}

// 统一事件发射：仅在总闸开启时发送结构化 sysmsg-event
fn emit_sys_event(app: &AppHandle, kind: &str, level: &str, text: &str) {
    if !SYS_EVT_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let payload = SysEventPayload {
        kind: kind.to_string(),
        level: level.to_string(),
        text: text.to_string(),
        ts: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    };
    let _ = app.emit("sysmsg-event", payload);
}

/// 缓存默认播放设备的 IAudioEndpointVolume，避免每轮重新 CoCreate。
/// 仅在本监控线程内使用（与 CoInitializeEx 同线程）。设备切换 / COM 失败时 invalidate。
struct VolumeEndpointCache {
    endpoint: Option<IAudioEndpointVolume>,
}

impl VolumeEndpointCache {
    fn new() -> Self {
        Self { endpoint: None }
    }

    fn invalidate(&mut self) {
        self.endpoint = None;
    }

    fn ensure(&mut self) -> bool {
        if self.endpoint.is_some() {
            return true;
        }
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                    Ok(e) => e,
                    Err(_) => return false,
                };
            let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                Ok(d) => d,
                Err(_) => return false,
            };
            // 与原先 get_system_volume 一致：靠目标类型注解完成 Activate 泛型推导
            let endpoint: Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None);
            match endpoint {
                Ok(ep) => {
                    self.endpoint = Some(ep);
                    true
                }
                Err(_) => false,
            }
        }
    }

    /// 读取主音量标量 0.0~1.0；失败则清空缓存，下次重建。
    fn read_scalar(&mut self) -> Option<f32> {
        if !self.ensure() {
            return None;
        }
        unsafe {
            match self.endpoint.as_ref()?.GetMasterVolumeLevelScalar() {
                Ok(v) => Some(v),
                Err(_) => {
                    self.invalidate();
                    None
                }
            }
        }
    }
}

pub fn start_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        // 初始化 COM 接口以获取音频
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        let mut volume_cache = VolumeEndpointCache::new();
        // last_volume_pct: 0..=100；None 表示尚无有效读数，不触发“变化”
        let mut last_volume_pct: Option<i32> = volume_cache
            .read_scalar()
            .map(|v| (v * 100.0).round() as i32);
        let mut last_power_state = 255; // 255 = 未知状态
        let mut last_battery_percent = 255; // 255 = 未知电量
        // 锁屏检测：true = 已解锁（可操作桌面），false = 已锁定
        let mut last_unlocked = is_desktop_unlocked().unwrap_or(true);
        // 空闲优化：连续多次状态无变化时延长休眠
        let mut stability_counter: u32 = 0;
        // 最近一次音量变化时间：用于拉高轮询频率，避免连续调音量漏检
        let mut last_volume_change_at: Option<Instant> = None;
        // COM 连续失败计数：达阈值后强制 invalidate，防止设备切换后长期失效
        let mut volume_fail_streak: u32 = 0;

        // 启动时先拉取一次真实状态打底
        if let Some((ac_status, battery_percent)) = get_power_status() {
            last_power_state = ac_status;
            last_battery_percent = battery_percent;
        }

        loop {
            // 音量活跃窗口内 300ms（保证连续调音不漏）；刚有其它变化 600ms；完全空闲 2000ms 省性能
            let volume_hot = last_volume_change_at
                .map(|t| t.elapsed() < Duration::from_millis(2500))
                .unwrap_or(false);
            let sleep_ms = if volume_hot {
                300
            } else if stability_counter >= 3 {
                2000
            } else {
                600
            };
            std::thread::sleep(Duration::from_millis(sleep_ms));

            let mut changed = false;

            // 1. 检查音量变化（按整数百分比比较，对齐 Windows 步进，避免 0.01 浮点漏检）
            match volume_cache.read_scalar() {
                Some(current_volume) => {
                    volume_fail_streak = 0;
                    // 钳制到合法区间后四舍五入
                    let vol_percent = ((current_volume.clamp(0.0, 1.0)) * 100.0).round() as i32;
                    let vol_percent = vol_percent.clamp(0, 100);

                    if let Some(prev) = last_volume_pct {
                        if vol_percent != prev {
                            if SYS_EVT_VOLUME.load(Ordering::Relaxed) {
                                emit_sys_event(
                                    &app,
                                    "volume",
                                    "info",
                                    &format!("当前系统音量 {}%", vol_percent),
                                );
                            }
                            last_volume_change_at = Some(Instant::now());
                            changed = true;
                        }
                    }
                    last_volume_pct = Some(vol_percent);
                }
                None => {
                    volume_fail_streak = volume_fail_streak.saturating_add(1);
                    // 连续失败 3 次强制丢弃 endpoint，下一轮重新枚举默认设备
                    if volume_fail_streak >= 3 {
                        volume_cache.invalidate();
                        volume_fail_streak = 0;
                        // 设备可能已切换：清空基线，恢复后首次读数不弹（防假变化）
                        last_volume_pct = None;
                    }
                }
            }

            // 2. 检查电源状态与电量变化
            if let Some((current_power, current_percent)) = get_power_status() {
                // 【情况 A】电源插入/拔出状态发生了改变
                if current_power != last_power_state && last_power_state != 255 {
                    if current_power == 1 {
                        // 插入电源
                        if SYS_EVT_POWER.load(Ordering::Relaxed) {
                            emit_sys_event(
                                &app,
                                "power",
                                "success",
                                &format!("已接入电源，当前电量 {}%", current_percent),
                            );
                        }
                    } else if current_power == 0 {
                        // 拔出电源
                        if SYS_EVT_POWER.load(Ordering::Relaxed) {
                            emit_sys_event(&app, "power", "info", "正在使用电池供电");
                        }
                    }
                    changed = true;
                }

                // 【情况 B】正在使用电池（未插电），且电量正在下降
                if current_power == 0 && current_percent < last_battery_percent {
                    // 低电量防抖机制：仅在跌破这几个关键节点时，触发红色警告
                    if current_percent <= 20 && [20, 15, 10, 5].contains(&current_percent) {
                        if SYS_EVT_BATTERY.load(Ordering::Relaxed) {
                            emit_sys_event(
                                &app,
                                "battery",
                                "warn",
                                &format!("电池电量低，剩余 {}%", current_percent),
                            );
                        }
                        changed = true;
                    }
                }

                if current_power != last_power_state || current_percent != last_battery_percent {
                    changed = true;
                }
                last_power_state = current_power;
                last_battery_percent = current_percent;
            }

            // 3. 锁屏/解锁检测（解锁提示）：桌面不可用时视为已锁定
            if let Some(unlocked) = is_desktop_unlocked() {
                if unlocked != last_unlocked {
                    if SYS_EVT_UNLOCK.load(Ordering::Relaxed) {
                        if unlocked {
                            emit_sys_event(&app, "unlock", "info", "已解锁，欢迎回来");
                        } else {
                            emit_sys_event(&app, "lock", "info", "系统已锁定");
                        }
                    }
                    changed = true;
                }
                last_unlocked = unlocked;
            }

            // 空闲优化：状态变化时重置计数器，无变化时递增
            if changed {
                stability_counter = 0;
            } else {
                stability_counter = stability_counter.saturating_add(1);
            }
        }
    });
}

// 辅助函数：同时获取电源插入状态和剩余电量
fn get_power_status() -> Option<(u8, u8)> {
    unsafe {
        let mut status: SYSTEM_POWER_STATUS = std::mem::zeroed();
        if GetSystemPowerStatus(&mut status).is_ok() {
            // 返回元组: (ACLineStatus, BatteryLifePercent)
            // ACLineStatus: 0 = 使用电池, 1 = 插入电源
            // BatteryLifePercent: 0 ~ 100
            Some((status.ACLineStatus, status.BatteryLifePercent))
        } else {
            None
        }
    }
}

// 辅助函数：通过 OpenInputDesktop 判断当前会话是否处于锁定状态。
// 锁屏时输入桌面不可打开，返回 false；可操作时返回 true。非 Windows 平台直接返回 None（跳过检测）。
fn is_desktop_unlocked() -> Option<bool> {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            // OpenInputDesktop：锁屏时输入桌面不可打开，返回 NULL；可操作时返回有效句柄。
            // dwFlags=0，fInherit=FALSE(0)，dwDesiredAccess=GENERIC_READ。
            let h = OpenInputDesktop(0, 0, GENERIC_READ);
            if h.is_null() {
                // 打不开输入桌面（返回 NULL）→ 已锁定
                return Some(false);
            }
            let _ = CloseDesktop(h);
            Some(true)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // 非 Windows 平台无锁屏概念，跳过检测
        None
    }
}

// =============================================================================
// 网络感知：独立 NetworkMonitor 线程（NLM 连通性 + 延迟探测）
// =============================================================================

/// 延迟告警阈值：严格高于此值才算“延迟异常”
const NETWORK_LATENCY_ALERT_MS: u128 = 200;
/// 连续超时 ≤ 此值按“延迟过高”；超过则按“网络异常”
const NETWORK_TIMEOUT_WARN_MAX: u32 = 5;
/// NLM 轮询间隔
const NLM_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// 延迟探测的可观察结果（纯状态机输入）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatencySample {
    /// 探测成功，值为毫秒
    Ok(u128),
    /// 探测超时 / 失败
    Timeout,
}

/// 延迟子状态（用于边沿触发 toast）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatencyPhase {
    /// 正常（≤200ms）
    Good,
    /// 成功但 >200ms
    HighLatency,
    /// 连续超时 1..=5
    HighTimeout,
    /// 连续超时 ≥6
    NetworkAnomaly,
}

/// 网络监控内部状态
#[derive(Debug, Clone, Copy)]
struct NetworkMonitorState {
    /// None = 尚未建立 NLM 基线（首次有效结果不弹断开/恢复）
    connected: Option<bool>,
    consecutive_timeouts: u32,
    latency_phase: LatencyPhase,
    /// 最近一次发出的状态灯，避免重复 emit
    last_status: Option<&'static str>,
}

impl NetworkMonitorState {
    fn new() -> Self {
        Self {
            connected: None,
            consecutive_timeouts: 0,
            latency_phase: LatencyPhase::Good,
            last_status: None,
        }
    }
}

/// 状态机动作：由一轮 NLM / 延迟探测产生
#[derive(Debug, Clone, PartialEq, Eq)]
struct NetworkActions {
    /// 状态灯：good / warning / error；None 表示本轮不更新
    status: Option<&'static str>,
    /// 可选 toast：(kind, level, text)
    toast: Option<(&'static str, &'static str, String)>,
}

/// 处理 NLM 连通性变化。返回需要执行的动作，并就地更新 state。
/// - 首次有效结果仅建立基线
/// - connected→disconnected：清零超时与延迟子状态
/// - disconnected→connected：清零；随后延迟探测正常工作
fn apply_connectivity(
    state: &mut NetworkMonitorState,
    is_connected: bool,
) -> NetworkActions {
    match state.connected {
        None => {
            // 建立基线，不弹 toast
            state.connected = Some(is_connected);
            state.consecutive_timeouts = 0;
            state.latency_phase = LatencyPhase::Good;
            let status = if is_connected { "good" } else { "error" };
            state.last_status = Some(status);
            NetworkActions {
                status: Some(status),
                toast: None,
            }
        }
        Some(prev) if prev == is_connected => {
            // 无变化：断网期间保持 error 灯；联网期间由延迟探测驱动
            if !is_connected {
                NetworkActions {
                    status: Some("error"),
                    toast: None,
                }
            } else {
                NetworkActions {
                    status: None,
                    toast: None,
                }
            }
        }
        Some(true) if !is_connected => {
            // connected → disconnected
            state.connected = Some(false);
            state.consecutive_timeouts = 0;
            state.latency_phase = LatencyPhase::Good;
            state.last_status = Some("error");
            NetworkActions {
                status: Some("error"),
                toast: Some((
                    "network_disconnect",
                    "warn",
                    "网络已断开".to_string(),
                )),
            }
        }
        Some(false) if is_connected => {
            // disconnected → connected
            state.connected = Some(true);
            state.consecutive_timeouts = 0;
            state.latency_phase = LatencyPhase::Good;
            state.last_status = Some("good");
            NetworkActions {
                status: Some("good"),
                toast: Some((
                    "network_recovery",
                    "success",
                    "网络已恢复连接".to_string(),
                )),
            }
        }
        _ => NetworkActions {
            status: None,
            toast: None,
        },
    }
}

/// 处理一次延迟探测结果（仅在 NLM 判定已联网时调用）。
/// 细节1：超时 ≤5 → warning + “延迟过高”
/// 细节2：超时 >5 → error + “网络异常”（第 6 次边沿）
/// 成功且 ≤200ms → good；成功且 >200ms → warning + “网络延迟异常 Xms”
fn apply_latency_sample(
    state: &mut NetworkMonitorState,
    sample: LatencySample,
) -> NetworkActions {
    // 未联网时不应调用；防御性跳过
    if state.connected != Some(true) {
        return NetworkActions {
            status: None,
            toast: None,
        };
    }

    match sample {
        LatencySample::Ok(ms) => {
            state.consecutive_timeouts = 0;
            if ms > NETWORK_LATENCY_ALERT_MS {
                let entering = state.latency_phase != LatencyPhase::HighLatency;
                state.latency_phase = LatencyPhase::HighLatency;
                state.last_status = Some("warning");
                NetworkActions {
                    status: Some("warning"),
                    toast: if entering {
                        Some((
                            "network_latency",
                            "warn",
                            format!("网络延迟异常 {}ms", ms),
                        ))
                    } else {
                        None
                    },
                }
            } else {
                state.latency_phase = LatencyPhase::Good;
                state.last_status = Some("good");
                NetworkActions {
                    status: Some("good"),
                    toast: None,
                }
            }
        }
        LatencySample::Timeout => {
            state.consecutive_timeouts = state.consecutive_timeouts.saturating_add(1);
            let n = state.consecutive_timeouts;
            if n <= NETWORK_TIMEOUT_WARN_MAX {
                // 第 1~5 次：延迟过高
                let entering = state.latency_phase != LatencyPhase::HighTimeout;
                state.latency_phase = LatencyPhase::HighTimeout;
                state.last_status = Some("warning");
                NetworkActions {
                    status: Some("warning"),
                    toast: if entering {
                        Some((
                            "network_latency",
                            "warn",
                            "延迟过高".to_string(),
                        ))
                    } else {
                        None
                    },
                }
            } else {
                // 第 6 次及以后：网络异常；仅第 6 次边沿弹一次
                let entering = state.latency_phase != LatencyPhase::NetworkAnomaly;
                state.latency_phase = LatencyPhase::NetworkAnomaly;
                state.last_status = Some("error");
                NetworkActions {
                    status: Some("error"),
                    toast: if entering {
                        Some((
                            "network_latency",
                            "warn",
                            "网络异常".to_string(),
                        ))
                    } else {
                        None
                    },
                }
            }
        }
    }
}

/// 按门控发送 sysmsg toast；network-status 始终发送（不受分类门控）
fn emit_network_actions(app: &AppHandle, actions: &NetworkActions) {
    if let Some(status) = actions.status {
        let _ = app.emit(
            "network-status",
            NetworkStatusPayload {
                status: status.to_string(),
            },
        );
    }
    if let Some((kind, level, text)) = &actions.toast {
        let allowed = match *kind {
            "network_latency" => SYS_EVT_NETWORK_LATENCY.load(Ordering::Relaxed),
            "network_disconnect" => SYS_EVT_NETWORK_DISCONNECT.load(Ordering::Relaxed),
            "network_recovery" => SYS_EVT_NETWORK_RECOVERY.load(Ordering::Relaxed),
            _ => true,
        };
        if allowed {
            emit_sys_event(app, kind, level, text);
        }
    }
}

/// 启动网络感知监控线程。
/// Windows：独立 std::thread + CoInitializeEx + NLM；非 Windows：空操作。
pub fn start_network_monitor(app: AppHandle) {
    #[cfg(target_os = "windows")]
    {
        std::thread::spawn(move || {
            network_monitor_loop(app);
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        // 非 Windows：NLM 不存在，跳过网络监测
    }
}

#[cfg(target_os = "windows")]
fn network_monitor_loop(app: AppHandle) {
    use windows::Win32::NetworkManagement::NetworkListManager::{
        INetworkListManager, NetworkListManager,
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    // 创建 NLM；失败时后续轮询重试
    let mut nlm: Option<INetworkListManager> = unsafe {
        CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).ok()
    };

    let mut state = NetworkMonitorState::new();
    // 下次允许延迟探测的时间；启动后立即尝试（若已联网）
    let mut next_latency_at = Instant::now();

    loop {
        // 1) NLM 连通性（每 5s）
        if nlm.is_none() {
            nlm = unsafe { CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).ok() };
        }

        let connectivity = nlm.as_ref().and_then(|mgr| {
            // IsConnectedToInternet 失败时返回 None，本轮跳过，不误判断网
            unsafe { mgr.IsConnectedToInternet().ok().map(|v| v.as_bool()) }
        });

        if let Some(is_connected) = connectivity {
            let actions = apply_connectivity(&mut state, is_connected);
            emit_network_actions(&app, &actions);

            // 细节3：未连接时延迟探测 skip，并清零超时计数（已在 apply_connectivity 处理）
            if is_connected {
                // 2) 延迟探测：按用户设定间隔（1~60s，默认 30）
                let interval =
                    Duration::from_secs(crate::network_latency_interval_secs());
                if Instant::now() >= next_latency_at {
                    let sample = match tauri::async_runtime::block_on(crate::get_network_latency())
                    {
                        Ok(ms) => LatencySample::Ok(ms),
                        Err(_) => LatencySample::Timeout,
                    };
                    // 探测期间若 NLM 已断网，跳过（防御）
                    if state.connected == Some(true) {
                        let lat_actions = apply_latency_sample(&mut state, sample);
                        emit_network_actions(&app, &lat_actions);
                    }
                    next_latency_at = Instant::now() + interval;
                }
            } else {
                // 断网：不探测；重置下次探测为“恢复后立即探测”
                next_latency_at = Instant::now();
            }
        }
        // NLM 调用失败：本轮跳过，不改变 state

        std::thread::sleep(NLM_POLL_INTERVAL);
    }
}

#[cfg(test)]
mod network_monitor_tests {
    use super::*;

    #[test]
    fn baseline_does_not_toast() {
        let mut s = NetworkMonitorState::new();
        let a = apply_connectivity(&mut s, true);
        assert_eq!(a.status, Some("good"));
        assert!(a.toast.is_none());
        assert_eq!(s.connected, Some(true));

        let mut s2 = NetworkMonitorState::new();
        let a2 = apply_connectivity(&mut s2, false);
        assert_eq!(a2.status, Some("error"));
        assert!(a2.toast.is_none());
    }

    #[test]
    fn disconnect_and_recovery_edge() {
        let mut s = NetworkMonitorState::new();
        let _ = apply_connectivity(&mut s, true);
        let disc = apply_connectivity(&mut s, false);
        assert_eq!(disc.status, Some("error"));
        assert_eq!(
            disc.toast.as_ref().map(|(k, _, t)| (*k, t.as_str())),
            Some(("network_disconnect", "网络已断开"))
        );
        assert_eq!(s.consecutive_timeouts, 0);

        let rec = apply_connectivity(&mut s, true);
        assert_eq!(rec.status, Some("good"));
        assert_eq!(
            rec.toast.as_ref().map(|(k, _, t)| (*k, t.as_str())),
            Some(("network_recovery", "网络已恢复连接"))
        );
    }

    #[test]
    fn latency_threshold_200ms() {
        let mut s = NetworkMonitorState::new();
        let _ = apply_connectivity(&mut s, true);

        let good = apply_latency_sample(&mut s, LatencySample::Ok(200));
        assert_eq!(good.status, Some("good"));
        assert!(good.toast.is_none());

        let high = apply_latency_sample(&mut s, LatencySample::Ok(201));
        assert_eq!(high.status, Some("warning"));
        assert!(high
            .toast
            .as_ref()
            .unwrap()
            .2
            .contains("网络延迟异常 201ms"));

        // 持续高延迟不再弹
        let high2 = apply_latency_sample(&mut s, LatencySample::Ok(300));
        assert_eq!(high2.status, Some("warning"));
        assert!(high2.toast.is_none());
    }

    #[test]
    fn timeout_5_then_6() {
        let mut s = NetworkMonitorState::new();
        let _ = apply_connectivity(&mut s, true);

        // 第 1 次超时：进入延迟过高
        let t1 = apply_latency_sample(&mut s, LatencySample::Timeout);
        assert_eq!(t1.status, Some("warning"));
        assert_eq!(
            t1.toast.as_ref().map(|(_, _, t)| t.as_str()),
            Some("延迟过高")
        );
        assert_eq!(s.consecutive_timeouts, 1);

        // 第 2~5 次：保持 warning，不再弹
        for i in 2..=5 {
            let t = apply_latency_sample(&mut s, LatencySample::Timeout);
            assert_eq!(t.status, Some("warning"), "timeout #{}", i);
            assert!(t.toast.is_none(), "timeout #{} should not toast", i);
            assert_eq!(s.consecutive_timeouts, i);
        }

        // 第 6 次：网络异常
        let t6 = apply_latency_sample(&mut s, LatencySample::Timeout);
        assert_eq!(t6.status, Some("error"));
        assert_eq!(
            t6.toast.as_ref().map(|(_, _, t)| t.as_str()),
            Some("网络异常")
        );
        assert_eq!(s.consecutive_timeouts, 6);

        // 第 7 次：不再弹
        let t7 = apply_latency_sample(&mut s, LatencySample::Timeout);
        assert_eq!(t7.status, Some("error"));
        assert!(t7.toast.is_none());
    }

    #[test]
    fn success_resets_timeout_counter() {
        let mut s = NetworkMonitorState::new();
        let _ = apply_connectivity(&mut s, true);
        for _ in 0..3 {
            let _ = apply_latency_sample(&mut s, LatencySample::Timeout);
        }
        assert_eq!(s.consecutive_timeouts, 3);
        let ok = apply_latency_sample(&mut s, LatencySample::Ok(50));
        assert_eq!(ok.status, Some("good"));
        assert_eq!(s.consecutive_timeouts, 0);
        assert_eq!(s.latency_phase, LatencyPhase::Good);
    }

    #[test]
    fn disconnect_resets_timeout_and_skips_latency() {
        let mut s = NetworkMonitorState::new();
        let _ = apply_connectivity(&mut s, true);
        for _ in 0..4 {
            let _ = apply_latency_sample(&mut s, LatencySample::Timeout);
        }
        assert_eq!(s.consecutive_timeouts, 4);
        let _ = apply_connectivity(&mut s, false);
        assert_eq!(s.consecutive_timeouts, 0);
        // 未联网时延迟探测被忽略
        let skipped = apply_latency_sample(&mut s, LatencySample::Timeout);
        assert!(skipped.status.is_none());
        assert!(skipped.toast.is_none());
        assert_eq!(s.consecutive_timeouts, 0);
    }
}
