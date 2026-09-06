use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

use crate::thread_mgr;
use crate::win32_utils::log_err;

// 存储 5 个频段的全局数组，默认高度 0.35 (对应前端的 scaleY(0.35))
static SPECTRUM: Mutex<[f32; 5]> = Mutex::new([0.35; 5]);

// FFT 计划缓存，避免每次回调都重建 FftPlanner
static FFT_CACHE: Mutex<Option<(usize, std::sync::Arc<dyn Fft<f32>>)>> = Mutex::new(None);

// 频谱处理开关：仅在前端需要频谱数据时才执行 FFT 运算，空闲时零分配零 CPU
static SPECTRUM_ACTIVE: AtomicBool = AtomicBool::new(false);

// 采集线程是否在运行（§4.2 按需启停的幂等守卫）
static MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);

// AGC：每频段滑动峰值包络，使任意音量下视觉幅度一致
static PEAK_ENV: Mutex<[f32; 5]> = Mutex::new([0.0; 5]);

// B8: 全局 AppHandle，用于向 Tauri 事件系统推送频谱数据（前端从 invoke 轮询改为 listen 被动接收）
static APP_HANDLE: Mutex<Option<Arc<tauri::AppHandle>>> = Mutex::new(None);

// B9: 频谱 emit 节流锁 — 限制每 50ms 最多推送一次（从 ~86Hz 降至 ~20Hz），大幅减少 WebSocket 消息积压
static LAST_EMIT_MS: AtomicU64 = AtomicU64::new(0);

// 流失效看门狗：最近一次数据回调到达的毫秒时间戳（0 表示尚无数据）。
// 正常暂停时采集流仍在运行（回调持续送达，频谱自然回落基准线），
// 但播放器瞬间退出/系统音频流被强行终止时回调会直接停摆，
// 看门狗据此把频谱条平滑衰减回基准线并推送，避免前端卡死在最后一帧。
static LAST_DATA_AT_MS: AtomicU64 = AtomicU64::new(0);

/// 注册 AppHandle（在 Tauri setup 阶段调用），用于 emit 事件到前端
pub fn set_app_handle(handle: Arc<tauri::AppHandle>) {
    *APP_HANDLE.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
}

/// 向 WebSocket 推送最新频谱数据（5 个频段，范围 0.35~0.95）
fn emit_spectrum(data: &[f32; 5]) {
    let handle = {
        let guard = APP_HANDLE.lock().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    };
    if let Some(handle) = handle {
        log_err(handle.emit("spectrum-data", data), "emit spectrum-data");
    }
}

/// 刷新「最近一次数据回调到达」时间戳（数据回调在频谱激活时调用）
fn touch_data_timestamp() {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    LAST_DATA_AT_MS.store(now_ms, Ordering::Relaxed);
}

/// 流失效看门狗（采集线程每 50ms 醒一次时调用）：
/// 正常暂停时数据回调仍在送达，无需干预；
/// 但采集流停摆超过 150ms（播放器瞬间退出等异常打断）时，
/// 把频谱逐帧衰减回静默基准线 0.35 并推送给前端，直到各条归位——
/// 保证前端不会停留在音频中断前最后一刻的波形上。
fn watchdog_tick() {
    if !SPECTRUM_ACTIVE.load(Ordering::Relaxed) { return; }
    let last = LAST_DATA_AT_MS.load(Ordering::Relaxed);
    if last == 0 { return; } // 尚无数据回调，初始态即基准线，无需衰减
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    if now_ms.wrapping_sub(last) < 150 { return; } // 数据回调仍在活跃，流正常

    // 流已静默：每帧向基准线收缩 15%（约 1.5s 内归位），归位后停止推送
    let mut spec = SPECTRUM.lock().unwrap_or_else(|e| e.into_inner());
    let mut any = false;
    for i in 0..5 {
        if (spec[i] - 0.35).abs() > 0.002 {
            spec[i] = 0.35 + (spec[i] - 0.35) * 0.85;
            any = true;
        }
    }
    if any {
        emit_spectrum(&spec);
    }
}

// 线程本地复用 buffer，避免每次回调都分配/释放 Vec（容量只增不减）
// MONO_BUF: 多声道合并后的单声道数据
// COMPLEX_BUF: FFT 输入的复数缓冲
// I16_F32_BUF: I16 格式设备的 f32 转换缓冲（独立于 MONO_BUF 避免借用冲突）
thread_local! {
    static MONO_BUF: std::cell::RefCell<Vec<f32>> = std::cell::RefCell::new(Vec::new());
    static COMPLEX_BUF: std::cell::RefCell<Vec<Complex<f32>>> = std::cell::RefCell::new(Vec::new());
    static I16_F32_BUF: std::cell::RefCell<Vec<f32>> = std::cell::RefCell::new(Vec::new());
}

