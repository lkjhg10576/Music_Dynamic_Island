//! 开发者桥接（MDI Developer Bridge）—— 仅 9.9.9-* 测试版启用
//!
//! 恶劣天气依赖真实天象、日程依赖真实日历、任务栏进度依赖真实下载，三者都难以
//! 稳定复现。本模块在 127.0.0.1 上开一个「换行分隔 JSON」的 TCP 端口，让外部
//! 开发者工具（MDI_Developer_Tools）可以直接注入这三类活动的输入，并实时收到
//! MDI 抛出的相关事件，从而把"能不能测"变成"点一下就测"。
//!
//! # 门禁
//!
//! [`DEV_BUILD`] 由**编译期**的 `CARGO_PKG_VERSION` 推导：只有版本号以 `9.9.9`
//! 开头的构建才会启动监听。正式版（0.6.x 等）编译出的二进制该常量为 `false`，
//! 即使本文件被误合进正式分支，也永远不会打开端口。
//!
//! # 协议
//!
//! 请求（一行一个 JSON 对象）：
//! ```text
//! {"id":1,"cmd":"weather.force_severe","args":{"icon":"rain","title":"即将有暴雨"}}
//! ```
//! 应答（一行一个 JSON 对象，与请求 `id` 对应）：
//! ```text
//! {"id":1,"ok":true,"data":{...}}
//! {"id":1,"ok":false,"error":"..."}
//! ```
//! 事件推送（无 `id`，由 MDI 主动下发）：
//! ```text
//! {"event":"weather-toast","seq":3,"ts":1757500000,"payload":{...}}
//! ```

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::{AppHandle, Listener};

/// 桥接监听端口。仅绑 127.0.0.1：回环地址不触发 Windows 防火墙授权弹窗。
pub(crate) const BRIDGE_PORT: u16 = 17999;

/// 编译期版本门禁。
///
/// `matches!` 在字节切片上做常量模式匹配，整个表达式在编译期求值。版本号不以
/// `9.9.9` 开头的构建恒为 `false`，因此正式版永远不可能启动桥接。
pub(crate) const DEV_BUILD: bool = matches!(
    env!("CARGO_PKG_VERSION").as_bytes(),
    [b'9', b'.', b'9', b'.', b'9', ..]
);

/// 转发给开发者工具的 Tauri 事件（消息名与前端消费的完全一致）
const FORWARDED_EVENTS: [&str; 5] = [
    "weather-tick",
    "weather-toast",
    "weather-light-alert",
    "calendar-tick",
    "taskbar-progress-tick",
];

/// 已连接的开发者工具：地址（用于断连清理）+ 写端句柄
static CLIENTS: Mutex<Vec<(SocketAddr, TcpStream)>> = Mutex::new(Vec::new());
/// 事件序号：工具侧可据此判断是否有丢包
static EVENT_SEQ: AtomicU64 = AtomicU64::new(0);

/// 支持的指令清单（`help` 返回给工具，便于工具侧自检）
const COMMANDS: &[&str] = &[
    "ping",
    "help",
    "status",
    // 天气
    "weather.status",
    "weather.set_city",
    "weather.fetch_now",
    "weather.inject",
    "weather.clear_inject",
    "weather.force_severe",
    "weather.force_brief",
    "weather.force_light_alert",
    "weather.force_unavailable",
    "weather.simulate_edge",
    "weather.set_brief_minutes",
    "weather.reset_brief",
    "weather.run_brief",
    "weather.set_threshold",
    "weather.set_light_alert_enabled",
    "weather.set_poll_interval",
    // 日程
    "calendar.status",
    "calendar.add",
    "calendar.remove",
    "calendar.clear",
    "calendar.force_emit",
    "calendar.drop_system",
    // 任务栏进度
    "taskbar.status",
    "taskbar.inject",
    "taskbar.clear",
    "taskbar.set_enabled",
    "taskbar.set_interval",
];

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ──────────────────────────────────────────────
// 参数取值助手（一律给默认值，缺参不报错）
// ──────────────────────────────────────────────

