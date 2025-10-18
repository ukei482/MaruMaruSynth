// lib.rs

// Cargo.toml の [lib] セクションで crate-type = ["cdylib"] を指定すること
// 例:
// [lib]
// crate-type = ["cdylib"]

use lazy_static::lazy_static; 
use std::sync::Mutex;        
use std::fs::{File, OpenOptions}; 
use std::io::Write;      
use std::time::{SystemTime, UNIX_EPOCH}; 
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::boxed::Box;
use std::ops::Rem; 

pub mod analyzer;
pub mod oscillator; 

// ★ 修正点: 必要な型をインポート
use crate::analyzer::types::{AnalysisResult}; 


//==============================================================================
//
//  Logger Implementation
//
//==============================================================================

struct AppLogger {
    file: Option<File>,
}

lazy_static! {
    // Mutexで保護されたグローバルなLoggerインスタンスを作成
    static ref LOGGER: Mutex<AppLogger> = Mutex::new(AppLogger {
        file: OpenOptions::new()
            .create(true)
            .append(true)
            .open("C:/my_programs/MaruMaru/log/log.txt")
            .ok(),
    });
}

// --- Rust内部から呼び出すための関数 ---
fn log_message_internal(prefix: &str, message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        if let Some(file) = &mut logger.file {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();

            // [タイムスタンプ][プレフィックス] メッセージ という形式で書き込む
            let _ = writeln!(file, "[{}][{}] {}", timestamp, prefix, message);
        }
    }
}

// デバッグ用の簡易ファイルロガー
fn log_to_file(msg: &str) {
    if let Ok(mut f) = OpenOptions::new()
        .append(true)
        .create(true)
        .open("C:\\temp\\vst_log.txt")
    {
        let _ = writeln!(f, "{}", msg);
        let _ = f.flush(); // 即座に書き込む
    }
}


//==============================================================================
//
//  FFI Data Structures
//
//==============================================================================

/// Rust と C++ 間で共有するパラメータ構造体
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ParamBundle {
    pub attack    : f32,
    pub decay     : f32,
    pub sustain   : f32,
    pub release   : f32,
    pub blend     : f32,
    pub cutoff    : f32,
    pub resonance : f32,
    // ★ 修正点: OSC制御パラメータを追加
    pub osc1_level: f32,
    pub osc2_level: f32,
    pub osc3_level: f32,
    pub osc1_ratio: f32,
    pub osc2_ratio: f32,
    pub osc3_ratio: f32,
    pub fm_index: f32, // OSC2がOSC1を変調する強度
    pub mix_mode_f: f32, // MixModeをf32で受け取る (0.0=Add, 1.0=FMなど)
}

/// プラグイン内部コンテキスト（C++からは不透明ポインタで扱う）
pub struct Context {
    pub sample_rate : f32,
    pub block_size  : i32,
    pub channels    : i32,
    pub params_ptr  : AtomicPtr<ParamBundle>,
    pub phase       : f32,
    pub phase_inc   : f32,
    pub active      : bool,
    pub amp         : f32,
    pub osc_bank    : Mutex<oscillator::OscillatorBank>, 
}

impl Context {
    /// 初期パラメータをヒープに確保してポインタを返す
    fn default_params_ptr() -> *mut ParamBundle {
        let b = Box::new(ParamBundle {
            attack     : 0.01,
            decay      : 0.1,
            sustain    : 0.8,
            release    : 0.5,
            blend      : 0.5,
            cutoff     : 20000.0,
            resonance  : 1.0,
            // ★ 修正点: OSC制御パラメータの初期化
            osc1_level : 1.0,
            osc2_level : 1.0,
            osc3_level : 0.0,
            osc1_ratio : 1.0,
            osc2_ratio : 1.0,
            osc3_ratio : 2.0,
            fm_index   : 5.0, // 初期FM変調強度
            mix_mode_f : 0.0, // 初期値は加算合成(Add)
        });
        Box::into_raw(b)
    }
}