// ===== 按需启停（§4.2）=====

/// 启动音频频谱采集线程（幂等：重复调用不会生成多个线程）。
/// 线程持有 cpal loopback stream；退出标志置位后 stream 随线程结束一并 Drop，
/// 释放音频采集资源（旧实现的 `loop { sleep(3600s) }` 空转线程已废弃）。
pub fn start_monitor() {
    if MONITOR_RUNNING.swap(true, Ordering::SeqCst) {
        return; // 已在运行
    }
    thread_mgr::spawn_managed("audio_spectrum", |exit| {
        monitor_thread(exit);
        MONITOR_RUNNING.store(false, Ordering::SeqCst);
    });
}

/// 请求停止采集线程并等待其退出（stream 随之 Drop）
pub fn stop_monitor() {
    thread_mgr::stop_thread("audio_spectrum");
}

/// 启动采集 + 激活 FFT（前端命令）
#[tauri::command]
pub fn start_audio_spectrum() {
    SPECTRUM_ACTIVE.store(true, Ordering::Relaxed);
    start_monitor();
}

/// 停止 FFT + 关闭采集线程（前端命令）
#[tauri::command]
pub fn stop_audio_spectrum() {
    SPECTRUM_ACTIVE.store(false, Ordering::Relaxed);
    stop_monitor();
}

/// 兼容入口：原统一开关（等价于 start/stop_audio_spectrum）
#[tauri::command]
pub fn set_spectrum_active(active: bool) {
    if active {
        start_audio_spectrum();
    } else {
        stop_audio_spectrum();
    }
}

// ===== 采集线程 =====

fn monitor_thread(exit: thread_mgr::ExitFlag) {
    let host = cpal::default_host();

    // 获取系统默认播放设备
    let device = match host.default_output_device() {
        Some(d) => d,
        None => return,
    };

    let config = match device.default_output_config() {
        Ok(c) => c,
        Err(_) => return,
    };

    let err_fn = |err| eprintln!("Audio capture error: {}", err);
    let sample_format = config.sample_format();
    let config: cpal::StreamConfig = config.into();
    let channels = config.channels;

    // 在 Windows 上，对 output_device 调用 build_input_stream 会自动开启 Loopback(内录) 模式
    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _: &_| {
                // 频谱未激活时直接早退，避免无谓的 process_data 函数调用和 thread_local buffer 操作
                if !SPECTRUM_ACTIVE.load(Ordering::Relaxed) { return; }
                touch_data_timestamp();
                process_data(data, channels)
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _: &_| {
                // 频谱未激活时直接早退，避免无谓的 Vec 分配和类型转换
                if !SPECTRUM_ACTIVE.load(Ordering::Relaxed) { return; }
                touch_data_timestamp();
                // 复用 thread_local buffer，避免每次回调都分配
                I16_F32_BUF.with(|buf| {
                    let mut f32_data = buf.borrow_mut();
                    f32_data.clear();
                    f32_data.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
                    process_data(&f32_data, channels);
                });
            },
            err_fn,
            None,
        ),
        _ => return,
    };

    if let Ok(stream) = stream {
        if let Err(e) = stream.play() {
            eprintln!("[NSD][warn] 音频频谱采集启动失败: {}", e);
            return;
        }
        // 挂起等待退出信号；正常情况每 1h 醒来复查一次防错过，
        // stop_monitor 触发 signal 后立即醒来，stream 随作用域结束 Drop。
        // 流失效看门狗（每 50ms 醒一次）：正常暂停时数据回调仍在送达（频谱自然回落基准线），
        // 但播放器瞬间退出 / 系统音频流被强行终止时回调会直接停摆——
        // 超过阈值未收到新数据即判定流已静默，把频谱条按帧衰减回基准线并推送，
        // 修复「音频突然被打断时频谱卡在最后一刻状态」的问题。
        loop {
            if exit.sleep_interruptible(std::time::Duration::from_millis(50)) {
                break;
            }
            watchdog_tick();
        }
    }
}

