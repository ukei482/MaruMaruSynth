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
    
    // 5. DynamicPitchSync (プレースホルダー)
    let final_tables = dynamic_pitch::apply_pitch_sync(&tables, &f0_curve)?;
    
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
        quality: quality_metrics,
    })
}