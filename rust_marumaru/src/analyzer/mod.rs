// src/analyzer/mod.rs

// 各モジュールを公開
pub mod types;
pub mod preprocess;
pub mod f0_estimator;
pub mod mode_time;
pub mod mode_freq;
pub mod mode_hybrid;
pub mod dynamic_pitch;
pub mod quality;

// ★ 修正点: 未使用の型を削除
pub use self::types::{AnalysisResult};


/// 音声データを解析するメイン関数（最終版）
pub fn analyze_audio(
    audio_slice: &[f32],
    sample_rate: u32,
    core_end_ratio: f32, 
    release_start_ratio: f32, 
) -> Result<AnalysisResult, String> {

    // 1. 前処理
    println!("Applying preprocessing...");
    let processed_audio = preprocess::apply_all_preprocessing(audio_slice)?;

    // 2. F0推定
    println!("Estimating F0 curve...");
    let (f0_curve, confidence) = f0_estimator::estimate_f0_curve(&processed_audio, sample_rate)?;
    
    // 3. 音響指標計算 (周期性 = F0の平均信頼度)
    let periodicity = if !confidence.is_empty() {
        confidence.iter().sum::<f32>() / confidence.len() as f32
    } else {
        0.0 // F0が全く検出できなかった場合
    };
    println!("[INFO] Acoustic feature calculated. Periodicity = {:.3}", periodicity);

    // 4. モード判定と実行
    let tables = match periodicity {
        p if p > 0.6 => {
            println!("[INFO] Mode selected: Time Domain (High Periodicity)");
            mode_time::analyze_time_domain(&processed_audio, sample_rate, &f0_curve)?
        },
        p if p > 0.35 => {
            println!("[INFO] Mode selected: Hybrid (Medium Periodicity)");
            mode_hybrid::analyze_hybrid(&processed_audio, sample_rate, &f0_curve)?
        },
        _ => {
            println!("[INFO] Mode selected: Frequency Domain (Low Periodicity)");
            mode_freq::analyze_freq_domain(&processed_audio, sample_rate)?
        }
    };
    
    // 5. DynamicPitchSync (ピッチ揺れ正規化)
    let final_tables = dynamic_pitch::apply_pitch_sync(&tables, &f0_curve)?;
    
    // ★ 5.5. 振幅プロファイルの抽出と分割ロジック
    // 5.5.1. 振幅プロファイル（RMS）の抽出
    let mut amp_curve = Vec::new();
    let window_size = 512; 
    for chunk in audio_slice.chunks(window_size) { 
        let rms = chunk.iter().map(|&s| s * s).sum::<f32>() / chunk.len() as f32;
        amp_curve.push(rms.sqrt());
    }
    
    // 5.5.2. ゲインカーブをリサンプリングし、波形と同じ長さに正規化
    let total_len = final_tables.get(0).map_or(0, |t| t.len());
    let resampled_gain = if total_len > 0 && !amp_curve.is_empty() {
        // RMSカーブを波形長に線形リサンプリング
        let mut resampled = vec![0.0; total_len];
        let amp_curve_len = amp_curve.len() as f32;
        for i in 0..total_len {
            let index_f = i as f32 / total_len as f32 * amp_curve_len;
            let idx0 = index_f.floor() as usize;
            let idx1 = (idx0 + 1).min(amp_curve.len().saturating_sub(1));
            let frac = index_f - idx0 as f32;
            resampled[i] = amp_curve[idx0] * (1.0 - frac) + amp_curve[idx1] * frac;
        }
        resampled
    } else {
        vec![1.0; total_len]
    };

    // 5.5.3. Core/Loop/Releaseのインデックスを計算
    let core_end_ratio = core_end_ratio.clamp(0.0, 1.0);
    let release_start_ratio = release_start_ratio.clamp(0.0, 1.0);
    
    let core_end_idx = (total_len as f32 * core_end_ratio).round() as usize;
    let release_start_idx = (total_len as f32 * release_start_ratio).round() as usize;
    
    let safe_core_end_idx = core_end_idx.min(release_start_idx);
    let safe_release_start_idx = release_start_idx.max(core_end_idx);

    // 5.5.4. 波形とゲインカーブを分割
    let main_table = final_tables.get(0)
        .ok_or_else(|| "Final table is empty. Cannot split sections.".to_string())?;

    let core_wave = main_table[0..safe_core_end_idx.min(total_len)].to_vec();
    let loop_wave = main_table[safe_core_end_idx.min(total_len)..safe_release_start_idx.min(total_len)].to_vec();
    let release_wave = main_table[safe_release_start_idx.min(total_len)..].to_vec();

    let mut core_gain = resampled_gain[0..safe_core_end_idx.min(total_len)].to_vec();
    let loop_gain = resampled_gain[safe_core_end_idx.min(total_len)..safe_release_start_idx.min(total_len)].to_vec();
    let release_gain = resampled_gain[safe_release_start_idx.min(total_len)..].to_vec();

    // ★ 5.5.5. 「必ずゼロから始まる」条件を強制 (Coreゲインカーブの最初の10サンプルを滑らかにゼロから立ち上げる)
    if !core_gain.is_empty() {
        let fade_len = core_gain.len().min(10);
        for i in 0..fade_len {
            let t = i as f32 / fade_len as f32;
            core_gain[i] *= t; 
            if i == 0 { core_gain[i] = 0.0; } // 念のため最初のサンプルはゼロ保証
        }
    }
    
    // 6. 品質検査
    let quality_metrics = quality::inspect_quality(
        audio_slice,
        &final_tables,
        &f0_curve,
        sample_rate
    )?;
    
    // 最終的な解析結果を返す
    Ok(AnalysisResult {
        f0_curve,
        confidence,
        tables: final_tables,
        core_wave,
        loop_wave,
        release_wave,
        core_gain,
        loop_gain,
        release_gain,
        quality: quality_metrics,
    })
}