fn s_arg(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn s_arg_or(args: &Value, key: &str, default: &str) -> String {
    let v = s_arg(args, key);
    if v.is_empty() {
        default.to_string()
    } else {
        v
    }
}

fn f_arg(args: &Value, key: &str, default: f64) -> f64 {
    args.get(key)
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(default)
}

fn u_arg(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key)
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_f64().map(|f| f.max(0.0) as u64))
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(default)
}

fn i_arg(args: &Value, key: &str, default: i64) -> i64 {
    args.get(key)
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_f64().map(|f| f as i64))
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(default)
}

fn b_arg(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

// ──────────────────────────────────────────────
// 广播 / 写行
// ──────────────────────────────────────────────

/// 把一条事件广播给所有已连接的开发者工具；写失败的连接就地剔除
fn broadcast(name: &str, payload: Value) {
    let seq = EVENT_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let msg = json!({
        "event": name,
        "seq": seq,
        "ts": now_secs(),
        "payload": payload,
    });
    let mut line = match serde_json::to_string(&msg) {
        Ok(s) => s,
        Err(_) => return,
    };
    line.push('\n');
    let bytes = line.as_bytes();

    let mut clients = CLIENTS.lock().unwrap_or_else(|e| e.into_inner());
    let mut alive: Vec<(SocketAddr, TcpStream)> = Vec::with_capacity(clients.len());
    for (addr, mut sock) in clients.drain(..) {
        if sock.write_all(bytes).is_ok() {
            alive.push((addr, sock));
        }
    }
    *clients = alive;
}

fn write_line(sock: &mut TcpStream, value: &Value) -> std::io::Result<()> {
    let mut line = serde_json::to_string(value)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"应答序列化失败\"}".to_string());
    line.push('\n');
    sock.write_all(line.as_bytes())?;
    sock.flush()
}

// ──────────────────────────────────────────────
// 指令分发
// ──────────────────────────────────────────────

