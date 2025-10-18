// src/oscillator/mod.rs

// use std::ops::Rem; // ★ 削除 (std::ops::Remはグローバルではなく、f32のメソッドとして利用される)

// 各セクションの実装モジュールを公開
pub mod core;
pub mod r#loop; // 'loop'はRustのキーワードのため r#loop と表記
pub mod release;
pub mod wavetable;
pub mod fm;
pub mod additive;

// --- 新規追加: 再生状態 ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    Off,
    Core,
    Loop,
    Release,
}

// --- 構造体と列挙体の定義 (変更なし) ---

/// 発振モード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscillatorMode {
    FM,
    Additive,
}

/// 複数OSCのミックスモード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixMode {
    FM,
    Add,
}

/// 各セクション（Core/Loop/Release）の波形データを表す
#[derive(Debug, Clone)]
pub struct WaveSection {
    // 解析結果の波形を保持 (時間軸に沿ったサンプリング波形)
    pub wavetable: Vec<f32>,       
    pub crossfade: bool,           // クロスフェード有無 (Loopセクションで使用)
}

impl WaveSection {
    pub fn new(wavetable: Vec<f32>) -> Self {
        Self {
            wavetable,
            crossfade: false, // デフォルトではクロスフェード無効
        }
    }
    
    pub fn len(&self) -> usize {
        self.wavetable.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.wavetable.is_empty()
    }
}

// --- 線形補間ヘルパー関数 ---
/// 線形補間を使ってウェーブテーブルからサンプルを読み出す
fn sample_linear(wave: &[f32], index_f: f32) -> f32 {
    let wave_len = wave.len() as f32;
    if wave_len < 1.0 { return 0.0; }
    
    // インデックスを 0 から wave_len の間に正規化 (ループ用)
    let index_f_wrapped = index_f.rem_euclid(wave_len);
    
    // 補間
    let idx0 = index_f_wrapped.floor() as usize;
    let idx1 = (idx0 + 1) % wave.len();
    let frac = index_f_wrapped - idx0 as f32;

    wave[idx0] * (1.0 - frac) + wave[idx1] * frac
}


/// 単一OSCを表す構造体
#[derive(Debug)]
pub struct OscillatorUnit {
    pub core: WaveSection,
    pub loop_section: WaveSection,
    pub release: WaveSection,
    
    pub core_gain: Vec<f32>,
    pub loop_gain: Vec<f32>,
    pub release_gain: Vec<f32>,
    
    pub mode: OscillatorMode, 
    pub sample_rate: f32,
    pub position: f32, 
    pub frequency: f32,
    pub play_mode: PlayMode, 
    
    // FM/Additive 合成用の追加パラメータ
    pub level: f32,      // OSCの音量レベル (0.0 - 1.0)
    pub ratio: f32,      // FM変調比 (Carrier/Modulator)
    pub modulation_index: f32, // FM変調強度 (Osc2のみが使用)
    pub feedback: f32,   // フィードバック量 (FM/Additive合成の拡張用)
    
    // FM合成用の内部状態
    pub fm_phase: f32,   // FM合成のための位相 (0.0〜1.0)
    pub fm_output: f32,  // 1サンプル前の出力 (フィードバック用)
}

impl OscillatorUnit {
    pub fn new(sample_rate: f32) -> Self {
        OscillatorUnit {
            core: WaveSection::new(vec![]),
            loop_section: WaveSection::new(vec![]),
            release: WaveSection::new(vec![]),
            core_gain: vec![],
            loop_gain: vec![],
            release_gain: vec![],
            mode: OscillatorMode::Additive, 
            sample_rate,
            position: 0.0,
            frequency: 440.0,
            play_mode: PlayMode::Off,
            
            level: 1.0, 
            ratio: 1.0, 
            modulation_index: 0.0,
            feedback: 0.0,
            
            fm_phase: 0.0, 
            fm_output: 0.0,
        }
    }

