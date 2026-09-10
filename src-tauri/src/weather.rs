//! 恶劣天气提醒（Weather Reminder）后端模块
//!
//! 数据源：小米天气 wtr-v3（无需 Key、固定 sign）；每小时自动拉取；
//! 恶劣天气边沿触发灵动岛通知 + 用户可选的早/午/晚报。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter};
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// 常量
// ──────────────────────────────────────────────

const DEFAULT_POLL_INTERVAL_SECS: u64 = 3600;   // 默认 1 小时
const MIN_POLL_INTERVAL_SECS: u64 = 1800;       // 0.5h 下限
const MAX_POLL_INTERVAL_SECS: u64 = 10800;      // 3h 上限
const IDLE_WAKE_SECS: u64 = 60;                // 空闲时 1min 唤醒（用于判定早午晚报）
const SEVERE_MAX_FAIL: u32 = 3;
const SEVERE_SOUND_COUNT: u32 = 3;
const SEVERE_SOUND_GAP_MS: u64 = 600;

const BASE_URL: &str = "https://weatherapi.market.xiaomi.com/wtr-v3";
const APP_KEY: &str = "weather20151024";
const SIGN: &str = "zUFJoAR2ZVrDy1vF3D07";

// 早午晚报时段（本地时区，按分钟计）
const BRIEF_MORNING_START: u32 = 7 * 60;       // 7:00
const BRIEF_MORNING_END: u32 = 10 * 60 + 59;   // 10:59
const BRIEF_NOON_START: u32 = 11 * 60;
const BRIEF_NOON_END: u32 = 14 * 60 + 59;
const BRIEF_EVENING_START: u32 = 18 * 60;
const BRIEF_EVENING_END: u32 = 20 * 60 + 59;

// 动态配置（用户设置后立即生效）
static WEATHER_POLL_INTERVAL_SECS: AtomicU64 = AtomicU64::new(DEFAULT_POLL_INTERVAL_SECS);
static WEATHER_ALERT_THRESHOLD: AtomicU8 = AtomicU8::new(0);  // 0=B 默认全提醒
static WEATHER_LIGHT_ALERT_ENABLED: AtomicBool = AtomicBool::new(false);

// ──────────────────────────────────────────────
// 数据模型
// ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityInfo {
    pub city_id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub province: Option<String>,
}

