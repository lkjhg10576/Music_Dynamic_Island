use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Mutex;
use std::thread;

// 存储 5 个频段的全局数组，默认高度 0.35 (对应前端的 scaleY(0.35))
static SPECTRUM: Mutex<[f32; 5]> = Mutex::new([0.35; 5]);

// FFT 计划缓存，避免每次回调都重建 FftPlanner
static FFT_CACHE: Mutex<Option<(usize, std::sync::Arc<dyn Fft<f32>>)>> = Mutex::new(None);

#[tauri::command]
pub fn get_audio_spectrum() -> [f32; 5] {
    *SPECTRUM.lock().unwrap()
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
                    let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    process_data(&f32_data, channels);
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
    if data.is_empty() { return; }

    // 1. 将双声道/多声道合并为单声道
    let mut mono = Vec::with_capacity(data.len() / channels as usize);
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
    let mut buffer: Vec<Complex<f32>> = mono.iter().enumerate().map(|(i, &val)| {
        let multiplier = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos());
        Complex { re: val * multiplier, im: 0.0 }
    }).collect();

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
    }
}