/// 解析結果の要約（FFIとしてC++に公開するため）
#[repr(C)]
pub struct AnalysisResultFFI {
    // Wave Data Pointers
    pub core_section_ptr    : *mut f32,
    pub core_num_samples    : usize,
    pub loop_section_ptr    : *mut f32,
    pub loop_num_samples    : usize,
    pub release_section_ptr : *mut f32,
    pub release_num_samples : usize,
    
    // Gain Curve Pointers
    pub core_gain_ptr       : *mut f32,
    pub core_gain_len       : usize,
    pub loop_gain_ptr       : *mut f32,
    pub loop_gain_len       : usize,
    pub release_gain_ptr    : *mut f32,
    pub release_gain_len    : usize,
    
    // Other Analysis Data
    pub avg_periodicity     : f32,
    pub quality_score       : f32,
}

impl From<AnalysisResult> for AnalysisResultFFI {
    fn from(analysis: AnalysisResult) -> Self {
        
        // Wave Pointers (E0382 修正: len()を先に計算)
        let core_box = analysis.core_wave.into_boxed_slice();
        let core_len = core_box.len(); 
        let loop_box = analysis.loop_wave.into_boxed_slice();
        let loop_len = loop_box.len(); 
        let release_box = analysis.release_wave.into_boxed_slice();
        let release_len = release_box.len(); 
        
        // Gain Curve Pointers (E0382 修正: len()を先に計算)
        let core_gain_box = analysis.core_gain.into_boxed_slice();
        let core_gain_len = core_gain_box.len(); 
        let loop_gain_box = analysis.loop_gain.into_boxed_slice();
        let loop_gain_len = loop_gain_box.len(); 
        let release_gain_box = analysis.release_gain.into_boxed_slice();
        let release_gain_len = release_gain_box.len(); 

        // F0の平均信頼度を計算
        let avg_periodicity = if !analysis.confidence.is_empty() {
            analysis.confidence.iter().sum::<f32>() / analysis.confidence.len() as f32
        } else {
            0.0
        };

        AnalysisResultFFI {
            // Wave Pointers
            core_section_ptr: Box::into_raw(core_box) as *mut f32,
            core_num_samples: core_len, 
            loop_section_ptr: Box::into_raw(loop_box) as *mut f32,
            loop_num_samples: loop_len, 
            release_section_ptr: Box::into_raw(release_box) as *mut f32,
            release_num_samples: release_len, 
            
            // Gain Pointers
            core_gain_ptr: Box::into_raw(core_gain_box) as *mut f32,
            core_gain_len: core_gain_len, 
            loop_gain_ptr: Box::into_raw(loop_gain_box) as *mut f32,
            loop_gain_len: loop_gain_len, 
            release_gain_ptr: Box::into_raw(release_gain_box) as *mut f32,
            release_gain_len: release_gain_len, 
            
            avg_periodicity: avg_periodicity,
            quality_score: analysis.quality.correlation,
        }
    }
}


//==============================================================================
//
//  FFI Exported Functions
//
//==============================================================================

///-----------------------------------------------------------------------------
/// mm_log_message
/// - C++側からのログ出力を受け付ける
///-----------------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn mm_log_message(message: *const c_char) {
    if message.is_null() { return; }
    let c_str = CStr::from_ptr(message);
    if let Ok(rust_str) = c_str.to_str() {
        // C++からのログには "[JUCE]" というプレフィックスを付ける
        log_message_internal("JUCE", rust_str);
    }
}