// ⚠ 这三个结构体直接进 weather-tick 事件载荷，前端（useWeather.ts / IslandWeatherPanel.vue）
// 按 camelCase 读取（weatherText / feelsLike / tempMax / alertId ...）。此前缺 rename_all，
// serde 输出蛇形字段名，导致面板当前天气、今日预报、预警条三块全部取到 undefined（静默空白）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertInfo {
    pub alert_id: String,
    // 前端 WeatherAlert.type 读的是 `type`（不是 typeName），单独改名对齐
    #[serde(rename = "type")]
    pub type_name: String,
    pub level: String,       // 'B' | 'Y' | 'O' | 'R' | 'W'
    pub level_text: String,  // '蓝色预警'
    pub title: String,
    pub pub_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherSnapshot {
    pub current: Option<CurrentWeather>,
    pub today: Option<DailyForecast>,
    pub tomorrow: Option<DailyForecast>,
    pub alerts: Vec<AlertInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentWeather {
    pub temperature: f64,
    pub weather_code: i32,
    pub weather_text: String,
    pub feels_like: f64,
    pub humidity: f64,
    pub wind_speed: f64,
    pub pm25: Option<f64>,
    pub aqi: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyForecast {
    pub temp_max: f64,
    pub temp_min: f64,
    pub day_code: i32,
    pub night_code: i32,
    pub precip_prob: f64,
    pub sunrise: Option<String>,
    pub sunset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SevereEvent {
    pub kind: SevereKind,
    pub icon: String,
    pub title: String,
    pub body: String,
    pub severity: String,
    pub dedup_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SevereKind {
    RainOrSnow,
    FogOrHaze,
    Alert(AlertInfo),
    TempGap,
}

// ──────────────────────────────────────────────
// 状态机
// ──────────────────────────────────────────────

struct WeatherState {
    city: Option<CityInfo>,
    last: Option<WeatherSnapshot>,
    last_alert_ids: HashSet<String>,
    fail_streak: u32,
    last_fetch_at: AtomicU64,
    dirty: AtomicBool,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            city: None,
            last: None,
            last_alert_ids: HashSet::new(),
            fail_streak: 0,
            last_fetch_at: AtomicU64::new(0),
            dirty: AtomicBool::new(false),
        }
    }
}

// ──────────────────────────────────────────────
// 共享运行时状态 + 开发者桥接注入点
//
// 后台线程原本把状态全放在循环内的局部 `state` 里，外部（weather_get_state /
// 后续的拉取判定）读不到，导致"改城市不生效""状态查询恒为空"。
// 这里把需要跨线程可见的部分提为全局；注入点仅由 dev_bridge 使用
// （仅 9.9.9-* 测试版会启动桥接，见 dev_bridge::DEV_BUILD）。
// ──────────────────────────────────────────────

/// 最近一次真实拉取成功的快照（供状态查询与 tick 复用）
static LATEST_SNAPSHOT: Mutex<Option<WeatherSnapshot>> = Mutex::new(None);
/// 当前城市（线程与命令共享；桥接改城市后线程下一轮即生效，无需重启）
static LATEST_CITY: Mutex<Option<CityInfo>> = Mutex::new(None);
/// 桥接注入的快照：Some 时 tick 优先使用它，清空后自动回到真实数据
static INJECTED_SNAPSHOT: Mutex<Option<WeatherSnapshot>> = Mutex::new(None);
/// 桥接覆盖的本地分钟数（模拟早/午/晚报时段）；-1 = 不覆盖，用真实本地时间
static BRIEF_MINUTES_OVERRIDE: std::sync::atomic::AtomicI32 =
    std::sync::atomic::AtomicI32::new(-1);
/// 桥接请求立即拉取（线程下一轮即触发，不等整点）
static FORCE_FETCH: AtomicBool = AtomicBool::new(false);
/// 早/午/晚报"当天已推送"记录（key='morning' value='2026-09-08'）。
/// 提为全局而非线程局部：桥接需要能重置它，否则同一自然日内只能测一次。
static LAST_BRIEF_DATES: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
/// 最近一次拉取时刻 / 连续失败次数（供状态查询）
static LATEST_FETCH_AT: AtomicU64 = AtomicU64::new(0);
static LATEST_FAIL_STREAK: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

/// 当前有效快照：注入优先，否则回落到最近一次真实拉取结果
fn effective_snapshot() -> Option<WeatherSnapshot> {
    INJECTED_SNAPSHOT
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .or_else(|| LATEST_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).clone())
}

fn current_city() -> Option<CityInfo> {
    LATEST_CITY.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

// ──────────────────────────────────────────────
// 工具函数
// ──────────────────────────────────────────────

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 本地日期字符串 YYYY-MM-DD（东八区），用于"每天一次"类去重键。
/// 旧实现按"一年 365 天 / 一月 30 天"近似换算，会算出不存在的日期、并把去重键
/// 落到错误的日子；这里改用 Howard Hinnant 的 civil_from_days 精确换算。
fn today_key() -> String {
    let days = (unix_now() as i64 + 8 * 3600).div_euclid(86_400);
    // days 为 1970-01-01 起的天数；+719468 平移到 0000-03-01 纪元
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// 本地（东八区）当天已过的分钟数。
/// 桥接可通过 BRIEF_MINUTES_OVERRIDE 覆盖，用于在任意时刻测试早/午/晚报时段判定。
fn local_minutes() -> u32 {
    let over = BRIEF_MINUTES_OVERRIDE.load(Ordering::Relaxed);
    if over >= 0 {
        return (over as u32).min(24 * 60 - 1);
    }
    let now = unix_now() as i64;
    let ts = now + 8 * 3600;
    let mins = (ts % 86400) / 60;
    mins as u32
}

fn level_rank(level: &str) -> u8 {
    match level {
        "B" | "蓝色" => 0,
        "Y" | "黄色" => 1,
        "O" | "橙色" => 2,
        "R" | "红色" => 3,
        _ => 0, // 白色 / 未知 → 蓝色档兜底
    }
}

fn level_to_text(level: &str) -> String {
    match level {
        "B" | "蓝色" => "蓝色预警".to_string(),
        "Y" | "黄色" => "黄色预警".to_string(),
        "O" | "橙色" => "橙色预警".to_string(),
        "R" | "红色" => "红色预警".to_string(),
        _ => "蓝色预警".to_string(),
    }
}

fn level_to_letter(level: &str) -> String {
    match level {
        "B" | "蓝色" => "B".to_string(),
        "Y" | "黄色" => "Y".to_string(),
        "O" | "橙色" => "O".to_string(),
        "R" | "红色" => "R".to_string(),
        _ => "B".to_string(),
    }
}

fn is_rain_or_snow(c: i32) -> bool {
    matches!(c, 3..=6 | 7..=12 | 13..=17 | 19 | 21..=28 | 301 | 302)
}

fn is_fog(c: i32) -> bool {
    matches!(c, 18 | 32 | 35 | 49 | 53..=58)
}

fn weather_code_to_text(code: i32) -> &'static str {
    match code {
        0 => "晴",
        1 => "多云",
        2 => "阴",
        3 => "阵雨",
        4 => "雷阵雨",
        5 => "雷阵雨伴有冰雹",
        6 => "雨夹雪",
        7 => "小雨",
        8 => "中雨",
        9 => "大雨",
        10 => "暴雨",
        11 => "大暴雨",
        12 => "特大暴雨",
        13 => "阵雪",
        14 => "小雪",
        15 => "中雪",
        16 => "大雪",
        17 => "暴雪",
        18 => "雾",
        19 => "冻雨",
        20 => "沙尘暴",
        21 => "小到中雨",
        22 => "中到大雨",
        23 => "大到暴雨",
        24 => "暴雨到大暴雨",
        25 => "大暴雨到特大暴雨",
        26 => "小到中雪",
        27 => "中到大雪",
        28 => "大到暴雪",
        301 => "雨",
        302 => "雪",
        _ => "未知",
    }
}

// ──────────────────────────────────────────────
// HTTP 客户端
// ──────────────────────────────────────────────

async fn fetch_weather(city: &CityInfo) -> Result<WeatherSnapshot, String> {
    let url = format!(
        "{}/weather/all?latitude={}&longitude={}&locationKey=weathercn:{}&days=15&appKey={}&sign={}&isGlobal=false&locale=zh_cn&ts={}",
        BASE_URL, city.lat, city.lon, city.city_id, APP_KEY, SIGN, unix_now()
    );
    let body: serde_json::Value = reqwest::Client::new()
        .get(&url).timeout(Duration::from_secs(10))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    parse_snapshot(&body)
}

fn parse_snapshot(body: &serde_json::Value) -> Result<WeatherSnapshot, String> {
    let current = body.get("current").and_then(|c| {
        Some(CurrentWeather {
            temperature: c.get("temperature")?.get("value")?.as_str()?.parse::<f64>().ok()?,
            weather_code: c.get("weather")?.as_str()?.parse::<i32>().ok()?,
            weather_text: weather_code_to_text(
                c.get("weather")?.as_str()?.parse::<i32>().ok()?
            ).to_string(),
            feels_like: c.get("feelsLike")?.get("value")?.as_str()?.parse::<f64>().ok()?,
            humidity: c.get("humidity")?.get("value")?.as_str()?.parse::<f64>().ok()?,
            wind_speed: c.get("wind")?.get("speed")?.get("value")?.as_str()?.parse::<f64>().ok()?,
            pm25: body.get("aqi").and_then(|a| a.get("pm25")).and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
            aqi: body.get("aqi").and_then(|a| a.get("aqi")).and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()),
        })
    });

    let today = body.get("forecastDaily").and_then(|fd| {
        let temp_vals = fd.get("temperature")?.get("value")?.as_array()?;
        let weather_vals = fd.get("weather")?.get("value")?.as_array()?;
        let precip_vals = fd.get("precipitationProbability")?.get("value")?.as_array()?;
        let sun_vals = fd.get("sunRiseSet")?.get("value")?.as_array()?;
        Some(DailyForecast {
            temp_max: temp_vals.get(0)?.get("from")?.as_str()?.parse::<f64>().ok()?,
            temp_min: temp_vals.get(0)?.get("to")?.as_str()?.parse::<f64>().ok()?,
            day_code: weather_vals.get(0)?.get("from")?.as_str()?.parse::<i32>().ok()?,
            night_code: weather_vals.get(0)?.get("to")?.as_str()?.parse::<i32>().ok()?,
            precip_prob: precip_vals.get(0)?.as_str()?.parse::<f64>().ok()?,
            sunrise: sun_vals.get(0)?.get("from")?.as_str().map(|s| s[..5].to_string()),
            sunset: sun_vals.get(0)?.get("to")?.as_str().map(|s| s[..5].to_string()),
        })
    });

    let tomorrow = body.get("forecastDaily").and_then(|fd| {
        let temp_vals = fd.get("temperature")?.get("value")?.as_array()?;
        let weather_vals = fd.get("weather")?.get("value")?.as_array()?;
        Some(DailyForecast {
            temp_max: temp_vals.get(1)?.get("from")?.as_str()?.parse::<f64>().ok()?,
            temp_min: temp_vals.get(1)?.get("to")?.as_str()?.parse::<f64>().ok()?,
            day_code: weather_vals.get(1)?.get("from")?.as_str()?.parse::<i32>().ok()?,
            night_code: weather_vals.get(1)?.get("to")?.as_str()?.parse::<i32>().ok()?,
            precip_prob: 0.0,
            sunrise: None,
            sunset: None,
        })
    });

    let alerts = body.get("alerts").and_then(|a| a.as_array()).map(|arr| {
        arr.iter().filter_map(|item| {
            Some(AlertInfo {
                alert_id: item.get("alertId")?.as_str()?.to_string(),
                type_name: item.get("type")?.as_str()?.to_string(),
                level: level_to_letter(item.get("level")?.as_str()?),
                level_text: level_to_text(item.get("level")?.as_str()?),
                title: item.get("title")?.as_str()?.to_string(),
                pub_time: item.get("pubTime")?.as_str().map(|s| s.to_string()),
            })
        }).collect()
    }).unwrap_or_default();

    Ok(WeatherSnapshot { current, today, tomorrow, alerts })
}

fn parse_cities(body: &serde_json::Value) -> Vec<CityInfo> {
    body.as_array().map(|arr| {
        arr.iter().filter_map(|item| {
            let location_key = item.get("locationKey")?.as_str()?;
            let city_id = location_key.strip_prefix("weathercn:").unwrap_or(location_key).to_string();
            Some(CityInfo {
                city_id,
                name: item.get("name")?.as_str()?.to_string(),
                lat: item.get("latitude")?.as_str()?.parse::<f64>().ok()?,
                lon: item.get("longitude")?.as_str()?.parse::<f64>().ok()?,
                province: item.get("affiliation")?.as_str().map(|s| s.to_string()),
            })
        }).collect()
    }).unwrap_or_default()
}

// ──────────────────────────────────────────────
// 恶劣天气判定
// ──────────────────────────────────────────────

fn detect_severe_changes(
    prev: &WeatherSnapshot,
    cur: &WeatherSnapshot,
    prev_alert_ids: &HashSet<String>,
    alert_threshold: u8,
) -> Vec<SevereEvent> {
    let mut out = vec![];
    let cur_codes: Vec<i32> = cur.current.iter().map(|c| c.weather_code).chain(
        cur.today.iter().flat_map(|t| vec![t.day_code, t.night_code])
    ).collect();
    let prev_codes: Vec<i32> = prev.current.iter().map(|c| c.weather_code).chain(
        prev.today.iter().flat_map(|t| vec![t.day_code, t.night_code])
    ).collect();

    // 1) 雨/雪
    let prev_any_rain = prev_codes.iter().any(|&c| is_rain_or_snow(c));
    let cur_any_rain = cur_codes.iter().any(|&c| is_rain_or_snow(c));
    if cur_any_rain && !prev_any_rain {
        let code = cur.current.as_ref().map(|c| c.weather_code).unwrap_or(0);
        let title = format!("即将有{}", weather_code_to_text(code));
        out.push(SevereEvent {
            kind: SevereKind::RainOrSnow,
            icon: "rain".to_string(),
            title,
            body: "".to_string(),
            severity: "warn".to_string(),
            dedup_key: format!("rain:{}", today_key()),
        });
    }

    // 2) 雾霾
    let prev_fog = prev_codes.iter().any(|&c| is_fog(c)) ||
        prev.current.as_ref().and_then(|c| c.aqi).unwrap_or(0.0) >= 150.0;
    let cur_fog = cur_codes.iter().any(|&c| is_fog(c)) ||
        cur.current.as_ref().and_then(|c| c.aqi).unwrap_or(0.0) >= 150.0;
    if cur_fog && !prev_fog {
        let code = cur.current.as_ref().map(|c| c.weather_code).unwrap_or(0);
        let title = format!("即将有{}", weather_code_to_text(code));
        out.push(SevereEvent {
            kind: SevereKind::FogOrHaze,
            icon: if is_fog(code) { "fog" } else { "haze" }.to_string(),
            title,
            body: "".to_string(),
            severity: "warn".to_string(),
            dedup_key: format!("fog:{}", today_key()),
        });
    }

    // 3) 预警
    for a in &cur.alerts {
        if level_rank(&a.level) >= alert_threshold && !prev_alert_ids.contains(&a.alert_id) {
            out.push(SevereEvent {
                kind: SevereKind::Alert(a.clone()),
                icon: "alert".to_string(),
                title: format!("{}{}", a.type_name, a.level_text),
                body: a.title.clone(),
                severity: "danger".to_string(),
                dedup_key: format!("alert:{}", a.alert_id),
            });
        }
    }

    // 4) 温度异常
    if let Some(cur_current) = &cur.current {
        let temp_gap = (cur_current.temperature - cur_current.feels_like).abs();
        if temp_gap > 5.0 {
            out.push(SevereEvent {
                kind: SevereKind::TempGap,
                icon: "temp".to_string(),
                title: "体感差异较大".to_string(),
                body: format!("当前 {}℃，体感 {}℃", cur_current.temperature, cur_current.feels_like),
                severity: "info".to_string(),
                dedup_key: format!("temp:{}", today_key()),
            });
        }
    }

    out
}

// ──────────────────────────────────────────────
// 提示音
// ──────────────────────────────────────────────

fn play_brief_sound() {
    std::thread::spawn(|| {
        crate::win32_utils::message_beep(0x30);
    });
}

fn play_severe_sound() {
    std::thread::spawn(|| {
        for i in 0..SEVERE_SOUND_COUNT {
            crate::win32_utils::message_beep(0x30);
            if i + 1 < SEVERE_SOUND_COUNT {
                std::thread::sleep(Duration::from_millis(SEVERE_SOUND_GAP_MS));
            }
        }
    });
}

// ──────────────────────────────────────────────
// Emit 函数
// ──────────────────────────────────────────────

/// 组装 weather-tick 载荷：注入快照优先，其余字段取全局运行时状态。
/// 快照在 json! 之前先取成自有值，避免借用临时量。
fn tick_payload() -> serde_json::Value {
    let (current, today, tomorrow, alerts) = match effective_snapshot() {
        Some(s) => (s.current, s.today, s.tomorrow, s.alerts),
        None => (None, None, None, Vec::new()),
    };
    serde_json::json!({
        "city": current_city(),
        "current": current,
        "today": today,
        "tomorrow": tomorrow,
        "alerts": alerts,
        "lastFetchAt": LATEST_FETCH_AT.load(Ordering::Relaxed),
        "fetchStatus": if LATEST_FAIL_STREAK.load(Ordering::Relaxed) > 0 { "failed" } else { "ok" },
    })
}

fn emit_tick(app: &AppHandle) {
    crate::win32_utils::log_err(app.emit("weather-tick", tick_payload()), "emit weather-tick");
}

fn push_severe(app: &AppHandle, event: &SevereEvent) {
    let payload = serde_json::json!({
        "kind": "severe",
        "icon": event.icon,
        "title": event.title,
        "body": event.body,
        "severity": event.severity,
        "dedupKey": event.dedup_key,
    });
    crate::win32_utils::log_err(app.emit("weather-toast", payload), "emit weather-toast");
}

fn push_light_alert(app: &AppHandle, alert: &AlertInfo) {
    let payload = serde_json::json!({
        "alertId": alert.alert_id,
        "level": alert.level,
        "levelText": alert.level_text,
        "type": alert.type_name,
        "title": alert.title,
    });
    crate::win32_utils::log_err(app.emit("weather-light-alert", payload), "emit weather-light-alert");
}

fn push_brief(app: &AppHandle, kind: &str, title: &str, body: &str) {
    let payload = serde_json::json!({
        "kind": kind,
        "icon": "sun",
        "title": title,
        "body": body,
        "severity": "info",
        "dedupKey": format!("{}:{}", kind, today_key()),
    });
    crate::win32_utils::log_err(app.emit("weather-toast", payload), "emit weather-toast brief");
}

fn push_severe_unavailable(app: &AppHandle) {
    let payload = serde_json::json!({
        "kind": "severe",
        "icon": "alert",
        "title": "天气服务暂时不可用",
        "body": "请检查网络连接",
        "severity": "warn",
        "dedupKey": format!("unavailable:{}", today_key()),
    });
    crate::win32_utils::log_err(app.emit("weather-toast", payload), "emit weather-toast unavailable");
}

// ──────────────────────────────────────────────
// 早午晚报判定
// ──────────────────────────────────────────────

fn detect_brief_window() -> Option<&'static str> {
    let mins = local_minutes();
    if mins >= BRIEF_MORNING_START && mins <= BRIEF_MORNING_END {
        Some("morning")
    } else if mins >= BRIEF_NOON_START && mins <= BRIEF_NOON_END {
        Some("noon")
    } else if mins >= BRIEF_EVENING_START && mins <= BRIEF_EVENING_END {
        Some("evening")
    } else {
        None
    }
}