    /// FFIからデータをロードするヘルパー関数
    pub unsafe fn load_data_from_ffi(
        &mut self,
        core_ptr: *const f32,
        core_len: usize,
        loop_ptr: *const f32,
        loop_len: usize,
        release_ptr: *const f32,
        release_len: usize,
        core_gain_ptr: *const f32,     
        core_gain_len: usize,          
        loop_gain_ptr: *const f32,     
        loop_gain_len: usize,          
        release_gain_ptr: *const f32,  
        release_gain_len: usize,       
    ) {
        // --- Wave Data Load ---
        if !core_ptr.is_null() && core_len > 0 {
            let core_slice = std::slice::from_raw_parts(core_ptr, core_len);
            self.core = WaveSection::new(core_slice.to_vec());
        } else {
             self.core = WaveSection::new(vec![]);
        }

        if !loop_ptr.is_null() && loop_len > 0 {
            let loop_slice = std::slice::from_raw_parts(loop_ptr, loop_len);
            self.loop_section = WaveSection::new(loop_slice.to_vec());
            self.loop_section.crossfade = true; // ループセクションのクロスフェードを有効化
        } else {
             self.loop_section = WaveSection::new(vec![]);
        }

        if !release_ptr.is_null() && release_len > 0 {
            let release_slice = std::slice::from_raw_parts(release_ptr, release_len);
            self.release = WaveSection::new(release_slice.to_vec());
        } else {
             self.release = WaveSection::new(vec![]);
        }
        
        // --- Gain Data Load ---
        if !core_gain_ptr.is_null() && core_gain_len > 0 {
            let core_gain_slice = std::slice::from_raw_parts(core_gain_ptr, core_gain_len);
            self.core_gain = core_gain_slice.to_vec();
        } else {
            self.core_gain = vec![];
        }

        if !loop_gain_ptr.is_null() && loop_gain_len > 0 {
            let loop_gain_slice = std::slice::from_raw_parts(loop_gain_ptr, loop_gain_len);
            self.loop_gain = loop_gain_slice.to_vec();
        } else {
            self.loop_gain = vec![];
        }

        if !release_gain_ptr.is_null() && release_gain_len > 0 {
            let release_gain_slice = std::slice::from_raw_parts(release_gain_ptr, release_gain_len);
            self.release_gain = release_gain_slice.to_vec();
        } else {
            self.release_gain = vec![];
        }
    }

    /// サンプルの生成ロジック
    pub fn generate_sample(&mut self, is_active: bool, _env_stage: usize) -> f32 {
        // 状態遷移の更新
        match (is_active, self.play_mode) {
            (true, PlayMode::Off) => {
                // 発音開始 (mm_note_onが呼ばれた直後)
                self.play_mode = PlayMode::Core;
                self.position = 0.0;
            },
            (false, PlayMode::Core) | (false, PlayMode::Loop) => {
                // ノートオフ (mm_note_offが呼ばれた直後)
                self.play_mode = PlayMode::Release;
                self.position = 0.0;
            },
            _ => {} // その他の状態は維持
        }

        // ★ 修正点: outputに初期値を割り当て
        let mut output: f32 = 0.0;
        let mut gain: f32 = 1.0; 

        match self.play_mode {
            PlayMode::Core => {
                let core_len = self.core.wavetable.len() as f32;
                let core_gain_len = self.core_gain.len();
                if core_len < 2.0 {
                    self.play_mode = PlayMode::Loop; 
                    self.position = 0.0;
                } else if self.position < core_len - 1.0 {
                    // Core再生中は、インデックスを1.0ずつ進める (サンプラー的再生)
                    output = sample_linear(&self.core.wavetable, self.position);
                    
                    // Coreゲインを適用 (インデックスを四捨五入して読み出す)
                    let gain_index = self.position.round() as usize;
                    if gain_index < core_gain_len {
                        gain = self.core_gain[gain_index];
                    }
                    
                    self.position += 1.0;

                } else {
                    // Core再生終了 -> Loopへ移行
                    self.play_mode = PlayMode::Loop;
                    self.position = 0.0;
                    // Coreの最終サンプルを返す
                    output = sample_linear(&self.core.wavetable, core_len - 1.0);
                }
            },
            PlayMode::Loop => {
                let loop_len = self.loop_section.wavetable.len();
                let loop_gain_len = self.loop_gain.len();
                if loop_len < 2 { 
                    self.play_mode = PlayMode::Off; 
                } else {
                    // Loop再生中は、周波数に基づいてポジションを進める (ウェーブテーブル的再生)
                    let phase_inc_index = self.frequency * loop_len as f32 / self.sample_rate;

                    output = sample_linear(&self.loop_section.wavetable, self.position);
                    
                    // Loopゲインを適用 (線形補間で読み出す)
                    if loop_gain_len > 0 {
                        let gain_pos = self.position / loop_len as f32 * loop_gain_len as f32;
                        gain = sample_linear(&self.loop_gain, gain_pos);
                    }
                    
                    self.position += phase_inc_index;
                    // ループ処理 (位相を正規化)
                    if self.position >= loop_len as f32 {
                        self.position -= loop_len as f32;
                    }
                }
            },
            PlayMode::Release => {
                let release_len = self.release.wavetable.len() as f32;
                let release_gain_len = self.release_gain.len() as f32;
                
                if release_len < 2.0 {
                    self.play_mode = PlayMode::Off;
                } else if self.position < release_len - 1.0 {
                    output = sample_linear(&self.release.wavetable, self.position);
                    
                    // Releaseゲインを適用 (線形補間で読み出す)
                    if release_gain_len > 0.0 {
                        let gain_pos = self.position / release_len * release_gain_len;
                        gain = sample_linear(&self.release_gain, gain_pos);
                    }
                    
                    self.position += 1.0;

                } else {
                    // Release再生終了
                    self.play_mode = PlayMode::Off;
                    output = 0.0;
                }
            },
            PlayMode::Off => {
                output = 0.0;
            }
        }
        
        // FM合成のベース位相計算と更新
        let freq_ratio = self.frequency * self.ratio / self.sample_rate;
        self.fm_phase += freq_ratio; 
        self.fm_phase = self.fm_phase.rem_euclid(1.0);
        
        // 最終出力にゲインカーブが適用される前の値を保存（フィードバック用）
        self.fm_output = output * gain;
        
        output * gain
    }
}


