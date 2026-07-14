use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// 存储 5 个频段的全局数组，默认高度 0.35 (对应前端的 scaleY(0.35))
static SPECTRUM: Mutex<[f32; 5]> = Mutex::new([0.35; 5]);

// FFT 计划缓存，避免每次回调都重建 FftPlanner
static FFT_CACHE: Mutex<Option<(usize, std::sync::Arc<dyn Fft<f32>>)>> = Mutex::new(None);

// 频谱处理开关：仅在前端需要频谱数据时才执行 FFT 运算，空闲时零分配零 CPU
static SPECTRUM_ACTIVE: AtomicBool = AtomicBool::new(false);

// B8: 全局 AppHandle，用于向 Tauri 事件系统推送频谱数据（前端从 invoke 轮询改为 listen 被动接收）
static APP_HANDLE: Mutex<Option<Arc<tauri::AppHandle>>> = Mutex::new(None);

/// 注册 AppHandle（在 Tauri setup 阶段调用），用于 emit 事件到前端
pub fn set_app_handle(handle: Arc<tauri::AppHandle>) {
    *APP_HANDLE.lock().unwrap() = Some(handle);
}

/// 向 WebSocket 推送最新频谱数据（5 个频段，范围 0.35~0.95）
fn emit_spectrum(data: &[f32; 5]) {
    let handle = {
        let guard = APP_HANDLE.lock().unwrap();
        guard.clone()
    };
    if let Some(handle) = handle {
        let _ = handle.emit("spectrum-data", &data.to_vec());
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

#[tauri::command]
pub fn get_audio_spectrum() -> [f32; 5] {
    *SPECTRUM.lock().unwrap()
}

#[tauri::command]
pub fn set_spectrum_active(active: bool) {
    SPECTRUM_ACTIVE.store(active, Ordering::Relaxed);
}

pub fn start_monitor() {
    thread::spawn(|| {
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
                move |data: &[f32], _: &_| process_data(data, channels),
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _: &_| {
                    // 频谱未激活时直接早退，避免无谓的 Vec 分配和类型转换
                    if !SPECTRUM_ACTIVE.load(Ordering::Relaxed) { return; }
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
            let _ = stream.play();
            // 保持子线程存活
            loop {
                thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
    });
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
            let mut cache = FFT_CACHE.lock().unwrap();
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

        // 6. 映射高度并平滑过渡
        let mut final_spectrum = [0.35_f32; 5];

        // 频段能量补偿权重（大幅压平）：高频保留最基础的补偿，不再强行放大
        let eq_weights = [1.2, 1.1, 1.5, 3.0, 5.0];
        // 整体小音量放大系数（再次削减）：直接降到个位数，大幅抑制微小底噪的跳动
        let base_gain = 5.0;

        for i in 0..5 {
            let energy = bins[i] * eq_weights[i] * base_gain;

            let scaled = ((energy + 1.0).log10() * 0.20) + 0.35;

            final_spectrum[i] = scaled.clamp(0.35, 0.95);
        }

        // 7. 更新到全局并应用平滑插值 (Lerp)，防止画面闪烁跳动过于剧烈
        if let Ok(mut spec) = SPECTRUM.lock() {
            for i in 0..5 {
                spec[i] = spec[i] * 0.6 + final_spectrum[i] * 0.4;
            }
            // B8: FFT 处理后通过 Tauri emit 推送，替代前端 setInterval 轮询
            emit_spectrum(&spec);
        }
    })();

    // 放回 buffer 供下次复用（容量保留，只增不减）
    MONO_BUF.with(|b| *b.borrow_mut() = mono);
    COMPLEX_BUF.with(|b| *b.borrow_mut() = buffer);
}