// FFT 核心处理逻辑
fn process_data(data: &[f32], channels: u16) {
    // 频谱未激活时直接早退，跳过所有 FFT 计算（F32 分支的二次保险）
    if !SPECTRUM_ACTIVE.load(Ordering::Relaxed) { return; }
    if data.is_empty() { return; }

    // 从 thread_local 取出可复用 buffer（std::mem::take 零成本移动取出，原位置留空 Vec）
    let mut mono = MONO_BUF.with(|b| std::mem::take(&mut *b.borrow_mut()));
    let mut buffer = COMPLEX_BUF.with(|b| std::mem::take(&mut *b.borrow_mut()));

    // 核心处理逻辑放在闭包里，确保所有 early return 后 buffer 都能被放回 thread_local
    (|| {
        // 1. 将双声道/多声道合并为单声道
        mono.clear();
        for chunk in data.chunks(channels as usize) {
            let sum: f32 = chunk.iter().sum();
            mono.push(sum / channels as f32);
        }

        let n = mono.len();
        if n < 128 { return; } // 样本太少不做分析

        // 2. FFT 准备 - 使用缓存避免每次重建
        let fft = {
            let mut cache = FFT_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if cache.as_ref().map_or(true, |(len, _)| *len != n) {
                let mut planner = FftPlanner::new();
                let plan = planner.plan_fft_forward(n);
                *cache = Some((n, plan));
            }
            cache.as_ref().unwrap().1.clone()
        };

        // 3. 加汉宁窗 (Hanning Window) 平滑边缘，减少频谱泄漏
        buffer.clear();
        buffer.extend(mono.iter().enumerate().map(|(i, &val)| {
            let multiplier = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos());
            Complex { re: val * multiplier, im: 0.0 }
        }));

        // 4. 执行 FFT 运算
        fft.process(&mut buffer);

        // 5. 分成 5 个对数频段：低音(Bass) -> 高音(Treble)
        let mut bins = [0.0_f32; 5];
        let half_n = n / 2;

        // 忽略直流分量(0)
        for i in 1..half_n {
            let mag = (buffer[i].re.powi(2) + buffer[i].im.powi(2)).sqrt();

            let bin_idx = if i < half_n / 16 { 0 }       // 低频
            else if i < half_n / 8 { 1 }                 // 中低频
            else if i < half_n / 4 { 2 }                 // 中频
            else if i < half_n / 2 { 3 }                 // 中高频
            else { 4 };                                  // 高频

            // 取该频段的最大振幅
            if mag > bins[bin_idx] {
                bins[bin_idx] = mag;
            }
        }

        // 6. AGC 峰值归一化 → 映射到 0.35~0.95，保证任意音量下视觉幅度一致
        let mut final_spectrum = [0.35_f32; 5];

        // 频段能量补偿权重：高频保留基础补偿
        let eq_weights = [1.2, 1.1, 1.5, 3.0, 5.0];
        // 静音/底噪门限：低于此能量直接归零，避免 AGC 放大底噪
        const NOISE_FLOOR: f32 = 1e-4;
        const PEAK_EPS: f32 = 1e-6;
        const PEAK_DECAY: f32 = 0.98;

        if let Ok(mut peaks) = PEAK_ENV.lock() {
            for i in 0..5 {
                let energy = bins[i] * eq_weights[i];

                // 快上升慢衰减的峰值包络
                if energy > peaks[i] {
                    peaks[i] = energy;
                } else {
                    peaks[i] *= PEAK_DECAY;
                }

                let normalized = if energy < NOISE_FLOOR {
                    0.0
                } else {
                    (energy / (peaks[i] + PEAK_EPS)).clamp(0.0, 1.0)
                };

                // 映射到前端基线 0.35 ~ 峰值 0.95
                final_spectrum[i] = (0.35 + normalized * 0.60).clamp(0.35, 0.95);
            }
        }

        // 7. 更新到全局并应用平滑插值 (Lerp)，防止画面闪烁跳动过于剧烈
        if let Ok(mut spec) = SPECTRUM.lock() {
            for i in 0..5 {
                spec[i] = spec[i] * 0.6 + final_spectrum[i] * 0.4;
            }
            // B8: FFT 处理后通过 Tauri emit 推送，替代前端 setInterval 轮询
            // B9: 节流 — 每 66ms 最多 emit 一次（~15fps 足够频谱动画），减少 IPC 次数与堆分配
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let prev_ms = LAST_EMIT_MS.load(Ordering::Relaxed);
            if now_ms.wrapping_sub(prev_ms) >= 66 {
                LAST_EMIT_MS.store(now_ms, Ordering::Relaxed);
                emit_spectrum(&spec);
            }
        }
    })();

    // 放回 buffer 供下次复用（容量保留，只增不减）
    MONO_BUF.with(|b| *b.borrow_mut() = mono);
    COMPLEX_BUF.with(|b| *b.borrow_mut() = buffer);
}
