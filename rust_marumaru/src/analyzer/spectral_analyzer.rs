// src/analyzer/spectral_analyzer.rs

use rustfft::{num_complex::Complex, FftPlanner};

//==============================================================================
// 公開データ構造体
//==============================================================================

/// 1つの領域（Attack, Core, Sustain）のスペクトル解析結果
#[derive(Debug, Clone)]
pub struct SpectralRegion {
    pub spectrum_data: Vec<(f32, f32, f32)>,  // スペクトルデータ: Vec<(周波数: f32, 振幅: f32, 位相: f32)>
    pub wavetable    : Vec<f32>,              // IFFTによって生成された単一サイクルの波形データ（ウェーブテーブル）
}
 
/// 音声ファイル全体の解析結果を保持する構造体
#[derive(Debug, Clone)]
pub struct AnalysisData {
    pub sample_rate    : u32,
    pub attack_region  : SpectralRegion,
    pub core_region    : SpectralRegion,
    pub sustain_region : SpectralRegion,
    pub pitch_contour  : Vec<(f32, f32)>, // 抽出されたピッチの輪郭データ: Vec<(時間: f32, ピッチ偏移: f32)>
}


//==============================================================================
// 公開関数
//==============================================================================

/// ## analyze_audio
/// 指定された領域情報に基づき、音声データ全体を解析するトップレベル関数。
///
/// ### 引数
/// * `audio_buffer` - モノラルの音声データ
/// * `sample_rate` - サンプリング周波数
/// * `attack_range`, `core_range`, `sustain_range` - 各領域の (開始サンプル, 終了サンプル)


pub fn analyze_audio(
    audio_buffer  : &[f32],
    sample_rate   : u32,
    attack_range  : (usize, usize),
    core_range    : (usize, usize),
    sustain_range : (usize, usize),
) -> Result<AnalysisData, String> {

    // 1. 各領域の音声データをスライスで切り出す
    //    範囲外アクセスの場合はエラーを返す
    let attack_samples = audio_buffer.get(attack_range.0..attack_range.1)
        .ok_or("Attack range is out of bounds")?;
    let core_samples = audio_buffer.get(core_range.0..core_range.1)
        .ok_or("Core range is out of bounds")?;
    let sustain_samples = audio_buffer.get(sustain_range.0..sustain_range.1)
        .ok_or("Sustain range is out of bounds")?;

    // 2. 各スライスに対して解析処理を実行
    let attack_region  = process_region(attack_samples, sample_rate)?;
    let core_region    = process_region(core_samples, sample_rate)?;
    let sustain_region = process_region(sustain_samples, sample_rate)?;

    // 3. ピッチ輪郭を抽出（現在はダミー実装）
    let pitch_contour = extract_pitch_contour(audio_buffer, sample_rate)?;

    // 4. すべてのデータをAnalysisDataにまとめて返す
    Ok(AnalysisData {
        sample_rate,
        attack_region,
        core_region,
        sustain_region,
        pitch_contour,
    })
}


//==============================================================================
// ヘルパー関数
//==============================================================================

/// ## process_region
/// 1つのオーディオ領域を処理し、スペクトルとウェーブテーブルを生成する。
fn process_region(samples: &[f32], sample_rate: u32) -> Result<SpectralRegion, String> {
    if samples.is_empty() {
        return Err("Cannot process an empty audio region.".to_string());
    }

    // --- FFTの実行 ---
    let mut planner   = FftPlanner::new();
    let fft         = planner.plan_fft_forward(samples.len());
    let mut buffer: Vec<Complex<f32>>  = samples
        .iter()
        .map(|&sample| Complex { re: sample, im: 0.0 })
        .collect();
    fft.process(&mut buffer);

    // --- スペクトルデータを抽出 (周波数, 振幅, 位相) ---
    let fft_len                          = buffer.len();
    let mut spectrum_data = Vec::new();

    // FFT結果の半分（ナイキスト周波数まで）をループ
    for i in 0..(fft_len / 2) {
        let complex_val = buffer[i];
        let freq                 = (i as f32 * sample_rate as f32) / fft_len as f32;
        
        // 振幅を正規化 (直流成分(i=0)以外は2倍する)
        let amplitude = if i > 0 {
            complex_val.norm() * 2.0 / fft_len as f32
        } else {
            complex_val.norm() / fft_len as f32
        };

        let phase = complex_val.arg(); // 位相 (atan2)
        
        spectrum_data.push((freq, amplitude, phase));
    }

    // --- IFFTによるウェーブテーブル生成 ---
    let mut ifft_planner = FftPlanner::new();
    let ifft = ifft_planner.plan_fft_inverse(fft_len);
    
    // bufferはFFTで消費されたのでクローンして使う
    let mut ifft_buffer = buffer.clone();
    ifft.process(&mut ifft_buffer);

    let wavetable: Vec<f32> = ifft_buffer
        .iter()
        .map(|c| c.re / fft_len as f32) // 正規化
        .collect();

    Ok(SpectralRegion {
        spectrum_data,
        wavetable,
    })
}

/// ## extract_pitch_contour
/// 音声データからピッチの輪郭を抽出する（TODO: 将来的な実装箇所）。
fn extract_pitch_contour(_samples: &[f32], _sample_rate: u32) -> Result<Vec<(f32, f32)>, String> {
    // ピッチ検出は複雑なアルゴリズム（YIN, AMDFなど）が必要なため、
    // ここではダミーとして空の配列を返す。
    Ok(vec![])
}