fn is_brief_enabled(_brief: &str) -> bool {
    // TODO: 从 config_store 读取 NSD_WEATHER_DAILY_BRIEF
    true // 简化实现
}

/// 早/午/晚报文案：(标题, 正文)。
/// 正文取当前快照的真实数据 —— 旧实现固定写 "数据加载中" 且从不更新，
/// 推送出去等于一条没有信息的空提醒。
fn brief_text(brief: &str, snap: Option<&WeatherSnapshot>) -> (String, String) {
    let greet = match brief {
        "morning" => "早上好",
        "noon" => "中午好",
        "evening" => "晚上好",
        _ => "天气提醒",
    };
    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = snap {
        if let Some(c) = s.current.as_ref() {
            parts.push(format!("{} {}℃", c.weather_text, c.temperature.round() as i64));
        }
        if let Some(t) = s.today.as_ref() {
            parts.push(format!(
                "{}/{}℃",
                t.temp_max.round() as i64,
                t.temp_min.round() as i64
            ));
            if t.precip_prob > 0.0 {
                parts.push(format!("降水 {}%", t.precip_prob.round() as i64));
            }
        }
        if let Some(a) = s.alerts.first() {
            parts.push(format!("{}{}", a.type_name, a.level_text));
        }
    }
    let body = if parts.is_empty() {
        "数据加载中".to_string()
    } else {
        parts.join(" · ")
    };
    (greet.to_string(), body)
}

