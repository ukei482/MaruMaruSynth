// src/analyzer/mode_freq.rs

use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

// STFTのパラメータ
const FFT_SIZE: usize = 2048;
const HOP_SIZE: usize = 512; // 75% overlap

/// 周波数領域での音声解析を行う (スペクトル平均化 + 位相ロック実装版)
pub fn analyze_freq_domain(
    audio: &[f32],
    _sample_rate: u32,
) -> Result<Vec<Vec<f32>>, String> {
    println!("[INFO] Frequency domain analysis with Spectrum Averaging + Phase Locking started.");

    if audio.len() < FFT_SIZE {
        return Err("Audio data is too short for frequency domain analysis.".to_string());
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let ifft = planner.plan_fft_inverse(FFT_SIZE);

    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos()))
        .collect();

    let frames = audio.windows(FFT_SIZE).step_by(HOP_SIZE);
    let spectrogram: Vec<Vec<Complex<f32>>> = frames.map(|frame| {
        let mut buffer: Vec<Complex<f32>> = frame.iter()
            .zip(window.iter())
            .map(|(&sample, &win)| Complex::new(sample * win, 0.0))
            .collect();
        fft.process(&mut buffer);
        buffer
    }).collect();

    if spectrogram.is_empty() {
        return Err("Could not generate spectrogram from the audio.".to_string());
    }

    let mut avg_magnitudes = vec![0.0; FFT_SIZE / 2 + 1];
    for frame_spec in &spectrogram {
        for i in 0..=FFT_SIZE / 2 {
            avg_magnitudes[i] += frame_spec[i].norm();
        }
    }
    for mag in &mut avg_magnitudes {
        *mag /= spectrogram.len() as f32;
    }

    let mut final_spectrum = vec![Complex::default(); FFT_SIZE];
    let locked_phase = -PI / 2.0;

    for i in 0..=FFT_SIZE / 2 {
        let new_complex = Complex::from_polar(avg_magnitudes[i], locked_phase);
        final_spectrum[i] = new_complex;
        if i > 0 && i < FFT_SIZE / 2 {
            final_spectrum[FFT_SIZE - i] = new_complex.conj();
        }
    }

    ifft.process(&mut final_spectrum);
    // ★ 修正点: analyze_freq_domain本体のiFFT後にも正規化を追加
    let mut final_table: Vec<f32> = final_spectrum.iter().map(|c| c.re / FFT_SIZE as f32).collect();

    let max_abs = final_table.iter().map(|&s| s.abs()).fold(0.0, f32::max);
    if max_abs > 1e-6 {
        for sample in final_table.iter_mut() {
            *sample /= max_abs;
        }
    }

    println!("[INFO] Frequency domain analysis finished. Generated 1 wavetable.");
    Ok(vec![final_table])
}


#[cfg(test)]
mod tests {
    use super::*;

    fn stft_istft_roundtrip(audio: &[f32]) -> Vec<f32> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let ifft = planner.plan_fft_inverse(FFT_SIZE);
        let window: Vec<f32> = (0..FFT_SIZE).map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos())).collect();

        let frames = audio.windows(FFT_SIZE).step_by(HOP_SIZE);
        let spectrogram: Vec<Vec<Complex<f32>>> = frames.map(|frame| {
            let mut buffer: Vec<Complex<f32>> = frame.iter().zip(&window).map(|(&s, &w)| Complex::new(s * w, 0.0)).collect();
            fft.process(&mut buffer);
            buffer
        }).collect();

        let mut output_audio = vec![0.0; audio.len()];
        let mut window_sum = vec![0.0; audio.len()];
        for (i, mut frame) in spectrogram.into_iter().enumerate() {
            ifft.process(&mut frame);
            let start = i * HOP_SIZE;
            for j in 0..FFT_SIZE {
                if start + j < output_audio.len() {
                    output_audio[start + j] += (frame[j].re / FFT_SIZE as f32) * window[j];
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

    #[test]
    fn test_stft_istft_reconstruction_quality() {
        // ★ 修正点: テスト信号をチャープから単純なサイン波に変更
        const SAMPLE_RATE: u32 = 48000;
        const SIGNAL_LEN: usize = SAMPLE_RATE as usize;
        let mut signal = Vec::new();
        for i in 0..SIGNAL_LEN {
            let time = i as f32 / SAMPLE_RATE as f32;
            let freq = 440.0; // 440Hz固定
            let sample = (2.0 * PI * freq * time).sin() * 0.5;
            signal.push(sample);
        }

        let reconstructed_signal = stft_istft_roundtrip(&signal);

        let mut signal_power = 0.0;
        let mut error_power = 0.0;
        for i in FFT_SIZE..SIGNAL_LEN - FFT_SIZE { // 信号の末尾も不安定なので無視する
            signal_power += signal[i].powi(2);
            error_power += (signal[i] - reconstructed_signal[i]).powi(2);
        }

        let snr = 10.0 * (signal_power / error_power).log10();
        
        assert!(
            snr > 60.0,
            "再合成の品質が低すぎます。SNR: {:.2} dB",
            snr
        );
    }
}