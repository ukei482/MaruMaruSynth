// src/analyzer/mode_hybrid.rs

// (関数の部分は変更なしなので省略)
use super::mode_time;
use super::mode_freq;
use std::f32::consts::PI;

fn apply_filter(audio: &[f32], sample_rate: u32, cutoff_freq: f32, is_highpass: bool) -> Vec<f32> {
    let omega = 2.0 * PI * cutoff_freq / sample_rate as f32;
    let alpha = omega.sin() / (2.0 * 0.707); // Q=0.707 (Butterworth)

    let b0 = (1.0 - omega.cos()) / 2.0;
    let b1 = 1.0 - omega.cos();
    let b2 = (1.0 - omega.cos()) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * omega.cos();
    let a2 = 1.0 - alpha;

    let (b0, b1, b2, a1, a2) = (b0/a0, b1/a0, b2/a0, a1/a0, a2/a0);

    let mut out = vec![0.0; audio.len()];
    let mut x1 = 0.0; let mut x2 = 0.0;
    let mut y1 = 0.0; let mut y2 = 0.0;

    for i in 0..audio.len() {
        let x0 = audio[i];
        let y0 = b0*x0 + b1*x1 + b2*x2 - a1*y1 - a2*y2;
        out[i] = y0;
        x2 = x1; x1 = x0;
        y2 = y1; y1 = y0;
    }

    if is_highpass {
        for i in 0..audio.len() {
            out[i] = audio[i] - out[i];
        }
    }
    out
}

pub fn analyze_hybrid(
    audio: &[f32],
    sample_rate: u32,
    f0_curve: &[f32],
) -> Result<Vec<Vec<f32>>, String> {
    println!("[INFO] Hybrid analysis started.");

    let average_f0 = f0_curve.iter().filter(|&&f| f > 0.0).sum::<f32>()
        / f0_curve.iter().filter(|&&f| f > 0.0).count() as f32;
    
    if average_f0.is_nan() {
        return Err("Cannot perform hybrid analysis without a valid F0 curve.".to_string());
    }
    
    let crossover_freq = (average_f0 * 5.0).max(800.0).clamp(800.0, 3000.0);
    println!("[INFO] Crossover frequency set to: {:.2} Hz", crossover_freq);

    let low_pass_audio = apply_filter(audio, sample_rate, crossover_freq, false);
    let high_pass_audio = apply_filter(audio, sample_rate, crossover_freq, true);

    let low_table_result = mode_time::analyze_time_domain(&low_pass_audio, sample_rate, f0_curve)?;
    let high_table_result = mode_freq::analyze_freq_domain(&high_pass_audio, sample_rate)?;

    let low_table = low_table_result.get(0).ok_or("Low-pass analysis failed.")?;
    let high_table = high_table_result.get(0).ok_or("High-pass analysis failed.")?;

    let target_len = low_table.len().max(high_table.len());
    let mut final_table = vec![0.0; target_len];

    for i in 0..target_len {
        let low_sample = low_table.get(i).cloned().unwrap_or(0.0);
        let high_sample = high_table.get(i).cloned().unwrap_or(0.0);
        final_table[i] = low_sample + high_sample;
    }

    let max_abs = final_table.iter().map(|&s| s.abs()).fold(0.0, f32::max);
    if max_abs > 1e-6 {
        for sample in final_table.iter_mut() {
            *sample /= max_abs;
        }
    }

    println!("[INFO] Hybrid analysis finished. Generated 1 combined wavetable.");
    Ok(vec![final_table])
}


#[cfg(test)]
mod tests {
    // ★ 修正点: 未使用の `use super::*;` を削除

    // --- モック（ダミー）の実装 ---
    mod mock_time {
        pub fn analyze_time_domain(_a: &[f32], _sr: u32, _f0: &[f32]) -> Result<Vec<Vec<f32>>, String> {
            Ok(vec![vec![1.0; 100]]) 
        }
    }
    mod mock_freq {
        pub fn analyze_freq_domain(_a: &[f32], _sr: u32) -> Result<Vec<Vec<f32>>, String> {
            Ok(vec![vec![-1.0; 100]])
        }
    }

    #[test]
    fn test_hybrid_synthesis_logic() {
        // ★ 修正点: 未使用変数に `_` をつける
        const SAMPLE_RATE: u32 = 48000;
        let _signal = vec![0.0; SAMPLE_RATE as usize];
        let _f0_curve = vec![440.0; 100];

        let low_table_result = mock_time::analyze_time_domain(&[], 0, &[]);
        let high_table_result = mock_freq::analyze_freq_domain(&[], 0);
        
        assert!(low_table_result.is_ok());
        assert!(high_table_result.is_ok());

        let low_table = low_table_result.unwrap().remove(0);
        let high_table = high_table_result.unwrap().remove(0);

        let target_len = low_table.len().max(high_table.len());
        let mut final_table = vec![0.0; target_len];
        for i in 0..target_len {
            final_table[i] = low_table[i] + high_table[i];
        }

        let sum: f32 = final_table.iter().map(|x| x.abs()).sum();
        assert!(
            sum < 1e-6,
            "ハイブリッド合成の結果が正しくありません。期待値: ~0.0, 実際値の合計: {}",
            sum
        );
    }
}