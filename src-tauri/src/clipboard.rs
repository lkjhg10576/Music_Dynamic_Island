//! 剪贴板历史（plan-20260906 功能二）。
//!
//! - 监听：`GetClipboardSequenceNumber` 300ms 轮询（可中断休眠），不建消息窗口，
//!   与项目现有「轮询 + 可中断休眠」风格一致
//! - 采集：`OpenClipboard` → CF_UNICODETEXT / 注册格式 "PNG"（原生 PNG 字节流，
//!   零重编码）。只记录能粘回去的内容：无原生 PNG 或超 20MB 的图片一律不记录，
//!   不存任何占位条目
//! - 存储：`clipboard.json`（仅元数据）+ `clipboard_imgs/<id>.png` + `clipboard_thumbs/<id>.jpg`。
//!   历史数据**绝不**写 config.json（该文件每次变更全量广播，历史数据会拖垮双窗口）
//! - 自写豁免：`clipboard_copy_item` 写入产生的新 seq 记入跳过集合，监听线程命中即跳过，
//!   不产生新历史 / 不触发岛提示 / 不重排列表
//! - 配额（plan §4.6，代码常量）：总条目 100 / 图片 20 / 图片容量 100MB / 单图 20MB /
//!   未置顶保留 3 天；三约束相互独立按池淘汰，循环至达标
//! - 缩略图：WIC 解码 → 最长边 256（只缩小不放大，Fant 插值）→ JPEG 直写文件；
//!   点击复制时写回的是原样 PNG 字节流，缩略图只影响面板预览

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

use crate::storage::{app_data_dir, atomic_write, read_json};

// ===== 配额常量（plan §4.6，不暴露为设置项） =====
const MAX_ITEMS: usize = 100;
const MAX_IMAGES: usize = 20;
const MAX_IMAGE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_SINGLE_IMAGE: u64 = 20 * 1024 * 1024;
/// 未置顶条目保留时长（3 天），置顶豁免
const RETENTION_MS: u64 = 3 * 24 * 3600 * 1000;
/// 剪贴板变化轮询间隔
const POLL_MS: u64 = 300;
/// 缩略图最长边
const THUMB_MAX_EDGE: u32 = 256;

// ===== 数据结构 =====

#[derive(Serialize, Deserialize, Clone)]
pub struct ClipItem {
    /// 毫秒时间戳 + 进程内序号
    pub id: String,
    /// "text" | "image"
    pub kind: String,
    /// 复制时间
    pub ts_ms: u64,
    pub pinned: bool,
    /// 置顶操作时间（置顶区排序依据，未置顶为 0）
    pub pin_ts_ms: u64,
    /// kind=text 时的正文
    pub text: Option<String>,
    pub char_len: usize,
    /// 原图文件名 `<id>.png`（持久化存文件名；get_history 返回时拼接绝对路径）
    pub img_path: Option<String>,
    /// 缩略图文件名 `<id>.jpg`；None = 缩略图生成失败（条目仍保留，可粘贴）
    pub thumb_path: Option<String>,
    /// 图片尺寸（面板显示用）
    pub img_w: u32,
    pub img_h: u32,
    /// 原图体积（面板显示 + 配额统计用）
    pub img_bytes: u64,
}

// ===== 状态 =====

/// 历史缓存：None = 未从磁盘载入。持久化保持插入序（ts 升序），展示序在查询时排
static HISTORY: Lazy<Mutex<Option<Vec<ClipItem>>>> = Lazy::new(|| Mutex::new(None));
/// 监听线程运行标记（clipboard_set_enabled 幂等的依据）
static ENABLED: AtomicBool = AtomicBool::new(false);
/// 自写豁免跳过集合：clipboard_copy_item 写入后的新 seq；监听线程命中即跳过并清除
static SKIP_SEQS: Lazy<Mutex<VecDeque<u32>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
/// id 进程内序号
static ID_SEQ: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn new_id() -> String {
    format!("{}-{}", now_ms(), ID_SEQ.fetch_add(1, Ordering::Relaxed))
}