/// 早/午/晚报去重键：该报今天是否已推送
fn brief_already_sent(brief: &str) -> bool {
    LAST_BRIEF_DATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(brief)
        == Some(&today_key())
}

fn mark_brief_sent(brief: &str) {
    LAST_BRIEF_DATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(brief.to_string(), today_key());
}

// ──────────────────────────────────────────────
// 主循环
// ──────────────────────────────────────────────

pub fn start_weather_thread(app: AppHandle) {
    crate::thread_mgr::spawn_managed("weather_poll", move |exit| {
        let rt = tokio::runtime::Runtime::new().expect("create tokio runtime for weather_poll");
        let mut state = WeatherState::default();

        // 尝试恢复城市配置（同时写入共享城市，供命令/桥接与状态查询读取）
        if let Some(city_json) = crate::config_store::get("nsd_weather_city") {
            if let Ok(city) = serde_json::from_value::<CityInfo>(city_json) {
                state.city = Some(city.clone());
                *LATEST_CITY.lock().unwrap_or_else(|e| e.into_inner()) = Some(city);
            }
        }

        loop {
            let poll_secs = WEATHER_POLL_INTERVAL_SECS.load(Ordering::Relaxed)
                .clamp(MIN_POLL_INTERVAL_SECS, MAX_POLL_INTERVAL_SECS);
            let due_threshold = poll_secs * 9 / 10;

            // 0) 城市同步：weather_set_city 命令或开发者桥接改城市后立即生效。
            //    旧实现只写 config，运行中的线程仍用启动时的城市，改城市必须重启才生效。
            {
                let desired = current_city();
                let changed = match (&state.city, &desired) {
                    (Some(a), Some(b)) => {
                        a.city_id != b.city_id || a.lat != b.lat || a.lon != b.lon
                    }
                    (None, None) => false,
                    _ => true,
                };
                if changed {
                    state.city = desired.clone();
                    *LATEST_CITY.lock().unwrap_or_else(|e| e.into_inner()) = desired;
                    // 换城市后旧快照不再可比：清掉 diff 基准与已见预警，避免新旧城市对比出假边沿
                    state.last = None;
                    state.last_alert_ids.clear();
                    state.dirty.store(true, Ordering::Relaxed);
                }
            }

            // 1) 早午晚报窗口判定
            if let Some(city) = state.city.as_ref() {
                if let Some(brief) = detect_brief_window() {
                    if is_brief_enabled(brief) && !brief_already_sent(brief) {
                        if let Ok(snap) = rt.block_on(fetch_weather(city)) {
                            let (title, body) = brief_text(brief, Some(&snap));
                            push_brief(&app, brief, &title, &body);
                            play_brief_sound();
                            mark_brief_sent(brief);
                        }
                    }
                }
            }

            // 2) 整点/重算触发拉取
            let now = unix_now();
            let last = state.last_fetch_at.load(Ordering::Relaxed);
            let dirty = state.dirty.swap(false, Ordering::Relaxed) || FORCE_FETCH.swap(false, Ordering::Relaxed);
            let due = last == 0 || now.saturating_sub(last) >= due_threshold;
            if dirty || due {
                if let Some(city) = state.city.as_ref() {
                    match rt.block_on(fetch_weather(city)) {
                        Ok(snap) => {
                            // diff 边沿触发恶劣天气
                            if let Some(prev) = state.last.as_ref() {
                                let threshold = WEATHER_ALERT_THRESHOLD.load(Ordering::Relaxed);
                                let triggers = detect_severe_changes(prev, &snap, &state.last_alert_ids, threshold);
                                if !triggers.is_empty() {
                                    for t in &triggers {
                                        push_severe(&app, t);
                                    }
                                    play_severe_sound();
                                }
                                // 轻提示
                                if WEATHER_LIGHT_ALERT_ENABLED.load(Ordering::Relaxed) {
                                    let new_light: Vec<_> = snap.alerts.iter()
                                        .filter(|a| level_rank(&a.level) < threshold
                                            && !state.last_alert_ids.contains(&a.alert_id))
                                        .collect();
                                    if let Some(top) = new_light.iter().max_by_key(|a| level_rank(&a.level)) {
                                        push_light_alert(&app, top);
                                    }
                                }
                            }
                            state.last_alert_ids = snap.alerts.iter().map(|a| a.alert_id.clone()).collect();
                            state.last = Some(snap.clone());
                            // 写入共享快照：weather_get_state / weather-tick 从这里取数
                            *LATEST_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = Some(snap);
                            state.fail_streak = 0;
                            state.last_fetch_at.store(now, Ordering::Relaxed);
                            LATEST_FETCH_AT.store(now, Ordering::Relaxed);
                            LATEST_FAIL_STREAK.store(0, Ordering::Relaxed);
                            emit_tick(&app);
                        }
                        Err(_) => {
                            state.fail_streak = state.fail_streak.saturating_add(1);
                            LATEST_FAIL_STREAK.store(state.fail_streak, Ordering::Relaxed);
                            if state.fail_streak == SEVERE_MAX_FAIL {
                                push_severe_unavailable(&app);
                                play_severe_sound();
                            }
                        }
                    }
                }
            }

            // 3) 唤醒间隔
            //    测试版额外收紧"无城市"空转间隔：城市可能是运行中才配上的，
            //    若仍按 poll_secs/4（最长 45min）空转，配完城市后要等很久才开始轮询。
            let sleep = if state.city.is_some() {
                IDLE_WAKE_SECS
            } else if crate::dev_bridge::DEV_BUILD {
                60
            } else {
                poll_secs / 4
            };
            if exit.sleep_interruptible(Duration::from_secs(sleep)) { return; }
        }
    });
}

