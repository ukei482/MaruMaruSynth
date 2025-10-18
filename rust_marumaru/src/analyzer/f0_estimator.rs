// src/analyzer/f0_estimator.rs

use rustfft::{FftPlanner, num_complex::Complex};
use pitch_detection::detector::{yin::YINDetector, PitchDetector};
use splines::{Spline, Key, Interpolation};


// --- 解析パラメータ ---
const FRAME_SIZE   : usize = 2048;
const HOP_SIZE     : usize = 512;
const PADDING_SIZE : usize = FRAME_SIZE / 2; // YIN用

fn cepstrum_estimator(frame: &[f32], sample_rate: u32, planner: &mut FftPlanner<f32>) -> (f32, f32) {
    let frame_size = frame.len();
    let window: Vec<f32> = (0..frame_size)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (frame_size - 1) as f32).cos()))
        .collect();
    let mut buffer: Vec<Complex<f32>> = frame.iter()
        .zip(window.iter())
        .map(|(&sample, &win)| Complex::new(sample * win, 0.0))
        .collect();
    let fft_forward = planner.plan_fft_forward(frame_size);
    fft_forward.process(&mut buffer);
    
    // ★★★ 修正点: FFT後の結果をFFT_SIZEで割って正規化する ★★★
    let normalized_buffer: Vec<Complex<f32>> = buffer.iter()
        .map(|c| *c / frame_size as f32)
        .collect();

    let log_power_spectrum: Vec<Complex<f32>> = normalized_buffer.iter()
        .map(|c| Complex::new((c.norm_sqr() + 1e-12).log10(), 0.0))
        .collect();
        
    let mut cepstrum_buffer = log_power_spectrum;
    let fft_inverse = planner.plan_fft_inverse(frame_size);
    fft_inverse.process(&mut cepstrum_buffer);
    
    // NOTE: iFFT後の結果もFFT_SIZEで割って正規化する必要がありますが、
    // ここでは比率しか見ていないため、省略します。
    // 信頼度を計算する際には最終的に正規化に近くなります。
    
    let min_period = (sample_rate as f32 / 1000.0).floor() as usize;
    let max_period = (sample_rate as f32 / 80.0).ceil() as usize;
    let search_range_start = min_period.max(1);
    let search_range_end = max_period.min(cepstrum_buffer.len() / 2);
    let peak = cepstrum_buffer[search_range_start..search_range_end]
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.re.partial_cmp(&b.1.re).unwrap());
    if let Some((index, &value)) = peak {
        let period = index + search_range_start;
        let f0 = sample_rate as f32 / period as f32;
        // FFT後の正規化で値が小さくなったため、ここで scaling factor を調整
        let confidence = (value.re / frame_size as f32).max(0.0).min(1.0); 
        (f0, confidence)
    } else {
        (0.0, 0.0)
    }
}


/// F0カーブの欠損（ゼロ）をスプライン補間する関数
fn post_process_f0_curve(f0_curve: &mut [f32]) {
    let mut zero_start_index = None;

    for i in 0..f0_curve.len() {
        if f0_curve[i] == 0.0 && zero_start_index.is_none() {
            zero_start_index = Some(i);
        } else if f0_curve[i] != 0.0 && zero_start_index.is_some() {
            let start = zero_start_index.unwrap();
            let end = i;
            
            if start > 0 {
                let p1_idx = start - 1;
                let p0_idx = if start > 1 { start - 2 } else { p1_idx };
                let p2_idx = end;
                let p3_idx = if end < f0_curve.len() - 1 { end + 1 } else { end };

                if p0_idx < p1_idx && p1_idx < p2_idx && p2_idx < p3_idx {
                    let p0 = f0_curve.get(p0_idx).cloned().unwrap_or(f0_curve[p1_idx]);
                    let p1 = f0_curve[p1_idx];
                    let p2 = f0_curve[p2_idx];
                    let p3 = f0_curve.get(p3_idx).cloned().unwrap_or(f0_curve[p2_idx]);
                    
                    let points = vec![
                        Key::new(p1_idx as f32, p0, Interpolation::CatmullRom),
                        Key::new(p1_idx as f32, p1, Interpolation::CatmullRom),
                        Key::new(p2_idx as f32, p2, Interpolation::CatmullRom),
                        Key::new(p2_idx as f32, p3, Interpolation::CatmullRom),
                    ];
                    let spline = Spline::from_vec(points);

                    for j in start..end {
                        if let Some(val) = spline.sample(j as f32) {
                            f0_curve[j] = val;
                        }
                    }
                } else if p1_idx < p2_idx {
                    let p1 = f0_curve[p1_idx];
                    let p2 = f0_curve[p2_idx];
                    for j in start..end {
                        let t = (j - p1_idx) as f32 / (p2_idx - p1_idx) as f32;
                        f0_curve[j] = p1 * (1.0 - t) + p2 * t;
                    }
                }
            }
            zero_start_index = None;
        }
    }
}

/// F0の時系列データ（カーブ）を推定する
pub fn estimate_f0_curve(
    audio: &[f32],
    sample_rate: u32
) -> Result<(Vec<f32>, Vec<f32>), String> {
    println!("[INFO] F0 estimation started with robust fusion logic.");
    if audio.len() < FRAME_SIZE {
        return Err("Audio data is too short for F0 estimation.".to_string());
    }
    let mut yin_detector = YINDetector::<f32>::new(FRAME_SIZE, PADDING_SIZE);
    let mut fft_planner: FftPlanner<f32> = FftPlanner::new();
    
    let mut f0_curve = Vec::new();
    let mut confidence_curve = Vec::new();
    let frames = audio.windows(FRAME_SIZE).step_by(HOP_SIZE);
    
    for frame in frames {
        let yin_pitch = yin_detector.get_pitch(frame, sample_rate as usize, 0.1, 0.0);
        
        let (final_f0, final_confidence);

        if let Some(p) = yin_pitch {
            if p.clarity > 0.7 {
                final_f0 = p.frequency;
                final_confidence = p.clarity;
            } else {
                let (f0_cep, c_cep) = cepstrum_estimator(frame, sample_rate, &mut fft_planner);
                if c_cep > p.clarity {
                    final_f0 = f0_cep;
                    final_confidence = c_cep;
                } else {
                    final_f0 = p.frequency;
                    final_confidence = p.clarity;
                }
            }
        } else {
            let (f0_cep, c_cep) = cepstrum_estimator(frame, sample_rate, &mut fft_planner);
            final_f0 = f0_cep;
            final_confidence = c_cep;
        }

        f0_curve.push(final_f0);
        confidence_curve.push(final_confidence);
    }

    println!("[INFO] Post-processing F0 curve with spline interpolation...");
    post_process_f0_curve(&mut f0_curve);
    
    println!("[INFO] F0 estimation finished. Generated {} frames.", f0_curve.len());
    Ok((f0_curve, confidence_curve))
}