// ===== 目录与持久化 =====

fn imgs_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("clipboard_imgs");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建 clipboard_imgs 目录失败: {}", e))?;
    }
    Ok(dir)
}

fn thumbs_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("clipboard_thumbs");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建 clipboard_thumbs 目录失败: {}", e))?;
    }
    Ok(dir)
}

fn index_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("clipboard.json"))
}

/// 载入缓存（一次性初始化）并返回列表克隆；载入时执行一次孤儿清理 + 配额淘汰
fn load_history(app: &AppHandle) -> Vec<ClipItem> {
    let mut cached = HISTORY.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(list) = cached.as_ref() {
        return list.clone();
    }
    // 数据 0 信任：损坏一律视为空（重建），宁丢不坏
    let mut list: Vec<ClipItem> = index_path(app)
        .ok()
        .and_then(|p| read_json(&p))
        .unwrap_or_default();
    // 以复制时间升序为插入序，防手改文件导致顺序错乱
    list.sort_by_key(|i| i.ts_ms);
    *cached = Some(list.clone());
    drop(cached);

    // 清理时机（plan §4.6）：载入时一次（缓存初始化天然只发生一次）
    clean_orphans(app, &list);
    if enforce_quotas(app, &mut list) {
        persist_history(app, &list).ok();
    }
    list
}

fn persist_history(app: &AppHandle, list: &[ClipItem]) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(list).map_err(|e| format!("序列化剪贴板历史失败: {}", e))?;
    atomic_write(&index_path(app)?, &data)?;
    *HISTORY.lock().unwrap_or_else(|e| e.into_inner()) = Some(list.to_vec());
    Ok(())
}

// ===== 配额与淘汰（plan §4.6） =====

enum Pool {
    /// 全部条目（总条目超限用）
    All,
    /// 仅图片（图片数 / 图片容量超限用：淘汰文本对释放图片容量无效）
    Images,
}

/// 配额淘汰：时间 → 总条目 → 图片数 → 图片容量，循环至全部达标。返回列表是否有变化。
fn enforce_quotas(app: &AppHandle, list: &mut Vec<ClipItem>) -> bool {
    let mut changed = false;

    // 时间淘汰：仅未置顶，一次性移除全部超期（置顶豁免）
    let cutoff = now_ms().saturating_sub(RETENTION_MS);
    let before = list.len();
    list.retain(|i| i.pinned || i.ts_ms >= cutoff);
    if list.len() != before {
        changed = true;
    }

    // 循环淘汰：每轮删一条直到三项硬指标全部达标
    loop {
        let img_count = list.iter().filter(|i| i.kind == "image").count();
        let img_bytes: u64 = list.iter().filter(|i| i.kind == "image").map(|i| i.img_bytes).sum();
        if list.len() > MAX_ITEMS {
            evict_one(app, list, &Pool::All);
        } else if img_count > MAX_IMAGES || img_bytes > MAX_IMAGE_BYTES {
            evict_one(app, list, &Pool::Images);
        } else {
            break;
        }
        changed = true;
    }
    changed
}

/// 淘汰一条：按池与组别优先级选「最该删」的条目，组内最旧优先（ts 最小）。
/// 组别顺序（plan §4.6）：All = 未置顶图 → 未置顶文本 → 置顶图 → 置顶文本；
/// Images = 未置顶图 → 置顶图
fn evict_one(app: &AppHandle, list: &mut Vec<ClipItem>, pool: &Pool) {
    let rank = |i: &ClipItem| -> Option<(u8, u64)> {
        if matches!(pool, Pool::Images) && i.kind != "image" {
            return None;
        }
        let group = match (i.pinned, i.kind.as_str()) {
            (false, "image") => 0u8,
            (false, _) => 1,
            (true, "image") => 2,
            (true, _) => 3,
        };
        Some((group, i.ts_ms))
    };
    let victim = list
        .iter()
        .enumerate()
        .filter_map(|(idx, i)| rank(i).map(|r| (r, idx)))
        .min_by_key(|(r, _)| *r)
        .map(|(_, idx)| idx);
    if let Some(idx) = victim {
        let item = list.remove(idx);
        delete_item_files(app, &item);
    }
}