// ──────────────────────────────────────────────
// Tauri Commands
// ──────────────────────────────────────────────

#[tauri::command]
pub async fn weather_search_city(kw: String) -> Result<Vec<CityInfo>, String> {
    let url = format!("{}/location/city/search?name={}&locale=zh_cn", BASE_URL, urlencoding::encode(&kw));
    let body: serde_json::Value = reqwest::Client::new()
        .get(&url).timeout(Duration::from_secs(5))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    Ok(parse_cities(&body))
}

#[tauri::command]
pub fn weather_set_city(app: AppHandle, city: CityInfo) -> Result<(), String> {
    crate::config_store::set(&app, "nsd_weather_city".to_string(), serde_json::to_value(&city).map_err(|e| e.to_string())?)?;
    // 写入共享城市：天气线程在下个循环开头同步并立即触发拉取（无需重启即生效）
    *LATEST_CITY.lock().unwrap_or_else(|e| e.into_inner()) = Some(city);
    FORCE_FETCH.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn weather_get_state(_app: AppHandle) -> serde_json::Value {
    let (current, today, tomorrow, alerts) = match effective_snapshot() {
        Some(s) => (s.current, s.today, s.tomorrow, s.alerts),
        None => (None, None, None, Vec::new()),
    };
    serde_json::json!({
        "city": current_city(),
        "current": current,
        "today": today,
        "tomorrow": tomorrow,
        "alerts": alerts,
        "lastFetchAt": LATEST_FETCH_AT.load(Ordering::Relaxed),
        "injected": INJECTED_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).is_some(),
    })
}