fn dispatch(app: &AppHandle, cmd: &str, args: &Value) -> Result<Value, String> {
    match cmd {
        "ping" => Ok(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "devBuild": DEV_BUILD,
            "bridgePort": BRIDGE_PORT,
            "pid": std::process::id(),
        })),
        "help" => Ok(json!({ "commands": COMMANDS })),
        "status" => Ok(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "weather": crate::weather::dev_status(),
            "calendar": crate::calendar::dev_status(app),
            "taskbar": crate::taskbar_progress::dev_status(),
        })),

        // ────────── 天气：恶劣天气提醒 ──────────
        "weather.status" => Ok(crate::weather::dev_status()),

        "weather.set_city" => {
            let city = crate::weather::CityInfo {
                city_id: s_arg(args, "cityId"),
                name: s_arg(args, "name"),
                lat: f_arg(args, "lat", 0.0),
                lon: f_arg(args, "lon", 0.0),
                province: args
                    .get("province")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };
            if city.city_id.is_empty() || city.name.is_empty() {
                return Err("cityId 与 name 均不能为空".to_string());
            }
            crate::weather::dev_set_city(app, city)
        }

        "weather.fetch_now" => crate::weather::dev_fetch_now(app, None),

        "weather.inject" => {
            // 快照字段名与 weather-tick 载荷一致（camelCase）
            let raw = args.get("snapshot").cloned().unwrap_or(Value::Null);
            let snap: crate::weather::WeatherSnapshot = serde_json::from_value(raw)
                .map_err(|e| format!("快照解析失败（字段需为 camelCase，如 weatherCode/feelsLike）: {e}"))?;
            Ok(crate::weather::dev_inject(app, snap))
        }

        "weather.clear_inject" => Ok(crate::weather::dev_clear_inject(app)),

        "weather.force_severe" => {
            let icon = s_arg_or(args, "icon", "alert");
            let severity = s_arg_or(args, "severity", "warn");
            let title = s_arg_or(args, "title", "开发者工具注入的恶劣天气提醒");
            let body = s_arg(args, "body");
            crate::weather::dev_force_severe(app, &icon, &title, &body, &severity);
            Ok(json!({ "pushed": true, "icon": icon, "title": title, "severity": severity }))
        }

        "weather.force_brief" => {
            let brief = s_arg_or(args, "brief", "morning");
            if !matches!(brief.as_str(), "morning" | "noon" | "evening") {
                return Err("brief 必须是 morning / noon / evening".to_string());
            }
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let body = args
                .get("body")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::weather::dev_force_brief(app, &brief, title, body);
            Ok(json!({ "pushed": true, "brief": brief }))
        }

        "weather.force_light_alert" => {
            let alert = crate::weather::AlertInfo {
                alert_id: format!("dev-light-{}", now_secs()),
                type_name: s_arg_or(args, "type", "暴雨"),
                level: s_arg_or(args, "level", "Y"),
                level_text: s_arg_or(args, "levelText", "黄色预警"),
                title: s_arg_or(args, "title", "开发者工具注入的低等级预警"),
                pub_time: None,
            };
            crate::weather::dev_force_light_alert(app, &alert);
            Ok(json!({ "pushed": true, "alert": alert }))
        }

        "weather.force_unavailable" => {
            crate::weather::dev_force_unavailable(app);
            Ok(json!({ "pushed": true }))
        }

        "weather.simulate_edge" => {
            let from = s_arg_or(args, "from", "sunny");
            let to = s_arg_or(args, "to", "storm");
            Ok(crate::weather::dev_simulate_edge(app, &from, &to))
        }

        "weather.set_brief_minutes" => {
            // 传数字 = 覆盖本地分钟数；传 null / 省略 = 恢复真实本地时间
            let m = args.get("minutes").and_then(|v| v.as_f64());
            crate::weather::dev_set_brief_minutes(m.map(|f| f.max(0.0) as u32));
            Ok(crate::weather::dev_status())
        }

        "weather.reset_brief" => {
            crate::weather::dev_reset_brief();
            Ok(json!({ "reset": true }))
        }

        "weather.run_brief" => Ok(crate::weather::dev_run_brief(app)),

        "weather.set_threshold" => {
            let level = s_arg_or(args, "level", "B");
            crate::weather::weather_set_alert_threshold(app.clone(), level)?;
            Ok(crate::weather::dev_status())
        }

        "weather.set_light_alert_enabled" => {
            crate::weather::weather_set_light_alert_enabled(app.clone(), b_arg(args, "enabled", true))?;
            Ok(crate::weather::dev_status())
        }

        "weather.set_poll_interval" => {
            crate::weather::weather_set_poll_interval(app.clone(), u_arg(args, "secs", 3600))?;
            Ok(crate::weather::dev_status())
        }

        // ────────── 日程同步 ──────────
        "calendar.status" => Ok(crate::calendar::dev_status(app)),

        "calendar.add" => {
            let title = s_arg_or(args, "title", "开发者工具测试日程");
            let delay = i_arg(args, "delaySecs", 60);
            let duration = u_arg(args, "durationMins", 5).max(1).min(u32::MAX as u64) as u32;
            let repeat_daily = b_arg(args, "repeatDaily", false);
            let id = crate::calendar::dev_add(app, title, delay, duration, repeat_daily);
            let mut payload = crate::calendar::dev_force_emit(app);
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("addedId".to_string(), json!(id));
            }
            Ok(payload)
        }

        "calendar.remove" => {
            let id = u_arg(args, "id", 0);
            if id == 0 {
                return Err("缺少 id".to_string());
            }
            Ok(crate::calendar::dev_remove(app, id))
        }

        "calendar.clear" => Ok(crate::calendar::dev_clear(app)),

        "calendar.force_emit" => Ok(crate::calendar::dev_force_emit(app)),

        "calendar.drop_system" => Ok(crate::calendar::dev_drop_system_events(app)),

        // ────────── 任务栏进度 ──────────
        "taskbar.status" => Ok(crate::taskbar_progress::dev_status()),

        "taskbar.inject" => {
            if !b_arg(args, "active", true) {
                crate::taskbar_progress::dev_set_override(app, None);
                return Ok(json!({ "injected": false }));
            }
            let percent = u_arg(args, "percent", 50).min(100) as u8;
            let state = crate::taskbar_progress::TaskbarProgressState {
                active: true,
                app_name: s_arg_or(args, "appName", "开发者工具"),
                percent,
                ts: now_secs(),
            };
            let echo = state.clone();
            crate::taskbar_progress::dev_set_override(app, Some(state));
            Ok(json!({ "injected": true, "state": echo }))
        }

        "taskbar.clear" => {
            crate::taskbar_progress::dev_set_override(app, None);
            Ok(json!({ "injected": false }))
        }

        "taskbar.set_enabled" => {
            crate::taskbar_progress::set_taskbar_progress_enabled(b_arg(args, "enabled", true));
            Ok(crate::taskbar_progress::dev_status())
        }

        "taskbar.set_interval" => {
            crate::taskbar_progress::set_taskbar_progress_interval(u_arg(args, "ms", 1000) as u32);
            Ok(crate::taskbar_progress::dev_status())
        }

        other => Err(format!("未知指令：{other}（可用 help 查看全部指令）")),
    }
}