/// 淘汰条目时同步删除对应 .png / .jpg，防孤儿文件累积
fn delete_item_files(app: &AppHandle, item: &ClipItem) {
    if item.kind != "image" {
        return;
    }
    if let Ok(dir) = imgs_dir(app) {
        if let Err(e) = std::fs::remove_file(dir.join(format!("{}.png", item.id))) {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("[NSD][warn] 删除剪贴板原图失败 ({}): {}", item.id, e);
            }
        }
    }
    if let Ok(dir) = thumbs_dir(app) {
        if let Err(e) = std::fs::remove_file(dir.join(format!("{}.jpg", item.id))) {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("[NSD][warn] 删除剪贴板缩略图失败 ({}): {}", item.id, e);
            }
        }
    }
}

/// 启动扫描：删除目录中无主文件（clipboard.json 里没有对应 id），防孤儿累积
fn clean_orphans(app: &AppHandle, list: &[ClipItem]) {
    let known: HashSet<&str> = list.iter().map(|i| i.id.as_str()).collect();
    if let Ok(dir) = imgs_dir(app) {
        clean_dir_orphans(&dir, &known, ".png");
    }
    if let Ok(dir) = thumbs_dir(app) {
        clean_dir_orphans(&dir, &known, ".jpg");
    }
}

fn clean_dir_orphans(dir: &Path, known: &HashSet<&str>, ext: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else { continue };
        let Some(stem) = name_str.strip_suffix(ext) else { continue };
        if !known.contains(stem) {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                eprintln!("[NSD][warn] 清理剪贴板孤儿文件失败 ({}): {}", name_str, e);
            }
        }
    }
}

// ===== 监听线程（轮询 + 自写豁免 + 采集 + 归一） =====

fn current_seq() -> u32 {
    unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() }
}

/// 记录自写豁免 seq（上限 8 个防膨胀：写回后 300ms 内的轮询必然命中）
fn mark_skip_seq(seq: u32) {
    let mut q = SKIP_SEQS.lock().unwrap_or_else(|e| e.into_inner());
    q.push_back(seq);
    while q.len() > 8 {
        q.pop_front();
    }
}

/// 监听线程命中自写豁免 seq：跳过本条并清除
fn take_skip_seq(seq: u32) -> bool {
    let mut q = SKIP_SEQS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(pos) = q.iter().position(|&s| s == seq) {
        q.remove(pos);
        true
    } else {
        false
    }
}

fn start_listener_thread(app: AppHandle, exit: crate::thread_mgr::ExitFlag) {
    // 基线：线程启动时刻的 seq，避免把应用启动前已有的剪贴板内容当作新复制
    let mut last_seq = current_seq();
    let mut last_hourly = std::time::Instant::now();
    loop {
        if exit.is_exiting() {
            break;
        }
        if exit.sleep_interruptible(std::time::Duration::from_millis(POLL_MS)) {
            break;
        }
        // 每小时兜底清理一次：时间淘汰 + 配额（plan §4.6 清理时机）
        if last_hourly.elapsed() >= std::time::Duration::from_secs(3600) {
            last_hourly = std::time::Instant::now();
            let mut list = load_history(&app);
            if enforce_quotas(&app, &mut list) {
                persist_history(&app, &list).ok();
            }
        }
        let seq = current_seq();
        if seq == last_seq || seq == 0 {
            continue;
        }
        last_seq = seq;
        // 自写豁免：面板复制产生的新 seq，不算用户复制
        if take_skip_seq(seq) {
            continue;
        }
        match capture() {
            Some(Captured::Text(text)) => add_text_item(&app, text),
            Some(Captured::Image(png)) => add_image_item(&app, png),
            None => {}
        }
    }
    ENABLED.store(false, Ordering::SeqCst);
}

