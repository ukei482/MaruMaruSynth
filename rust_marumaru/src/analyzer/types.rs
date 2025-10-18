// analyzer/types.rs

/// 品質指標を格納する構造体
#[derive(Debug, Clone, PartialEq)]
pub struct QualityMetrics {
    pub correlation: f32,       // 平均相関
    pub spectral_residual: f32, // スペクトル残差
    pub nan_ratio: f32,         // NaN率
}

/// 解析結果を格納する構造体
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub f0_curve: Vec<f32>,
    pub confidence: Vec<f32>,
    pub tables: Vec<Vec<f32>>,
    pub core_wave: Vec<f32>,
    pub loop_wave: Vec<f32>,
    pub release_wave: Vec<f32>,
    // ★ 修正点: ゲインカーブを追加
    pub core_gain: Vec<f32>,    // Coreセクションの振幅プロファイル
    pub loop_gain: Vec<f32>,    // Loopセクションの振幅プロファイル
    pub release_gain: Vec<f32>, // Releaseセクションの振幅プロファイル
    pub quality: QualityMetrics,
}

/// 解析モードの選択
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisMode {
    Wavetable,
    Additive,
}

/// 解析プロファイルの選択
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisProfile {
    Natural, // 自然音向け
    Electronic, // 電子音向け
}

// 他にもFFTサイズや窓関数など、共通で使うパラメータがあればここに追加します
pub struct AnalysisParams {
    pub fft_size: usize,
    pub hop_size: usize,
    // ...
}