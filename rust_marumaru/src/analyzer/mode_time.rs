// src/analyzer/mode_time.rs

/// 線形補間を使ってオーディオ波形をリサンプリングするヘルパー関数
fn resample_linear(wave: &[f32], target_len: usize) -> Vec<f32> {
    if wave.is_empty() || target_len == 0 {
        return vec![0.0; target_len];
    }
    let mut resampled = vec![0.0; target_len];
    let scale = (wave.len() - 1) as f32 / (target_len - 1) as f32;

    for i in 0..target_len {
        let original_idx_f = i as f32 * scale;
        let idx0 = original_idx_f.floor() as usize;
        let idx1 = (idx0 + 1).min(wave.len() - 1);
        let frac = original_idx_f - idx0 as f32;

        resampled[i] = wave[idx0] * (1.0 - frac) + wave[idx1] * frac;
    }
    resampled
}

/// 時間領域での音声解析を行う
pub fn analyze_time_domain(
    audio: &[f32],
    sample_rate: u32,
    f0_curve: &[f32],
) -> Result<Vec<Vec<f32>>, String> {
    println!("[INFO] Time domain analysis started.");

    // 1. F0カーブから平均的な周期（サンプル数）を計算する
    let average_f0: f32 = f0_curve.iter().filter(|&&f| f > 0.0).sum::<f32>() 
        / f0_curve.iter().filter(|&&f| f > 0.0).count() as f32;
    
    if average_f0.is_nan() || average_f0 < 1.0 {
        return Err("Could not determine a valid average F0 from the curve.".to_string());
    }
    let target_period_len = (sample_rate as f32 / average_f0).round() as usize;

    if target_period_len < 2 {
        return Err("Average period is too short to process.".to_string());
    }

    let mut cycles = Vec::new();
    let mut current_pos = 0.0;
    
    // F0カーブの1フレームがオーディオの何サンプル分に対応するか
    let hop_size = 512; // f0_estimator.rs で定義したHOP_SIZEと合わせる
    
    // 2. F0カーブに従ってオーディオから1周期ずつ切り出す
    while (current_pos as usize) < audio.len() {
        let frame_idx = (current_pos / hop_size as f32).floor() as usize;
        let f0 = f0_curve.get(frame_idx).cloned().unwrap_or(average_f0);

        let period_len = if f0 > 0.0 {
            sample_rate as f32 / f0
        } else {
            target_period_len as f32
        };

        let start = current_pos.round() as usize;
        let end = (current_pos + period_len).round() as usize;

        if end > audio.len() { break; }
        
        let cycle_slice = &audio[start..end];
        cycles.push(cycle_slice.to_vec());
        
        current_pos += period_len;
    }

    if cycles.is_empty() {
        return Err("No cycles could be extracted from the audio.".to_string());
    }
    
    // 3. 全ての周期を平均周期長にリサンプリングし、4. 平均化する
    let mut averaged_table = vec![0.0; target_period_len];
    for cycle in cycles.iter() {
        let resampled_cycle = resample_linear(cycle, target_period_len);
        for i in 0..target_period_len {
            averaged_table[i] += resampled_cycle[i];
        }
    }

    for sample in averaged_table.iter_mut() {
        *sample /= cycles.len() as f32;
    }

    // 5. 波形を -1.0 ~ 1.0 の範囲に正規化する
    let max_abs = averaged_table.iter().map(|&s| s.abs()).fold(0.0, f32::max);
    if max_abs > 1e-6 {
        for sample in averaged_table.iter_mut() {
            *sample /= max_abs;
        }
    }
    
    println!("[INFO] Time domain analysis finished. Generated 1 wavetable.");
    Ok(vec![averaged_table])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_domain_on_pure_sine() {
        // 1. Arrange: テストデータと完璧なF0カーブを準備
        const SAMPLE_RATE: u32 = 48000;
        const SIGNAL_FREQ: f32 = 440.0;
        const SIGNAL_LEN: usize = SAMPLE_RATE as usize; // 1秒

        let mut signal = Vec::new();
        for i in 0..SIGNAL_LEN {
            let time = i as f32 / SAMPLE_RATE as f32;
            let sample = (2.0 * std::f32::consts::PI * SIGNAL_FREQ * time).sin();
            signal.push(sample);
        }

        // 常に440Hzを返す、完璧なF0カーブ
        let f0_curve = vec![SIGNAL_FREQ; 100];

        // 2. Act: 時間領域解析を実行
        let result = analyze_time_domain(&signal, SAMPLE_RATE, &f0_curve);
        assert!(result.is_ok());
        let tables = result.unwrap();

        // 3. Assert: 結果を検証
        assert_eq!(tables.len(), 1, "ウェーブテーブルは1つだけ生成されるべきです");
        let table = &tables[0];

        // 期待されるテーブル長を計算 (48000 / 440 = 109.09... -> 109)
        let expected_len = (SAMPLE_RATE as f32 / SIGNAL_FREQ).round() as usize;
        assert_eq!(table.len(), expected_len, "ウェーブテーブルの長さが正しくありません");

        // 波形の主要なポイントを検証
        let peak_idx = (expected_len as f32 * 0.25).round() as usize;
        let trough_idx = (expected_len as f32 * 0.75).round() as usize;
        let tolerance = 0.05;

        // ピークは1.0に近いはず
        assert!(
            (table[peak_idx] - 1.0).abs() < tolerance,
            "波形のピークが正しくありません。期待値: ~1.0, 実際値: {}",
            table[peak_idx]
        );
        // トラフ（谷）は-1.0に近いはず
        assert!(
            (table[trough_idx] - -1.0).abs() < tolerance,
            "波形のトラフが正しくありません。期待値: ~-1.0, 実際値: {}",
            table[trough_idx]
        );
    }
}