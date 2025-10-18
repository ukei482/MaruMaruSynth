// src/analyzer/preprocess.rs
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

// --- STFTパラメータ ---
const FFT_SIZE: usize = 1024;
const HOP_SIZE: usize = 256;

/// RMS (二乗平均平方根) を基準に音量を正規化する
fn normalize(audio: &[f32], target_dbfs: f32) -> Vec<f32> {
    let target_rms = 10.0f32.powf(target_dbfs / 20.0);
    let mut sum_sq = 0.0;
    for &sample in audio {
        sum_sq += sample * sample;
    }
    let measured_rms = (sum_sq / audio.len() as f32).sqrt();

    if measured_rms < 1e-6 {
        return audio.to_vec();
    }

    let gain = target_rms / measured_rms;
    audio.iter().map(|&sample| sample * gain).collect()
}

/// DCオフセットを除去する
fn dc_remove(audio: &[f32], alpha: f32) -> Vec<f32> {
    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    audio.iter().map(|&x| {
        let y = x - prev_x + alpha * prev_y;
        prev_x = x;
        prev_y = y;
        y
    }).collect()
}

/// スペクトルゲートによるノイズ除去
fn spectral_gate(audio: &[f32]) -> Vec<f32> {
    println!("[INFO] Applying spectral gate for noise reduction.");
    if audio.len() < FFT_SIZE {
        return audio.to_vec();
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let ifft = planner.plan_fft_inverse(FFT_SIZE);
    let window: Vec<f32> = (0..FFT_SIZE).map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos())).collect();

    let frames = audio.windows(FFT_SIZE).step_by(HOP_SIZE);
    let mut spectrogram: Vec<Vec<Complex<f32>>> = frames.map(|frame| {
        let mut buffer: Vec<Complex<f32>> = frame.iter().zip(&window).map(|(&s, &w)| Complex::new(s * w, 0.0)).collect();
        fft.process(&mut buffer);
        buffer
    }).collect();

    let mut noise_floor = vec![0.0; FFT_SIZE / 2 + 1];
    if let Some(quietest_frame) = spectrogram.iter().min_by(|a, b| {
        let power_a: f32 = a.iter().map(|c| c.norm_sqr()).sum();
        let power_b: f32 = b.iter().map(|c| c.norm_sqr()).sum();
        power_a.partial_cmp(&power_b).unwrap()
    }) {
        for i in 0..=FFT_SIZE / 2 {
            noise_floor[i] = quietest_frame[i].norm();
        }
    }

    let noise_threshold = 1.5;
    for frame in &mut spectrogram {
        for i in 0..=FFT_SIZE / 2 {
            if frame[i].norm() < noise_floor[i] * noise_threshold {
                frame[i] = Complex::new(0.0, 0.0);
            }
        }
    }

    let mut output_audio = vec![0.0; audio.len()];
    let mut window_sum = vec![0.0; audio.len()];
    for (i, mut frame) in spectrogram.into_iter().enumerate() {
        ifft.process(&mut frame);
        let start = i * HOP_SIZE;
        for j in 0..FFT_SIZE {
            if start + j < output_audio.len() {
                output_audio[start + j] += frame[j].re * window[j];
                window_sum[start + j] += window[j].powi(2);
            }
        }
    }

    for i in 0..output_audio.len() {
        if window_sum[i] > 1e-6 {
            output_audio[i] /= window_sum[i];
        }
    }
    output_audio
}


/// 全ての前処理を順番に適用する
pub fn apply_all_preprocessing(audio: &[f32]) -> Result<Vec<f32>, String> {
    if audio.is_empty() {
        return Err("Input audio is empty.".to_string());
    }
    
    let normalized = normalize(audio, -10.0);
    let dc_removed = dc_remove(&normalized, 0.995);
    let noise_reduced = spectral_gate(&dc_removed);
    
    Ok(noise_reduced)
}

// --- テストモジュール ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_remove_positive_offset() {
        // 1. Arrange: 信号長を長くしてフィルターのウォームアップ時間を確保
        let signal_length = 2000;
        let mut input_signal = Vec::new();
        for i in 0..signal_length {
            let sine_val = (i as f32 * 0.1).sin();
            input_signal.push(sine_val + 2.0); // DCオフセット = 2.0
        }

        // 2. Act
        let output_signal = dc_remove(&input_signal, 0.995);

        // 3. Assert: 信号の「最後の」部分で検証する
        let later_samples = &output_signal[signal_length - 500..];
        let average: f32 = later_samples.iter().sum::<f32>() / later_samples.len() as f32;

        assert!(
            average.abs() < 1e-2,
            "DCオフセットが除去されませんでした。平均値: {}",
            average
        );
    }
    
    #[test]
    fn test_normalize_to_target_dbfs() {
        let rms = 10.0f32.powf(-20.0 / 20.0);
        let input: Vec<f32> = (0..100).map(|i| (i as f32).sin() * rms * 2.0f32.sqrt()).collect();
        let output = normalize(&input, -10.0);
        let output_rms = (output.iter().map(|&s| s*s).sum::<f32>() / output.len() as f32).sqrt();
        let target_rms = 10.0f32.powf(-10.0 / 20.0);
        assert!((output_rms - target_rms).abs() < 1e-6, "正規化後のRMSがターゲットと異なります。");
    }
}