#[tauri::command]
pub fn weather_set_daily_brief(app: AppHandle, brief: String) -> Result<(), String> {
    crate::config_store::set(&app, "nsd_weather_daily_brief".to_string(), serde_json::Value::String(brief))?;
    Ok(())
}

#[tauri::command]
pub fn weather_set_alert_threshold(app: AppHandle, level: String) -> Result<(), String> {
    let rank = match level.as_str() {
        "B" => 0, "Y" => 1, "O" => 2, "R" => 3,
        _ => return Err("alert level must be B/Y/O/R".into()),
    };
    WEATHER_ALERT_THRESHOLD.store(rank, Ordering::Relaxed);
    crate::config_store::set(&app, "nsd_weather_alert_threshold".to_string(), serde_json::Value::String(level))?;
    Ok(())
}

#[tauri::command]
pub fn weather_set_poll_interval(app: AppHandle, secs: u64) -> Result<(), String> {
    let clamped = if !(MIN_POLL_INTERVAL_SECS..=MAX_POLL_INTERVAL_SECS).contains(&secs) {
        DEFAULT_POLL_INTERVAL_SECS
    } else {
        secs
    };
    WEATHER_POLL_INTERVAL_SECS.store(clamped, Ordering::Relaxed);
    crate::config_store::set(&app, "nsd_weather_poll_interval".to_string(), serde_json::Value::Number(clamped.into()))?;
    Ok(())
}

#[tauri::command]
pub fn weather_set_light_alert_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    WEATHER_LIGHT_ALERT_ENABLED.store(enabled, Ordering::Relaxed);
    crate::config_store::set(
        &app,
        "nsd_weather_light_alert_enabled".to_string(),
        serde_json::Value::String(if enabled { "true".into() } else { "false".into() }),
    )?;
    Ok(())
}

