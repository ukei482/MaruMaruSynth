// src/analyzer/quality.rs
use super::types::QualityMetrics;
use rustfft::{FftPlanner, num_complex::Complex};

/// 線形補間を使ってウェーブテーブルからサンプルを読み出すヘルパー関数
fn sample_table_linear(table: &[f32], phase: f32) -> f32 {
    let table_len = table.len() as f32;
    if table_len < 2.0 { return 0.0; }
    let index_f = phase * table_len;
    let idx0 = index_f.floor() as usize;
    let idx1 = (idx0 + 1) % table.len();
    let frac = index_f - idx0 as f32;
    table[idx0] * (1.0 - frac) + table[idx1] * frac
}

/// ウェーブテーブルとF0カーブから音声を再合成する
fn resynthesize_audio(
    table: &[f32],
    f0_curve: &[f32],
    sample_rate: u32,
    output_len: usize,
) -> Vec<f32> {
    if table.is_empty() || f0_curve.is_empty() {
        return vec![0.0; output_len];
    }
    let mut output = vec![0.0; output_len];
    let mut phase = 0.0; // 0.0 ~ 1.0
    let hop_size = 512; // f0_estimator.rs と合わせる

    for i in 0..output_len {
        let frame_idx = (i / hop_size).min(f0_curve.len() - 1);
        let f0 = f0_curve[frame_idx];

        if f0 > 0.0 {
            phase += f0 / sample_rate as f32;
            if phase >= 1.0 {
                phase -= 1.0;
            }
        }
        output[i] = sample_table_linear(table, phase);
    }
    output
}


/// 解析結果の品質を検査する
pub fn inspect_quality(
    original_audio: &[f32],
    final_tables: &[Vec<f32>],
    f0_curve: &[f32], // F0カーブを引数として受け取るように変更
    sample_rate: u32,  // サンプルレートを引数として受け取るように変更
) -> Result<QualityMetrics, String> {
    println!("[INFO] Quality inspection started.");

    // 代表として最初のテーブルを使用
    let main_table = match final_tables.get(0) {
        Some(table) => table,
        None => return Ok(QualityMetrics { correlation: 0.0, spectral_residual: 1.0, nan_ratio: 0.0 }),
    };

    // 1. 音声を再合成
    let resynthesized_audio = resynthesize_audio(main_table, f0_curve, sample_rate, original_audio.len());

    // 2. 原音と再構成音の相関を計算 (簡易版: 正規化されたドット積)
    let mean_orig: f32 = original_audio.iter().sum::<f32>() / original_audio.len() as f32;
    let mean_resynth: f32 = resynthesized_audio.iter().sum::<f32>() / resynthesized_audio.len() as f32;
    
    let mut cov = 0.0;
    let mut var_orig = 0.0;
    let mut var_resynth = 0.0;

    for i in 0..original_audio.len() {
        let v_orig = original_audio[i] - mean_orig;
        let v_resynth = resynthesized_audio[i] - mean_resynth;
        cov += v_orig * v_resynth;
        var_orig += v_orig.powi(2);
        var_resynth += v_resynth.powi(2);
    }
    let correlation = cov / (var_orig.sqrt() * var_resynth.sqrt());

    // 3. スペクトル残差を計算
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(original_audio.len().next_power_of_two());
    
    let mut orig_spec = original_audio.to_vec();
    orig_spec.resize(fft.len(), 0.0);
    let mut resynth_spec = resynthesized_audio;
    resynth_spec.resize(fft.len(), 0.0);

    let mut orig_complex: Vec<_> = orig_spec.into_iter().map(|s| Complex::new(s, 0.0)).collect();
    let mut resynth_complex: Vec<_> = resynth_spec.into_iter().map(|s| Complex::new(s, 0.0)).collect();

    fft.process(&mut orig_complex);
    fft.process(&mut resynth_complex);

    let mut residual_sum = 0.0;
    let mut orig_power_sum = 0.0;
    for i in 0..fft.len() / 2 {
        let mag_diff = orig_complex[i].norm() - resynth_complex[i].norm();
        residual_sum += mag_diff.powi(2);
        orig_power_sum += orig_complex[i].norm().powi(2);
    }
    let spectral_residual = (residual_sum / orig_power_sum).sqrt();

    println!("[INFO] Quality inspection finished.");
    Ok(QualityMetrics {
        correlation: correlation.max(0.0),
        spectral_residual: spectral_residual.max(0.0),
        nan_ratio: 0.0, // TODO: f0_curveのNaN率を計算
    })
}