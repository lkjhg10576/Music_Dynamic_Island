//! 恶劣天气提醒（Weather Reminder）后端模块
//!
//! 数据源：小米天气 wtr-v3（无需 Key、固定 sign）；每小时自动拉取；
//! 恶劣天气边沿触发灵动岛通知 + 用户可选的早/午/晚报。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// 常量
// ──────────────────────────────────────────────

const DEFAULT_POLL_INTERVAL_SECS: u64 = 3600;   // 默认 1 小时
const MIN_POLL_INTERVAL_SECS: u64 = 1800;       // 0.5h 下限
const MAX_POLL_INTERVAL_SECS: u64 = 10800;      // 3h 上限
const IDLE_WAKE_SECS: u64 = 60;                // 空闲时 1min 唤醒（用于判定早午晚报）
const SEVERE_RETRY_SECS: u64 = 30;
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
pub struct CityInfo {
    pub city_id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub province: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertInfo {
    pub alert_id: String,
    pub type_name: String,
    pub level: String,       // 'B' | 'Y' | 'O' | 'R' | 'W'
    pub level_text: String,  // '蓝色预警'
    pub title: String,
    pub pub_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSnapshot {
    pub current: Option<CurrentWeather>,
    pub today: Option<DailyForecast>,
    pub tomorrow: Option<DailyForecast>,
    pub alerts: Vec<AlertInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    current: Option<WeatherSnapshot>,
    last_alert_ids: HashSet<String>,
    last_brief_dates: HashMap<String, String>, // key='morning' value='2026-09-08'
    fail_streak: u32,
    last_fetch_at: AtomicU64,
    dirty: AtomicBool,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            city: None,
            last: None,
            current: None,
            last_alert_ids: HashSet::new(),
            last_brief_dates: HashMap::new(),
            fail_streak: 0,
            last_fetch_at: AtomicU64::new(0),
            dirty: AtomicBool::new(false),
        }
    }
}

static WEATHER_STATE: Mutex<Option<WeatherState>> = Mutex::new(None);

fn get_state() -> std::sync::MutexGuard<'static, Option<WeatherState>> {
    WEATHER_STATE.lock().unwrap_or_else(|e| e.into_inner())
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

fn today_key() -> String {
    // 使用本地时区（与 calendar.rs 同源）
    let now = unix_now() as i64;
    // 简化：使用 UTC+8（中国时区）
    let ts = now + 8 * 3600;
    let days = ts / 86400;
    let year = 1970 + days / 365; // 近似
    let rem = days - (year - 1970) * 365;
    let month = rem / 30 + 1;
    let day = rem % 30 + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn local_minutes() -> u32 {
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
        let code = cur.current.map(|c| c.weather_code).unwrap_or(0);
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
        prev.current.and_then(|c| c.aqi).unwrap_or(0.0) >= 150.0;
    let cur_fog = cur_codes.iter().any(|&c| is_fog(c)) ||
        cur.current.and_then(|c| c.aqi).unwrap_or(0.0) >= 150.0;
    if cur_fog && !prev_fog {
        let code = cur.current.map(|c| c.weather_code).unwrap_or(0);
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

fn emit_tick(app: &AppHandle, state: &WeatherState) {
    let payload = serde_json::json!({
        "city": state.city,
        "current": state.current.as_ref().and_then(|s| s.current.as_ref()),
        "today": state.current.as_ref().and_then(|s| s.today.as_ref()),
        "tomorrow": state.current.as_ref().and_then(|s| s.tomorrow.as_ref()),
        "alerts": state.current.as_ref().map(|s| &s.alerts).unwrap_or(&vec![]),
        "lastFetchAt": state.last_fetch_at.load(Ordering::Relaxed),
        "fetchStatus": if state.fail_streak > 0 { "failed" } else { "ok" },
    });
    crate::win32_utils::log_err(app.emit("weather-tick", payload), "emit weather-tick");
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

fn is_brief_enabled(brief: &str) -> bool {
    // TODO: 从 config_store 读取 NSD_WEATHER_DAILY_BRIEF
    true // 简化实现
}

// ──────────────────────────────────────────────
// 主循环
// ──────────────────────────────────────────────

pub fn start_weather_thread(app: AppHandle) {
    crate::thread_mgr::spawn_managed("weather_poll", move |exit| {
        let mut state = WeatherState::default();

        // 尝试恢复城市配置
        if let Some(city_json) = crate::config_store::get(&app, "nsd_weather_city") {
            if let Ok(city) = serde_json::from_value::<CityInfo>(city_json) {
                state.city = Some(city);
            }
        }

        loop {
            let poll_secs = WEATHER_POLL_INTERVAL_SECS.load(Ordering::Relaxed)
                .clamp(MIN_POLL_INTERVAL_SECS, MAX_POLL_INTERVAL_SECS);
            let due_threshold = poll_secs * 9 / 10;

            // 1) 早午晚报窗口判定
            if let Some(city) = state.city.as_ref() {
                if let Some(brief) = detect_brief_window() {
                    if is_brief_enabled(brief) && state.last_brief_dates.get(brief) != Some(&today_key()) {
                        match fetch_weather(city).await {
                            Ok(snap) => {
                                let title = match brief {
                                    "morning" => "早上好，今天风和日丽",
                                    "noon" => "中午好，今天风和日丽",
                                    "evening" => "晚上好，今天风和日丽",
                                    _ => "今日天气",
                                };
                                push_brief(&app, brief, title, "数据加载中");
                                play_brief_sound();
                                state.last_brief_dates.insert(brief.to_string(), today_key());
                            }
                            Err(_) => {}
                        }
                    }
                }
            }

            // 2) 整点/重算触发拉取
            let now = unix_now();
            let last = state.last_fetch_at.load(Ordering::Relaxed);
            let dirty = state.dirty.swap(false, Ordering::Relaxed);
            let due = last == 0 || now.saturating_sub(last) >= due_threshold;
            if dirty || due {
                if let Some(city) = state.city.as_ref() {
                    match fetch_weather(city).await {
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
                            state.current = Some(snap);
                            state.fail_streak = 0;
                            state.last_fetch_at.store(now, Ordering::Relaxed);
                            emit_tick(&app, &state);
                        }
                        Err(_) => {
                            state.fail_streak = state.fail_streak.saturating_add(1);
                            if state.fail_streak == SEVERE_MAX_FAIL {
                                push_severe_unavailable(&app);
                                play_severe_sound();
                            }
                        }
                    }
                }
            }

            // 3) 唤醒间隔
            let sleep = if state.city.is_some() { IDLE_WAKE_SECS } else { poll_secs / 4 };
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
    crate::config_store::set(&app, "nsd_weather_city".to_string(), serde_json::to_value(&city)?)?;
    // 标记 dirty，线程下轮立即拉
    Ok(())
}

#[tauri::command]
pub fn weather_get_state(app: AppHandle) -> serde_json::Value {
    let state = get_state();
    serde_json::json!({
        "city": state.as_ref().and_then(|s| s.city.clone()),
        "lastFetchAt": state.as_ref().map(|s| s.last_fetch_at.load(Ordering::Relaxed)).unwrap_or(0),
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