// ══════════════════════════════════════════════
// 开发者桥接接口（仅供 dev_bridge 调用）
//
// 恶劣天气依赖真实天象，日程依赖真实日历，任务栏进度依赖真实下载 —— 这三者都
// 无法稳定复现，因此这里把"注入点"集中暴露出来：可以绕过网络直接送快照、
// 直推任意等级的恶劣天气事件、覆盖本地时间以进入早/午/晚报窗口、
// 以及用真实 detect_severe_changes 跑一遍边沿触发判定。
// ══════════════════════════════════════════════

/// 直推一条恶劣天气事件（走真实 push_severe + 提示音，等价于线上边沿触发后的表现）
pub(crate) fn dev_force_severe(app: &AppHandle, icon: &str, title: &str, body: &str, severity: &str) {
    let ev = SevereEvent {
        kind: SevereKind::TempGap, // 直推路径不消费 kind，载荷由下面四个字段决定
        icon: icon.to_string(),
        title: title.to_string(),
        body: body.to_string(),
        severity: severity.to_string(),
        dedup_key: format!("dev-severe:{}", unix_now()),
    };
    push_severe(app, &ev);
    play_severe_sound();
}

/// 直推一条早/午/晚报（title/body 为 None 时按当前快照自动生成文案）
pub(crate) fn dev_force_brief(
    app: &AppHandle,
    brief: &str,
    title: Option<String>,
    body: Option<String>,
) {
    let snap = effective_snapshot();
    let (auto_title, auto_body) = brief_text(brief, snap.as_ref());
    push_brief(
        app,
        brief,
        title.as_deref().unwrap_or(&auto_title),
        body.as_deref().unwrap_or(&auto_body),
    );
    play_brief_sound();
}

/// 直推一条低等级预警轻提示
pub(crate) fn dev_force_light_alert(app: &AppHandle, alert: &AlertInfo) {
    push_light_alert(app, alert);
}

/// 直推"天气服务不可用"
pub(crate) fn dev_force_unavailable(app: &AppHandle) {
    push_severe_unavailable(app);
    play_severe_sound();
}

/// 覆盖本地分钟数（模拟早/午/晚报时段）；None = 恢复真实本地时间
pub(crate) fn dev_set_brief_minutes(minutes: Option<u32>) {
    match minutes {
        Some(m) => BRIEF_MINUTES_OVERRIDE.store(m.min(24 * 60 - 1) as i32, Ordering::Relaxed),
        None => BRIEF_MINUTES_OVERRIDE.store(-1, Ordering::Relaxed),
    }
}