enum Captured {
    Text(String),
    Image(Vec<u8>),
}

// Win32 剪贴板标准格式常量（稳定 API 契约：CF_TEXT=1 / CF_UNICODETEXT=13），
// 手写规避 windows-sys 常量在 metadata 中的模块路径不确定性（可能落在 Ole 模块）
const CF_TEXT: u32 = 1;
const CF_UNICODETEXT: u32 = 13;
// Win32 稳定契约常量（同项目先例：win32_utils::SWP_POS_NO_ACTIVATE）
const GMEM_MOVEABLE: u32 = 0x0002;
const GENERIC_WRITE: u32 = 0x4000_0000;
const CP_ACP: u32 = 0;

/// 注册格式 "PNG"：同名格式全系统返回同一 id
fn register_png_format() -> u32 {
    let name: Vec<u16> = "PNG\0".encode_utf16().collect();
    unsafe { windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW(name.as_ptr()) }
}

/// 采集一轮剪贴板内容。文本优先（高频主路径），无文本且有原生 PNG 才走图片。
/// 全程不在 OpenClipboard 期间做任何慢操作（落盘 / 缩略图都在 CloseClipboard 之后）
fn capture() -> Option<Captured> {
    let png_fmt = register_png_format();
    unsafe {
        // OpenClipboard 失败重试 3 次（每次 10ms）：剪贴板可能被其他进程短暂占用；
        // 仍失败跳过本轮，绝不 panic、绝不长期持锁
        let mut opened = false;
        for _ in 0..3 {
            if windows_sys::Win32::System::DataExchange::OpenClipboard(std::ptr::null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if !opened {
            return None;
        }

        let captured = if windows_sys::Win32::System::DataExchange::IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
            read_text().map(Captured::Text)
        } else if png_fmt != 0 && windows_sys::Win32::System::DataExchange::IsClipboardFormatAvailable(png_fmt) != 0 {
            // 图片准入（两档，无中间态）：原生 PNG 且 ≤20MB 才记录；
            // 仅 CF_DIB（如 Photoshop）不记录
            read_png(png_fmt)
                .filter(|b| validate_png(b) && (b.len() as u64) <= MAX_SINGLE_IMAGE)
                .map(Captured::Image)
        } else {
            None
        };
        windows_sys::Win32::System::DataExchange::CloseClipboard();
        captured
    }
}

fn validate_png(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
}

/// 读取 CF_UNICODETEXT：NUL 结尾 UTF-16（GlobalSize 仅作上限，按 NUL 截断）
fn read_text() -> Option<String> {
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows_sys::Win32::System::DataExchange::GetClipboardData;

    unsafe {
        let h = GetClipboardData(CF_UNICODETEXT);
        if h.is_null() {
            return None;
        }
        let ptr = GlobalLock(h) as *const u16;
        if ptr.is_null() {
            return None;
        }
        let capacity = GlobalSize(h) as usize / 2;
        let mut len = 0usize;
        while len < capacity && *ptr.add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        GlobalUnlock(h);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

/// 读取注册格式 PNG 的原始字节流（原样拷贝，零重编码）
fn read_png(fmt: u32) -> Option<Vec<u8>> {
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows_sys::Win32::System::DataExchange::GetClipboardData;

    unsafe {
        let h = GetClipboardData(fmt);
        if h.is_null() {
            return None;
        }
        let ptr = GlobalLock(h) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr, GlobalSize(h) as usize).to_vec();
        GlobalUnlock(h);
        Some(bytes)
    }
}

// ===== 归一（去重 / 落盘 / 配额 / 广播） =====

fn add_text_item(app: &AppHandle, text: String) {
    let mut list = load_history(app);
    // 连续去重：与最新条目内容相同 → 只更新 ts_ms，不新增条目
    let mut dup = false;
    if let Some(latest) = list.last_mut() {
        if latest.kind == "text" && latest.text.as_deref() == Some(text.as_str()) {
            latest.ts_ms = now_ms();
            dup = true;
        }
    }
    if dup {
        normalize_and_persist(app, &mut list);
        return;
    }
    let id = new_id();
    let char_len = text.chars().count();
    let preview: String = text.chars().take(24).collect();
    list.push(ClipItem {
        id: id.clone(),
        kind: "text".into(),
        ts_ms: now_ms(),
        pinned: false,
        pin_ts_ms: 0,
        text: Some(text),
        char_len,
        img_path: None,
        thumb_path: None,
        img_w: 0,
        img_h: 0,
        img_bytes: 0,
    });
    normalize_and_persist(app, &mut list);
    emit_changed(app, &id, "text", &preview, char_len);
}

fn add_image_item(app: &AppHandle, png: Vec<u8>) {
    let mut list = load_history(app);
    // 连续去重（图片）：与最新条目字节一致 → 只更新 ts_ms，不新增条目
    let mut dup = false;
    if let Some(latest) = list.last() {
        if latest.kind == "image" && latest.img_bytes == png.len() as u64 {
            if let Ok(dir) = imgs_dir(app) {
                if let Ok(stored) = std::fs::read(dir.join(format!("{}.png", latest.id))) {
                    if stored == png {
                        dup = true;
                    }
                }
            }
        }
    }
    if dup {
        if let Some(latest) = list.last_mut() {
            latest.ts_ms = now_ms();
        }
        normalize_and_persist(app, &mut list);
        return;
    }

    let id = new_id();
    let (img_w, img_h, thumb_ok) = match (imgs_dir(app), thumbs_dir(app)) {
        (Ok(imgs), Ok(thumbs)) => {
            // 原图落盘：原样字节流，零重编码
            let img_path = imgs.join(format!("{}.png", id));
            if let Err(e) = atomic_write(&img_path, &png) {
                eprintln!("[NSD][warn] 保存剪贴板原图失败: {}", e);
                (0, 0, false)
            } else {
                let thumb_path = thumbs.join(format!("{}.jpg", id));
                make_thumbnail(&png, &thumb_path)
            }
        }
        _ => (0, 0, false),
    };

    let id_c = id.clone();
    list.push(ClipItem {
        id,
        kind: "image".into(),
        ts_ms: now_ms(),
        pinned: false,
        pin_ts_ms: 0,
        text: None,
        char_len: 0,
        img_path: Some(format!("{}.png", id_c)),
        thumb_path: if thumb_ok { Some(format!("{}.jpg", id_c)) } else { None },
        img_w,
        img_h,
        img_bytes: png.len() as u64,
    });
    // 面板预览文案：图片无正文，preview 固定为「图片」
    normalize_and_persist(app, &mut list);
    emit_changed(app, &id_c, "image", "图片", 0);
}

/// 新增条目后的归一：配额淘汰 + 落盘（清理时机：每次新增条目后）
fn normalize_and_persist(app: &AppHandle, list: &mut Vec<ClipItem>) {
    enforce_quotas(app, list);
    persist_history(app, list).ok();
}

/// 新条目广播：供灵动岛可选提示 + 控制台面板增量刷新
fn emit_changed(app: &AppHandle, id: &str, kind: &str, preview: &str, char_len: usize) {
    crate::win32_utils::log_err(
        app.emit(
            "clipboard-changed",
            serde_json::json!({ "id": id, "kind": kind, "preview": preview, "char_len": char_len }),
        ),
        "emit clipboard-changed",
    );
}

// ===== WIC 缩略图（plan §4.5） =====

/// PNG 字节流 → 最长边 ≤256 的 JPEG 缩略图（只缩小不放大，WICBitmapInterpolationModeFant）。
/// 返回 (原图宽, 原图高, 缩略图是否成功写出)。
/// 缩略图失败不丢条目：原图已落盘、可粘贴，仅面板预览降级为占位。
fn make_thumbnail(png: &[u8], thumb_path: &Path) -> (u32, u32, bool) {
    use windows::Win32::Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_WICPixelFormat24bppBGR, GUID_ContainerFormatJpeg, IWICImagingFactory,
        WICBitmapDitherTypeNone, WICBitmapEncoderNoCache, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
        WICDecodeMetadataCacheOnDemand,
    };
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};

    let _com = crate::win32_utils::ComGuard::new();

    let run = || -> windows::core::Result<(u32, u32, bool)> {
        unsafe {
            let factory: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

            // 输入：内存流 → 解码器 → 第 0 帧 → 原图尺寸
            let in_stream = factory.CreateStream()?;
            in_stream.InitializeFromMemory(png)?;
            let decoder = factory.CreateDecoderFromStream(&in_stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)?;
            let frame = decoder.GetFrame(0)?;
            let mut w = 0u32;
            let mut h = 0u32;
            frame.GetSize(&mut w, &mut h)?;
            if w == 0 || h == 0 {
                return Ok((0, 0, false));
            }

            // 按最长边缩到 256，保持宽高比；只缩小不放大（u64 中间量防 w*256 溢出）
            let (tw, th) = if w.max(h) > THUMB_MAX_EDGE {
                if w >= h {
                    (
                        THUMB_MAX_EDGE,
                        ((h as u64 * THUMB_MAX_EDGE as u64 / w as u64).max(1)) as u32,
                    )
                } else {
                    (
                        ((w as u64 * THUMB_MAX_EDGE as u64 / h as u64).max(1)) as u32,
                        THUMB_MAX_EDGE,
                    )
                }
            } else {
                (w, h)
            };

            // JPEG 只支持 24bppBGR：缩放（或原图）经格式转换后写入。
            // 两条分支各自调用，仅依赖接口层级 Param 转换（IWICBitmapScaler /
            // IWICBitmapFrameDecode → IWICBitmapSource）
            let conv = factory.CreateFormatConverter()?;
            if tw != w || th != h {
                let scaler = factory.CreateBitmapScaler()?;
                scaler.Initialize(&frame, tw, th, WICBitmapInterpolationModeFant)?;
                conv.Initialize(
                    &scaler,
                    &GUID_WICPixelFormat24bppBGR,
                    WICBitmapDitherTypeNone,
                    None,
                    0.0,
                    WICBitmapPaletteTypeCustom,
                )?;
            } else {
                conv.Initialize(
                    &frame,
                    &GUID_WICPixelFormat24bppBGR,
                    WICBitmapDitherTypeNone,
                    None,
                    0.0,
                    WICBitmapPaletteTypeCustom,
                )?;
            }

            // 输出：直写文件流，省一次内存回读
            let out_stream = factory.CreateStream()?;
            let path_w: Vec<u16> = thumb_path
                .as_os_str()
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            out_stream.InitializeFromFilename(windows::core::PCWSTR::from_raw(path_w.as_ptr()), GENERIC_WRITE)?;

            let encoder = factory.CreateEncoder(&GUID_ContainerFormatJpeg, std::ptr::null())?;
            encoder.Initialize(&out_stream, WICBitmapEncoderNoCache)?;
            let mut frame_enc = None;
            encoder.CreateNewFrame(&mut frame_enc, std::ptr::null_mut())?;
            let Some(frame_enc) = frame_enc else {
                return Ok((w, h, false));
            };
            frame_enc.Initialize(None)?;
            frame_enc.SetSize(tw, th)?;
            let mut pixel_format = GUID_WICPixelFormat24bppBGR;
            frame_enc.SetPixelFormat(&mut pixel_format)?;

            let stride = (tw * 3 + 3) / 4 * 4;
            let mut buf = vec![0u8; stride as usize * th as usize];
            // windows 0.58 的 WritePixels 签名为 (linecount, cbstride, &[u8])，prc 在该版元数据中不存在
            frame_enc.WritePixels(th, stride, &buf)?;
            frame_enc.Commit()?;
            encoder.Commit()?;
            Ok((w, h, true))
        }
    };

    match run() {
        Ok((w, h, ok)) => (w, h, ok),
        Err(e) => {
            eprintln!("[NSD][warn] 生成剪贴板缩略图失败: {}", e);
            (0, 0, false)
        }
    }
}