///-----------------------------------------------------------------------------
/// mm_create_context
/// - プラグインの内部状態(Context)を初期化してポインタを返す
///-----------------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn mm_create_context(sample_rate: f32, block_size: i32, channels: i32) -> *mut Context {
    let ctx = Box::new(Context {
        sample_rate,
        block_size,
        channels,
        params_ptr : AtomicPtr::new(Context::default_params_ptr()),
        phase      : 0.0,
        phase_inc  : 0.0,
        active     : false, // 初期状態は無音
        amp        : 0.0,
        // ★ 修正点: OscillatorBank の初期化
        osc_bank   : Mutex::new(oscillator::OscillatorBank::new(sample_rate)),
    });
    Box::into_raw(ctx)
}

///-----------------------------------------------------------------------------
/// mm_analyze_buffer
/// - C++(JUCE)またはテストコードから生の音声バッファを受け取り解析する
/// - 戻り値: AnalysisResultFFI のポインタ (解析に失敗した場合は null)
///-----------------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn mm_analyze_buffer(
    _ctx_ptr    : *mut Context, // 解析結果を保存しないので一旦未使用
    buffer      : *const f32,
    num_samples : usize,
    sample_rate : u32,
    core_end_ratio    : f32, 
    release_start_ratio : f32, 
) -> *mut AnalysisResultFFI { // ★ 戻り値を変更
    if buffer.is_null() { 
        log_message_internal("Rust", "mm_analyze_buffer failed: Input buffer is null.");
        return std::ptr::null_mut(); 
    }

    let audio_slice = std::slice::from_raw_parts(buffer, num_samples);

    log_message_internal("Rust", &format!(
        "mm_analyze_buffer called. Samples: {}, Rate: {}, Core End Ratio: {:.2}, Release Start Ratio: {:.2}",
        num_samples, sample_rate, core_end_ratio, release_start_ratio
    ));
    
    match analyzer::analyze_audio(
        audio_slice,
        sample_rate,
        core_end_ratio, 
        release_start_ratio, 
    ) {
        Ok(analysis_data) => {
            log_message_internal("Rust", &format!("Buffer analysis successful. Core len: {}, Loop len: {}, Release len: {}", 
                analysis_data.core_wave.len(),
                analysis_data.loop_wave.len(),
                analysis_data.release_wave.len(),
            ));
            
            // AnalysisResult を FFI 構造体に変換し、ヒープに確保してポインタを返す
            let ffi_result = AnalysisResultFFI::from(analysis_data);
            let boxed_result = Box::new(ffi_result);
            Box::into_raw(boxed_result) // ポインタを返し、C++側にメモリ管理を委譲
        }
        Err(e) => {
            log_message_internal("Rust", &format!("Buffer analysis failed: {}", e));
            std::ptr::null_mut() // 失敗時はnull
        }
    }
}

///-----------------------------------------------------------------------------
/// mm_load_analysis_result (新規追加)
/// - mm_analyze_bufferが返したAnalysisResultFFIの波形データをContextのOSCにロードする
///-----------------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn mm_load_analysis_result(
    ctx_ptr: *mut Context, 
    result_ptr: *const AnalysisResultFFI,
) -> i32 {
    if ctx_ptr.is_null() || result_ptr.is_null() {
        log_message_internal("Rust", "mm_load_analysis_result failed: null pointer.");
        return -1;
    }

    let ctx = &*ctx_ptr;
    let result = &*result_ptr;
    
    if let Ok(mut osc_bank) = ctx.osc_bank.lock() {
        let osc = &mut osc_bank.oscillators[0];
        
        log_message_internal("Rust", &format!(
            "Loaded Gains: Core Len={}, Loop Len={}, Release Len={}",
            result.core_gain_len, result.loop_gain_len, result.release_gain_len
        ));
        
        // Wave data と Gain data の両方を load_data_from_ffi に渡す
        osc.load_data_from_ffi(
            // Wave data
            result.core_section_ptr, result.core_num_samples,
            result.loop_section_ptr, result.loop_num_samples,
            result.release_section_ptr, result.release_num_samples,
            // Gain data
            result.core_gain_ptr, result.core_gain_len,
            result.loop_gain_ptr, result.loop_gain_len,
            result.release_gain_ptr, result.release_gain_len,
        );

        log_message_internal("Rust", "Analysis result successfully loaded (Gains applied).");
        return 0;
    } else {
        log_message_internal("Rust", "mm_load_analysis_result failed: Mutex lock error.");
        return -2;
    }
}