/// 清空"今天已推送过某报"的记录，便于同一天内反复测试早/午/晚报
pub(crate) fn dev_reset_brief() {
    LAST_BRIEF_DATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

/// 按后台线程的同一条判定链跑一次早/午/晚报（窗口 → 开关 → 当天去重 → 推送），
/// 返回每一步的判定结果，便于定位"到底是没到点、被关了，还是被去重挡了"。
pub(crate) fn dev_run_brief(app: &AppHandle) -> serde_json::Value {
    let window = match detect_brief_window() {
        Some(w) => w,
        None => {
            return serde_json::json!({
                "window": serde_json::Value::Null,
                "fired": false,
                "reason": "当前不在早/午/晚报时间窗口内（早 07:00-10:59 / 午 11:00-14:59 / 晚 18:00-20:59）",
            })
        }
    };
    if !is_brief_enabled(window) {
        return serde_json::json!({ "window": window, "fired": false, "reason": "该时段报道已在设置中关闭" });
    }
    if brief_already_sent(window) {
        return serde_json::json!({
            "window": window,
            "fired": false,
            "reason": "今天已推送过该报；可先执行 weather.reset_brief 清空去重记录",
        });
    }
    let snap = effective_snapshot();
    let (title, body) = brief_text(window, snap.as_ref());
    push_brief(app, window, &title, &body);
    play_brief_sound();
    mark_brief_sent(window);
    serde_json::json!({ "window": window, "fired": true, "title": title, "body": body })
}

/// 注入一份伪造的快照（绕过网络），注入期间 tick 一律用它
pub(crate) fn dev_inject(app: &AppHandle, snap: WeatherSnapshot) -> serde_json::Value {
    *INJECTED_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = Some(snap);
    emit_tick(app);
    tick_payload()
}

/// 取消注入，回到真实拉取数据
pub(crate) fn dev_clear_inject(app: &AppHandle) -> serde_json::Value {
    *INJECTED_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    emit_tick(app);
    tick_payload()
}

/// 立即同步拉取一次（不等后台线程的唤醒周期）；city 为 None 时用当前城市
pub(crate) fn dev_fetch_now(app: &AppHandle, city: Option<CityInfo>) -> Result<serde_json::Value, String> {
    let city = match city.or_else(current_city) {
        Some(c) => c,
        None => return Err("尚未配置城市，无法拉取".to_string()),
    };
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    match rt.block_on(fetch_weather(&city)) {
        Ok(snap) => {
            *LATEST_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = Some(snap);
            LATEST_FETCH_AT.store(unix_now(), Ordering::Relaxed);
            LATEST_FAIL_STREAK.store(0, Ordering::Relaxed);
            emit_tick(app);
            Ok(tick_payload())
        }
        Err(e) => {
            LATEST_FAIL_STREAK.store(
                LATEST_FAIL_STREAK.load(Ordering::Relaxed).saturating_add(1),
                Ordering::Relaxed,
            );
            Err(e)
        }
    }
}

/// 桥接：改城市（落盘 + 写共享城市 + 立即拉取）
pub(crate) fn dev_set_city(app: &AppHandle, city: CityInfo) -> Result<serde_json::Value, String> {
    let value = serde_json::to_value(&city).map_err(|e| e.to_string())?;
    crate::config_store::set(app, "nsd_weather_city".to_string(), value)?;
    *LATEST_CITY.lock().unwrap_or_else(|e| e.into_inner()) = Some(city.clone());
    FORCE_FETCH.store(true, Ordering::Relaxed);
    dev_fetch_now(app, Some(city))
}

/// 预设快照：给边沿触发模拟用（from / to 两个状态之间的迁移）
fn preset_snapshot(kind: &str) -> WeatherSnapshot {
    // (天气码, AQI, 气温, 体感)
    let (code, aqi, temp, feels) = match kind {
        "rain" => (7, 40.0, 22.0, 21.0),
        "storm" => (10, 45.0, 21.0, 20.0),
        "snow" => (15, 30.0, -3.0, -8.0),
        "fog" => (18, 60.0, 18.0, 18.0),
        "haze" => (2, 180.0, 20.0, 20.0),
        "tempgap" => (0, 30.0, 30.0, 22.0),
        // "sunny" 及未知一律按晴天处理（其余判定项全部不触发）
        _ => (0, 30.0, 25.0, 25.0),
    };
    let alerts = if kind.starts_with("alert") {
        let (level, text) = if kind.contains("red") {
            ("R", "红色预警")
        } else if kind.contains("orange") {
            ("O", "橙色预警")
        } else {
            ("Y", "黄色预警")
        };
        vec![AlertInfo {
            alert_id: format!("dev-alert-{}-{}", level, unix_now()),
            type_name: "暴雨".to_string(),
            level: level.to_string(),
            level_text: text.to_string(),
            title: "开发者工具注入的测试预警".to_string(),
            pub_time: None,
        }]
    } else {
        vec![]
    };
    WeatherSnapshot {
        current: Some(CurrentWeather {
            temperature: temp,
            weather_code: code,
            weather_text: weather_code_to_text(code).to_string(),
            feels_like: feels,
            humidity: 60.0,
            wind_speed: 3.0,
            pm25: Some(aqi / 2.0),
            aqi: Some(aqi),
        }),
        today: Some(DailyForecast {
            temp_max: temp + 4.0,
            temp_min: temp - 5.0,
            day_code: code,
            night_code: code,
            precip_prob: 30.0,
            sunrise: Some("06:00".to_string()),
            sunset: Some("18:30".to_string()),
        }),
        tomorrow: None,
        alerts,
    }
}

/// 用真实 detect_severe_changes 跑一遍 from → to 的边沿判定，并推送命中事件。
/// 这样测的是线上那条判定逻辑本身，而不是绕过它直推载荷。
pub(crate) fn dev_simulate_edge(app: &AppHandle, from: &str, to: &str) -> serde_json::Value {
    let prev = preset_snapshot(from);
    let cur = preset_snapshot(to);
    let threshold = WEATHER_ALERT_THRESHOLD.load(Ordering::Relaxed);
    // 用空集合充当"此前没见过任何预警"，让预警类事件在单次模拟里也能触发
    let seen: HashSet<String> = HashSet::new();
    let events = detect_severe_changes(&prev, &cur, &seen, threshold);

    let mut pushed = Vec::new();
    for e in &events {
        push_severe(app, e);
        pushed.push(serde_json::json!({
            "icon": e.icon,
            "title": e.title,
            "body": e.body,
            "severity": e.severity,
            "dedupKey": e.dedup_key,
        }));
    }
    if !events.is_empty() {
        play_severe_sound();
    }
    serde_json::json!({ "from": from, "to": to, "count": events.len(), "events": pushed })
}

/// 桥接用状态快照
pub(crate) fn dev_status() -> serde_json::Value {
    let injected = INJECTED_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).is_some();
    let override_minutes = BRIEF_MINUTES_OVERRIDE.load(Ordering::Relaxed);
    // json! 里不放块表达式（避免与嵌套对象字面量歧义），先算好再塞进去
    let override_value = if override_minutes >= 0 {
        serde_json::json!(override_minutes)
    } else {
        serde_json::Value::Null
    };
    let local_minutes: u32 = if override_minutes >= 0 {
        override_minutes as u32
    } else {
        let ts = unix_now() as i64 + 8 * 3600;
        ((ts % 86_400) / 60) as u32
    };
    serde_json::json!({
        "city": current_city(),
        "hasSnapshot": effective_snapshot().is_some(),
        "injected": injected,
        "lastFetchAt": LATEST_FETCH_AT.load(Ordering::Relaxed),
        "failStreak": LATEST_FAIL_STREAK.load(Ordering::Relaxed),
        "pollIntervalSecs": WEATHER_POLL_INTERVAL_SECS.load(Ordering::Relaxed),
        "alertThreshold": WEATHER_ALERT_THRESHOLD.load(Ordering::Relaxed),
        "lightAlertEnabled": WEATHER_LIGHT_ALERT_ENABLED.load(Ordering::Relaxed),
        "briefMinutesOverride": override_value,
        "localMinutes": local_minutes,
        "briefWindow": detect_brief_window(),
        "todayKey": today_key(),
    })
}
