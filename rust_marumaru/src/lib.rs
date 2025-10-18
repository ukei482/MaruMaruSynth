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
use std::f32::consts::PI;

pub mod analyzer;


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
        });
        Box::into_raw(b)
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
    });
    Box::into_raw(ctx)
}

///-----------------------------------------------------------------------------
/// mm_analyze_buffer (新規追加)
/// - C++(JUCE)またはテストコードから生の音声バッファを受け取り解析する
///-----------------------------------------------------------------------------
#[no_mangle]
pub unsafe extern "C" fn mm_analyze_buffer(
    _ctx_ptr    : *mut Context, // 解析結果を保存しないので一旦未使用
    buffer      : *const f32,
    num_samples : usize,
    sample_rate : u32,
) -> i32 {
    if buffer.is_null() { return -1; }

    let audio_slice = std::slice::from_raw_parts(buffer, num_samples);

    log_message_internal("Rust", &format!(
        "mm_analyze_buffer called. Samples: {}, Rate: {}",
        num_samples, sample_rate
    ));
    
    // ★ 修正点: analyzer::analyze_audio の呼び出しを新しいシグネチャに合わせる
    match analyzer::analyze_audio(
        audio_slice,
        sample_rate,
    ) {
        Ok(analysis_data) => {
            log_message_internal("Rust", "Buffer analysis successful.");
            log_message_internal("Rust", &format!("Analysis Result: {:?}", analysis_data));
            0 // 成功
        }
        Err(e) => {
            log_message_internal("Rust", &format!("Buffer analysis failed: {}", e));
            -2 // 失敗
        }
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
    let new_box = unsafe { Box::new(*params) };
    let new_ptr = Box::into_raw(new_box);
    // 新しいポインタをセットし、古いポインタを受け取る
    let old_ptr = ctx.params_ptr.swap(new_ptr, Ordering::SeqCst);
    // 古いポインタがnullでなければ、Boxに戻してメモリを解放する
    if !old_ptr.is_null() { 
        unsafe { let _ = Box::from_raw(old_ptr); } 
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
    ctx.phase_inc = (freq / ctx.sample_rate) * 2.0 * PI;
    ctx.phase = 0.0;
    ctx.active = true;
    ctx.amp = velocity as f32 / 127.0;
}

///-----------------------------------------------------------------------------
/// mm_note_off
/// - MIDIノートオフイベントを処理する
///-----------------------------------------------------------------------------
#[no_mangle]
pub extern "C" fn mm_note_off(ctx_ptr: *mut Context, _note: i32) {
    if ctx_ptr.is_null() { return; }
    let ctx = unsafe { &mut *ctx_ptr };
    ctx.active = false;
    ctx.amp = 0.0;
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
        // フォールバック用のデフォルト値
        ParamBundle {
            attack: 0.01, decay: 0.1, sustain: 0.8, release: 0.5,
            blend: 0.5, cutoff: 20000.0, resonance: 1.0,
        }
    };
    
    let samples = num_samples as usize;
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out_buffer, samples) };

    // バッファをクリア
    for sample in out_slice.iter_mut() { *sample = 0.0; }

    if ctx.active {
        let amp = ctx.amp * params_ref.blend; // ADSRも考慮に入れるとさらに良くなります
        for i in 0..samples {
            ctx.phase += ctx.phase_inc;
            if ctx.phase > 2.0 * PI { ctx.phase -= 2.0 * PI; }
            out_slice[i] = ctx.phase.sin() * amp;
        }
    }
}