// ===== 命令组（plan §4.7，注册进 lib.rs 的 generate_handler!） =====

/// 启停监听线程（幂等，两窗口都会调用；靠 ENABLED 标记保证始终只有一个监听线程）
#[tauri::command]
pub fn clipboard_set_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        if ENABLED.load(Ordering::SeqCst) {
            return Ok(());
        }
        // 兜底：快速关/开时确保上一条监听线程已退出（线程可中断休眠，立即响应）
        crate::thread_mgr::stop_thread("clipboard_listener");
        ENABLED.store(true, Ordering::SeqCst);
        let handle = app.clone();
        crate::thread_mgr::spawn_managed("clipboard_listener", move |exit| {
            start_listener_thread(handle, exit)
        });
    } else {
        ENABLED.store(false, Ordering::SeqCst);
        crate::thread_mgr::stop_thread("clipboard_listener");
    }
    Ok(())
}

/// 全量列表（已按展示顺序排序：置顶区按置顶时间倒序，其余按复制时间倒序）。
/// img_path / thumb_path 返回时动态拼接绝对路径（持久化始终存文件名，目录迁移不受影响）
#[tauri::command]
pub fn clipboard_get_history(app: AppHandle) -> Vec<ClipItem> {
    let mut list = load_history(&app);
    list.sort_by(|a, b| {
        if a.pinned != b.pinned {
            if b.pinned { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
        } else if a.pinned {
            b.pin_ts_ms.cmp(&a.pin_ts_ms)
        } else {
            b.ts_ms.cmp(&a.ts_ms)
        }
    });
    if let (Ok(imgs), Ok(thumbs)) = (imgs_dir(&app), thumbs_dir(&app)) {
        for item in list.iter_mut() {
            if item.kind != "image" {
                continue;
            }
            item.img_path = Some(imgs.join(format!("{}.png", item.id)).to_string_lossy().to_string());
            if item.thumb_path.is_some() {
                item.thumb_path =
                    Some(thumbs.join(format!("{}.jpg", item.id)).to_string_lossy().to_string());
            }
        }
    }
    list
}

/// 写回剪贴板（带自写豁免）：只做「设为剪贴板内容」，不做自动 Ctrl+V 粘贴
#[tauri::command]
pub fn clipboard_copy_item(app: AppHandle, id: String) -> Result<(), String> {
    let list = load_history(&app);
    let item = list.iter().find(|i| i.id == id).ok_or("条目不存在")?;
    let seq_before = current_seq();
    write_to_clipboard(&app, item)?;
    let seq_after = current_seq();
    // 自写豁免：MDI 自己写入产生的新 seq 记入跳过集合，
    // 监听线程命中即跳过 → 不产生新历史 / 不触发岛提示 / 不重排列表
    if seq_after != seq_before {
        mark_skip_seq(seq_after);
    }
    Ok(())
}

fn write_to_clipboard(app: &AppHandle, item: &ClipItem) -> Result<(), String> {
    unsafe {
        let mut opened = false;
        for _ in 0..3 {
            if windows_sys::Win32::System::DataExchange::OpenClipboard(std::ptr::null_mut()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if !opened {
            return Err("剪贴板被其他程序占用，请稍后再试".into());
        }
        let result = match item.kind.as_str() {
            "text" => write_text_locked(item.text.as_deref().unwrap_or("")),
            "image" => write_image_locked(app, item),
            _ => Err("未知条目类型".into()),
        };
        windows_sys::Win32::System::DataExchange::CloseClipboard();
        result
    }
}

/// 分配 GMEM_MOVEABLE 写入字节并 SetClipboardData；
/// 成功后系统接管内存**不得释放**，失败时所有权未移交，手动 GlobalFree 回收
unsafe fn set_clipboard_bytes(fmt: u32, bytes: &[u8]) -> bool {
    // GlobalFree 不在 windows-sys 0.59 的 Memory 模块（rustc E0432 实证），
    // 按项目规范剪贴板 Global* 一律走 winapi 0.3
    use winapi::um::winbase::{GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock};
    use windows_sys::Win32::System::DataExchange::SetClipboardData;

    let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
    if h.is_null() {
        return false;
    }
    let dst = GlobalLock(h);
    if dst.is_null() {
        GlobalFree(h);
        return false;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst as *mut u8, bytes.len());
    GlobalUnlock(h);
    if SetClipboardData(fmt, h).is_null() {
        GlobalFree(h);
        false
    } else {
        true
    }
}

/// 文本写回：同时写 CF_UNICODETEXT + CF_TEXT（ANSI，兼容老程序）
fn write_text_locked(text: &str) -> Result<(), String> {
    use windows_sys::Win32::Globalization::WideCharToMultiByte;

    unsafe {
        windows_sys::Win32::System::DataExchange::EmptyClipboard();
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);
        // SAFETY: reinterpret &[u16] as flat bytes for the GlobalAlloc copy
        let wide_bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
        if !set_clipboard_bytes(CF_UNICODETEXT, wide_bytes) {
            return Err("写入剪贴板失败".into());
        }
        // CF_TEXT：按系统 ANSI 代码页转换（转换失败不影响主格式）
        let ansi_len = WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        );
        if ansi_len > 0 {
            let mut ansi = vec![0u8; ansi_len as usize];
            WideCharToMultiByte(
                CP_ACP,
                0,
                wide.as_ptr(),
                wide.len() as i32,
                ansi.as_mut_ptr(),
                ansi_len,
                std::ptr::null(),
                std::ptr::null_mut(),
            );
            set_clipboard_bytes(CF_TEXT, &ansi);
        }
        Ok(())
    }
}

/// 图片写回：注册格式 "PNG" + 原样字节流（零重编码，粘回即原图）
fn write_image_locked(app: &AppHandle, item: &ClipItem) -> Result<(), String> {
    let dir = imgs_dir(app)?;
    let bytes = std::fs::read(dir.join(format!("{}.png", item.id)))
        .map_err(|e| format!("读取原图失败: {}", e))?;
    unsafe {
        windows_sys::Win32::System::DataExchange::EmptyClipboard();
        let png_fmt = register_png_format();
        if png_fmt == 0 {
            return Err("注册 PNG 剪贴板格式失败".into());
        }
        if set_clipboard_bytes(png_fmt, &bytes) {
            Ok(())
        } else {
            Err("写入剪贴板失败".into())
        }
    }
}

/// 置顶 / 取消置顶：仅改 pinned 与 pin_ts_ms，不动列表顺序（下次进入页面才重排）
#[tauri::command]
pub fn clipboard_toggle_pin(app: AppHandle, id: String) -> Result<(), String> {
    let mut list = load_history(&app);
    let item = list.iter_mut().find(|i| i.id == id).ok_or("条目不存在")?;
    item.pinned = !item.pinned;
    item.pin_ts_ms = if item.pinned { now_ms() } else { 0 };
    persist_history(&app, &list)
}

#[tauri::command]
pub fn clipboard_delete_item(app: AppHandle, id: String) -> Result<(), String> {
    let mut list = load_history(&app);
    if let Some(pos) = list.iter().position(|i| i.id == id) {
        let item = list.remove(pos);
        delete_item_files(&app, &item);
        persist_history(&app, &list)?;
    }
    Ok(())
}

#[tauri::command]
pub fn clipboard_clear(app: AppHandle) -> Result<(), String> {
    let list = load_history(&app);
    for item in &list {
        delete_item_files(&app, item);
    }
    persist_history(&app, &Vec::new())
}