///-----------------------------------------------------------------------------
/// mm_destroy_analysis_result (新規追加)
/// - C++側から呼ばれ、mm_analyze_buffer が返した AnalysisResultFFI を解放する
///-----------------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn mm_destroy_analysis_result(result_ptr: *mut AnalysisResultFFI) {
    if !result_ptr.is_null() {
        let result = &*result_ptr;
        
        let free_f32_slice = |ptr: *mut f32, len: usize| {
            if !ptr.is_null() {
                // ポインタと長さを指定してBoxに戻し、スコープを抜ける際に解放
                let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    ptr, 
                    len
                ) as *mut [f32]);
            }
        };

        // Wave Pointers
        free_f32_slice(result.core_section_ptr, result.core_num_samples);
        free_f32_slice(result.loop_section_ptr, result.loop_num_samples);
        free_f32_slice(result.release_section_ptr, result.release_num_samples);
        
        // ゲインカーブ Pointers を解放
        free_f32_slice(result.core_gain_ptr, result.core_gain_len);
        free_f32_slice(result.loop_gain_ptr, result.loop_gain_len);
        free_f32_slice(result.release_gain_ptr, result.release_gain_len);
        
        // AnalysisResultFFI 自体を解放
        let _ = Box::from_raw(result_ptr);
        log_message_internal("Rust", &format!("AnalysisResultFFI at {:?} destroyed.", result_ptr));
    }
}


///-----------------------------------------------------------------------------
/// mm_destroy_context
/// - C++側から呼ばれ、Contextのメモリを解放する
///-----------------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn mm_destroy_context(ctx_ptr: *mut Context) {
    if !ctx_ptr.is_null() {
        // Box::from_raw を使ってポインタをBoxに戻し、
        // スコープを抜ける際に自動的にメモリを解放させる
        let _ = Box::from_raw(ctx_ptr);
        log_message_internal("Rust", &format!("Context at {:?} destroyed.", ctx_ptr));
    }
}

///-----------------------------------------------------------------------------
/// mm_set_params
/// - C++側から送られてきたパラメータで内部状態をアトミックに更新する
///-----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn mm_set_params(ctx_ptr: *mut Context, params: *const ParamBundle) {
    if ctx_ptr.is_null() || params.is_null() { return; }
    let ctx = unsafe { &*ctx_ptr };
    
    let new_params = unsafe { *params };
    let new_box = Box::new(new_params);
    let new_ptr = Box::into_raw(new_box);
    
    // 1. パラメータポインタを更新
    let old_ptr = ctx.params_ptr.swap(new_ptr, Ordering::SeqCst);
    if !old_ptr.is_null() { 
        unsafe { let _ = Box::from_raw(old_ptr); } 
    }
    
    // 2. ★ 修正点: OscillatorBankにパラメータを適用
    if let Ok(mut osc_bank) = ctx.osc_bank.lock() {
        // MixModeの設定 (簡易的に0.5未満をAdd, 0.5以上をFMとする)
        osc_bank.mix_mode = if new_params.mix_mode_f < 0.5 {
            oscillator::MixMode::Add
        } else {
            oscillator::MixMode::FM
        };
        osc_bank.fm_mix = new_params.blend; // BlendをFMミックスレベルに流用

        // OSCごとのレベルと周波数比を設定
        osc_bank.oscillators[0].level = new_params.osc1_level;
        osc_bank.oscillators[1].level = new_params.osc2_level;
        osc_bank.oscillators[2].level = new_params.osc3_level;
        
        osc_bank.oscillators[0].ratio = new_params.osc1_ratio;
        osc_bank.oscillators[1].ratio = new_params.osc2_ratio;
        osc_bank.oscillators[2].ratio = new_params.osc3_ratio;

        // FM変調強度を設定 (OSC2のみが使用)
        osc_bank.oscillators[1].modulation_index = new_params.fm_index;
    }
}