// ──────────────────────────────────────────────
// 连接处理
// ──────────────────────────────────────────────

fn handle_client(app: AppHandle, stream: TcpStream) {
    let peer = stream
        .peer_addr()
        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 0)));

    // 同一条 TCP 连接需要三个句柄：读端、写应答端、广播端
    let read_sock = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut write_sock = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let _ = stream.set_nodelay(true);

    {
        let mut clients = CLIENTS.lock().unwrap_or_else(|e| e.into_inner());
        clients.push((peer, stream));
    }

    let reader = BufReader::new(read_sock);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let _ = write_line(
                    &mut write_sock,
                    &json!({ "id": Value::Null, "ok": false, "error": format!("JSON 解析失败: {e}") }),
                );
                continue;
            }
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let cmd = req.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
        let args = req
            .get("args")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let response = match dispatch(&app, cmd, &args) {
            Ok(data) => json!({ "id": id, "ok": true, "data": data }),
            Err(err) => json!({ "id": id, "ok": false, "error": err }),
        };
        if write_line(&mut write_sock, &response).is_err() {
            break;
        }
    }

    // 断连清理：广播时也会顺手剔除失效连接，这里即时移除更干净
    let mut clients = CLIENTS.lock().unwrap_or_else(|e| e.into_inner());
    clients.retain(|(addr, _)| *addr != peer);
}

// ──────────────────────────────────────────────
// 启动
// ──────────────────────────────────────────────

/// 启动开发者桥接（lib.rs setup 调用；非 9.9.9 版本直接返回，不开端口）
pub(crate) fn start(app: AppHandle) {
    if !DEV_BUILD {
        return;
    }

    // 事件转发：把三个活动的 Tauri 事件原样广播给已连接的开发者工具
    for name in FORWARDED_EVENTS {
        let _ = app.listen(name, move |event| {
            let payload = serde_json::from_str::<Value>(event.payload()).unwrap_or(Value::Null);
            broadcast(name, payload);
        });
    }

    std::thread::spawn(move || {
        let listener = match TcpListener::bind(("127.0.0.1", BRIDGE_PORT)) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[NSD][dev-bridge] 端口 {BRIDGE_PORT} 绑定失败：{e}");
                return;
            }
        };
        eprintln!(
            "[NSD][dev-bridge] 开发者桥接已就绪 127.0.0.1:{}（MDI {}）",
            BRIDGE_PORT,
            env!("CARGO_PKG_VERSION")
        );
        for incoming in listener.incoming() {
            match incoming {
                Ok(stream) => {
                    let conn_app = app.clone();
                    std::thread::spawn(move || handle_client(conn_app, stream));
                }
                Err(_) => continue,
            }
        }
    });
}
