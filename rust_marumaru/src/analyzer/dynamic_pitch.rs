// src/analyzer/dynamic_pitch.rs

/// 線形補間を使ってオーディオ波形から特定の位置のサンプル値を取得するヘルパー関数
fn sample_linear(wave: &[f32], index_f: f32) -> f32 {
    if wave.is_empty() || index_f < 0.0 || index_f >= (wave.len() - 1) as f32 {
        return 0.0;
    }
    let idx0 = index_f.floor() as usize;
    let idx1 = idx0 + 1;
    let frac = index_f - idx0 as f32;
    wave[idx0] * (1.0 - frac) + wave[idx1] * frac
}

/// DynamicPitchSyncを適用してウェーブテーブルのピッチ揺れを正規化する
pub fn apply_pitch_sync(
    tables: &[Vec<f32>],
    f0_curve: &[f32],
) -> Result<Vec<Vec<f32>>, String> {
    println!("[INFO] DynamicPitchSync started.");

    // F0カーブが無効な場合は何もしない
    let average_f0: f32 = f0_curve.iter().filter(|&&f| f > 0.0).sum::<f32>()
        / f0_curve.iter().filter(|&&f| f > 0.0).count() as f32;
    if average_f0.is_nan() || average_f0 < 1.0 {
        println!("[WARN] Invalid F0 curve for DynamicPitchSync. Skipping.");
        return Ok(tables.to_vec());
    }

    let mut final_tables = Vec::new();

    for table in tables {
        let table_len = table.len();
        if table_len == 0 {
            final_tables.push(Vec::new());
            continue;
        }

        let mut rescaled_table = vec![0.0; table_len];
        let mut current_pos = 0.0; // 元のテーブルを読む位置

        // F0カーブの1フレームがウェーブテーブルの何サンプルに対応するか
        // HOP_SIZE(512) / sample_rate * average_f0 * table_len
        // を簡略化して計算
        let f0_frames_per_table = f0_curve.len() as f32;

        for i in 0..table_len {
            // 現在位置(i)がF0カーブのどのインデックスに対応するか
            let f0_idx_f = (i as f32 / table_len as f32) * f0_frames_per_table;
            let f0_idx = f0_idx_f.round() as usize;

            let current_f0 = f0_curve.get(f0_idx).cloned().unwrap_or(average_f0);

            // 読み出し速度をピッチのズレに応じて調整
            // ピッチが高い -> 速く読む, ピッチが低い -> 遅く読む
            let speed_ratio = if current_f0 > 0.0 { current_f0 / average_f0 } else { 1.0 };

            rescaled_table[i] = sample_linear(table, current_pos);

            current_pos += speed_ratio;
            if current_pos >= table_len as f32 {
                current_pos -= table_len as f32; // ループさせる
            }
        }
        final_tables.push(rescaled_table);
    }

    println!("[INFO] DynamicPitchSync finished.");
    Ok(final_tables)
}