/// 複数OSCを束ねて管理する
#[derive(Debug)]
pub struct OscillatorBank {
    pub oscillators: [OscillatorUnit; 3], // OSC 3基を想定
    pub mix_mode: MixMode, // FM or Add
    pub fm_mix: f32, // FM合成時のミックスバランス (0.0: Osc1, 1.0: Osc2)
}

impl OscillatorBank {
    pub fn new(sample_rate: f32) -> Self {
        OscillatorBank {
            oscillators: [
                OscillatorUnit::new(sample_rate),
                OscillatorUnit::new(sample_rate),
                OscillatorUnit::new(sample_rate),
            ],
            mix_mode: MixMode::Add,
            fm_mix: 0.5,
        }
    }

    /// バンク全体でサンプルを生成し、ミックスする
    pub fn process_bank(&mut self, is_active: bool, _env_stage: usize) -> f32 {
        
        match self.mix_mode {
            MixMode::Add => {
                // 加算合成: 個別にサンプルを生成し、加算する
                let sample1 = self.oscillators[0].generate_sample(is_active, _env_stage);
                let sample2 = self.oscillators[1].generate_sample(is_active, _env_stage);
                let sample3 = self.oscillators[2].generate_sample(is_active, _env_stage);

                // 各OSCのレベルを考慮して加算
                let output = sample1 * self.oscillators[0].level +
                              sample2 * self.oscillators[1].level +
                              sample3 * self.oscillators[2].level;
                
                // 3つのOSCのレベル合計で正規化（簡易的な音量調整）
                output / 3.0
            },
            MixMode::FM => {
                // FM合成: Osc2がOsc1を変調する (設計書に従う)
                
                // ★ 修正点: E0499回避のため、split_first_mut を連続使用
                // 1. キャリア(0)を取得
                let (carrier_osc, rest) = self.oscillators.split_first_mut().unwrap();
                // 2. モジュレータ(1)を取得
                let (modulator_osc, _rest) = rest.split_first_mut().unwrap();
                
                // 1. Modulator (Osc2) の出力 (変調信号) を計算
                let mod_sample = {
                    let loop_len = modulator_osc.loop_section.wavetable.len();
                    if loop_len < 2 { 0.0 } else {
                        // 位相を Modulatorのfm_phase に反映 (0.0〜1.0 -> 0〜len)
                        let current_fm_phase_index = modulator_osc.fm_phase * loop_len as f32;
                        let output = sample_linear(&modulator_osc.loop_section.wavetable, current_fm_phase_index);
                        
                        // Modulatorのレベルを適用
                        output * modulator_osc.level
                    }
                };

                // 2. Modulatorの出力で Carrier (Osc1) の位相を変調
                let modulation = mod_sample * modulator_osc.modulation_index;
                
                let carrier_sample = {
                    let loop_len = carrier_osc.loop_section.wavetable.len();
                    if loop_len < 2 { 0.0 } else {
                        // 変調を加える (位相変調)
                        let current_fm_phase = carrier_osc.fm_phase + modulation;
                        
                        let current_fm_phase_index = current_fm_phase.rem_euclid(1.0) * loop_len as f32;
                        let output = sample_linear(&carrier_osc.loop_section.wavetable, current_fm_phase_index);
                        
                        output * carrier_osc.level
                    }
                };
                
                // 暫定的なCore/Release中の出力利用:
                let output = if carrier_osc.play_mode == PlayMode::Core || carrier_osc.play_mode == PlayMode::Release {
                    // Core/Release中は、generate_sample のゲイン適用済み出力を使用
                    carrier_osc.generate_sample(is_active, _env_stage) 
                } else {
                    // Loop中はFM合成出力を使用
                    carrier_sample 
                };
                
                output
            }
        }
    }
}