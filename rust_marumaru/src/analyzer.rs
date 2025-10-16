// src/analyzer.rs

use std::fmt;

//==============================================================================
//
//  Public Data Structures (analyzer_design.md に基づく)
//
//==============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalysisMode {
    Additive,
    Wavetable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalysisProfile {
    Natural,
    Percussive,
    Metallic,
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhaseStrategy {
    Preserve,
    ResetZero,
    AlignEnergyPeak,
    Hybrid(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowFunction {
    Hann,
    Blackman,
    Hamming,
    Rectangular,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Harmonic {
    pub frequency: f32,
    pub amplitude: f32,
    pub phase: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdditiveResult {
    pub harmonics: Vec<Harmonic>, // 1フレーム分の倍音情報
    pub pitch_curve: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WavetableResult {
    pub table: Vec<f32>, // 1フレーム分のウェーブテーブル
    pub pitch_curve: Vec<f32>,
}

/// 解析モードに応じて異なる結果を格納する
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisResult {
    Additive(AdditiveResult),
    Wavetable(WavetableResult),
}

/// 解析パラメータ
#[derive(Debug, Clone)]
pub struct AnalysisParams {
    pub mode: AnalysisMode,
    pub profile: AnalysisProfile,
    pub fft_size: usize,
    pub hop_size: usize,
    pub window: WindowFunction,
    pub phase_strategy: Option<PhaseStrategy>,
    pub normalize: bool,
    pub table_length: Option<usize>,
}

impl AnalysisParams {
    /// プロファイルに基づいてデフォルトのパラメータを生成する
    pub fn from_profile(mode: AnalysisMode, profile: AnalysisProfile) -> Self {
        match profile {
            AnalysisProfile::Natural => Self {
                mode, profile, fft_size: 4096, hop_size: 1024,
                window: WindowFunction::Hann, phase_strategy: Some(PhaseStrategy::Preserve),
                normalize: true, table_length: Some(2048),
            },
            AnalysisProfile::Percussive => Self {
                mode, profile, fft_size: 1024, hop_size: 256,
                window: WindowFunction::Blackman, phase_strategy: Some(PhaseStrategy::AlignEnergyPeak),
                normalize: true, table_length: Some(2048),
            },
            AnalysisProfile::Metallic => Self {
                mode, profile, fft_size: 2048, hop_size: 512,
                window: WindowFunction::Hamming, phase_strategy: Some(PhaseStrategy::Hybrid(0.5)),
                normalize: true, table_length: Some(2048),
            },
            AnalysisProfile::Synthetic => Self {
                mode, profile, fft_size: 1024, hop_size: 256,
                window: WindowFunction::Rectangular, phase_strategy: Some(PhaseStrategy::ResetZero),
                normalize: true, table_length: Some(2048),
            },
        }
    }
}


//==============================================================================
//
//  Error Handling
//
//==============================================================================

#[derive(Debug)]
pub struct AnalyzerError(String);

impl fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Analyzer Error: {}", self.0)
    }
}

//==============================================================================
//
//  Analyzer Core Implementation
//
//==============================================================================

pub struct Analyzer {
    pub sample_rate: u32,
}

impl Analyzer {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// `analyzer_design.md` の analyze 関数に相当するディスパッチャ
    pub fn analyze(
        &self,
        audio_frame: &[f32],
        params: &AnalysisParams,
    ) -> Result<AnalysisResult, AnalyzerError> {
        match params.mode {
            AnalysisMode::Additive => self.analyze_additive(audio_frame, params),
            AnalysisMode::Wavetable => self.analyze_wavetable(audio_frame, params),
        }
    }

    /// 加算合成用の解析処理
    fn analyze_additive(
        &self,
        _audio_frame: &[f32],
        _params: &AnalysisParams,
    ) -> Result<AnalysisResult, AnalyzerError> {
        // TODO: FFT、ピーク検出、ピッチ抽出などの実際の信号処理を実装
        println!("Performing Additive analysis...");

        // ダミーデータを返す
        Ok(AnalysisResult::Additive(AdditiveResult {
            harmonics: vec![
                Harmonic { frequency: 440.0, amplitude: 1.0, phase: 0.0 },
                Harmonic { frequency: 880.0, amplitude: 0.5, phase: 0.0 },
            ],
            pitch_curve: vec![440.0, 441.0],
        }))
    }

    /// ウェーブテーブル用の解析処理
    fn analyze_wavetable(
        &self,
        _audio_frame: &[f32],
        _params: &AnalysisParams,
    ) -> Result<AnalysisResult, AnalyzerError> {
        // TODO: FFT、位相補正、IFFTなどの実際の信号処理を実装
        println!("Performing Wavetable analysis...");
        
        let table_len = _params.table_length.unwrap_or(2048);
        let mut dummy_table = vec![0.0; table_len];
        for i in 0..table_len { // 簡単なサイン波を生成
            dummy_table[i] = (i as f32 / table_len as f32 * 2.0 * std::f32::consts::PI).sin();
        }

        Ok(AnalysisResult::Wavetable(WavetableResult {
            table: dummy_table,
            pitch_curve: vec![440.0, 441.0],
        }))
    }
}

//==============================================================================
//
//  Top-Level API for lib.rs
//
//==============================================================================

/// `設計.md` で定義された3領域の解析結果を保持する構造体
#[derive(Debug, Clone)]
pub struct FullAnalysisData {
    pub attack: AnalysisResult,
    pub core: AnalysisResult,
    pub sustain: AnalysisResult,
}

/// lib.rsから呼び出される、音声全体を3分割して解析するトップレベル関数
pub fn analyze_audio(
    audio_slice: &[f32],
    sample_rate: u32,
    attack_range: (usize, usize),
    core_range: (usize, usize),
    sustain_range: (usize, usize),
    // UIなどから指定されるパラメータ
    mode: AnalysisMode,
    profile: AnalysisProfile,
) -> Result<FullAnalysisData, AnalyzerError> {

    if attack_range.1 > audio_slice.len() || core_range.1 > audio_slice.len() || sustain_range.1 > audio_slice.len() {
        return Err(AnalyzerError("Analysis range exceeds audio buffer size.".to_string()));
    }

    let analyzer = Analyzer::new(sample_rate);
    let params = AnalysisParams::from_profile(mode, profile);

    // 各領域を解析
    let attack_result = analyzer.analyze(&audio_slice[attack_range.0..attack_range.1], &params)?;
    let core_result = analyzer.analyze(&audio_slice[core_range.0..core_range.1], &params)?;
    let sustain_result = analyzer.analyze(&audio_slice[sustain_range.0..sustain_range.1], &params)?;
    
    Ok(FullAnalysisData {
        attack: attack_result,
        core: core_result,
        sustain: sustain_result,
    })
}