///-----------------------------------------------------------------------------
/// mm_note_on
/// - MIDIノートオンイベントを処理する
///-----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn mm_note_on(ctx_ptr: *mut Context, note: i32, velocity: i32) {
    if ctx_ptr.is_null() { return; }
    let ctx = unsafe { &mut *ctx_ptr };
    println!("mm_note_on note={} vel={}", note, velocity);  // ←確認
    let freq = 440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0);
    // ctx.phase_incは使用しない（時間軸再生のため）
    ctx.phase = 0.0;
    ctx.active = true;
    ctx.amp = velocity as f32 / 127.0;
    
    // OSC Bank の周波数を設定
    if let Ok(mut osc_bank) = ctx.osc_bank.lock() {
        for osc in osc_bank.oscillators.iter_mut() {
            osc.frequency = freq;
            osc.position = 0.0; // 発音時にポジションをリセット
            osc.play_mode = oscillator::PlayMode::Core; // Coreモードに設定
            osc.fm_phase = 0.0; // FM位相をリセット
        }
    }
}

///-----------------------------------------------------------------------------
/// mm_note_off
/// - MIDIノートオフイベントを処理する
///-----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn mm_note_off(ctx_ptr: *mut Context, _note: i32) {
    if ctx_ptr.is_null() { return; }
    let ctx = unsafe { &mut *ctx_ptr };
    // mm_note_offではactiveをfalseにするだけで、OSCのPlayMode遷移はmm_processで行う
    ctx.active = false;
    // ctx.amp = 0.0; // Release中に音量がゼロにならないように、ここではampをリセットしない
}

///-----------------------------------------------------------------------------
/// mm_process
/// - オーディオバッファを処理し、音声信号を生成する
///-----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn mm_process(ctx_ptr: *mut Context, out_buffer: *mut f32, num_samples: i32, _num_channels: i32) {
    if ctx_ptr.is_null() || out_buffer.is_null() { return; }
    let ctx = unsafe { &mut *ctx_ptr };

    log_to_file(&format!("process: active={} amp={}", ctx.active, ctx.amp));

    // パラメータをアトミックに読み込む
    let params_ptr = ctx.params_ptr.load(Ordering::SeqCst);
    // `unsafe` ブロックは最小限に
    let params_ref = if !params_ptr.is_null() {
        unsafe { *params_ptr }
    } else {
        // ★ 修正点: フォールバック用のデフォルト値に全てのフィールドを追加 (E0063の修正)
        ParamBundle {
            attack: 0.01, decay: 0.1, sustain: 0.8, release: 0.5,
            blend: 0.5, cutoff: 20000.0, resonance: 1.0,
            osc1_level: 1.0, osc2_level: 1.0, osc3_level: 0.0,
            osc1_ratio: 1.0, osc2_ratio: 1.0, osc3_ratio: 2.0,
            fm_index: 5.0, mix_mode_f: 0.0,
        }
    };
    
    let samples = num_samples as usize;
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out_buffer, samples) };

    // バッファをクリア
    for sample in out_slice.iter_mut() { *sample = 0.0; }

    let amp = ctx.amp * params_ref.blend;
    
    if let Ok(mut osc_bank) = ctx.osc_bank.lock() {
        for i in 0..samples {
            // OscillatorBank を使ってサンプルを生成
            let osc_output = osc_bank.process_bank(ctx.active, 0); 
            out_slice[i] = osc_output * amp;
        }
    } else {
         log_message_internal("Rust", "mm_process failed: Mutex lock error for OscillatorBank.");
    }
}