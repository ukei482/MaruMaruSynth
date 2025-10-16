 // tests/debug_tests.rs

use rust_marumaru::*;
use std::ffi::CString;
use std::path::Path;
use std::ptr;

//==============================================================================
//
// 共通セットアップ
//
//==============================================================================

const SAMPLE_RATE  : f32 = 44100.0;
const BLOCK_SIZE   : i32 = 512;
const NUM_CHANNELS : i32 = 2;

const TEST_AUDIO_FILE_PATH : &str = "C:/my_programs/MaruMaru/assets/piano.wav";
const TEST_MIDI_FILE_PATH  : &str = "C:/my_programs/MaruMaru/assets/spaceii.mid";

/// テストで共通して使用するデータを保持する構造体
struct TestSetup {
    sample_rate     : f32,
    block_size      : i32,
    num_channels    : i32,
    test_audio_path : CString,
}

fn setup() -> TestSetup {
    
    // 存在するテスト用音声ファイルのパス
    let audio_path = Path::new(TEST_AUDIO_FILE_PATH);
    assert!(audio_path.exists(), "Test audio file not found at: {}", TEST_AUDIO_FILE_PATH);

    // 必要に応じてMIDIファイルの読み込みなどもここで行う
    let midi_path = Path::new(TEST_MIDI_FILE_PATH);
    assert!(midi_path.exists(), "Test MIDI file not found at: {}", TEST_MIDI_FILE_PATH);

    TestSetup {
        sample_rate     : SAMPLE_RATE,
        block_size      : BLOCK_SIZE,
        num_channels    : NUM_CHANNELS,
        test_audio_path : CString::new(TEST_AUDIO_FILE_PATH).unwrap(),
    }
}


//==============================================================================
//
// テストケース
//
//==============================================================================

/// Contextの生成と破棄が正常に行えるかテストする
#[test]
fn test_context_creation_and_destruction() {
    println!("--- Running test_context_creation_and_destruction ---");
    let setup = setup(); // セットアップを実行

    // Contextを生成
    let ctx_ptr = mm_create_context(setup.sample_rate, setup.block_size, setup.num_channels);
    assert!(!ctx_ptr.is_null(), "Context creation failed, pointer is null.");
    println!("Context created successfully at {:?}", ctx_ptr);

    // Contextを安全に破棄
    unsafe {
        mm_destroy_context(ctx_ptr);
    }
    println!("Context destruction called.");
}

/// パラメータ設定機能がクラッシュしないかテストする
#[test]
fn test_set_params() {
    println!("--- Running test_set_params ---");
    let setup = setup(); // セットアップを実行

    // Contextを生成
    let ctx_ptr = mm_create_context(setup.sample_rate, setup.block_size, setup.num_channels);
    assert!(!ctx_ptr.is_null(), "Context creation failed.");

    // デフォルトのパラメータを作成
    let params = ParamBundle {
        attack: 0.1, decay: 0.2, sustain: 0.8, release: 0.5,
        blend: 1.0, cutoff: 15000.0, resonance: 2.0,
    };

    // パラメータを設定
    unsafe { mm_set_params(ctx_ptr, &params) };
    println!("Parameters set successfully.");

    // 後片付け
    unsafe {
        mm_destroy_context(ctx_ptr);
    }
}

/// オーディオ処理の呼び出しをテストする
#[test]
fn test_process_block() {
    println!("--- Running test_process_block ---");
    let setup = setup(); // セットアップを実行

    let ctx_ptr = mm_create_context(setup.sample_rate, setup.block_size, setup.num_channels);
    assert!(!ctx_ptr.is_null());

    // Note Onをシミュレート
    let note = 69; // A4
    let velocity = 100;
    unsafe { mm_note_on(ctx_ptr, note, velocity) };
    println!("NoteOn event sent.");

    // 出力用のバッファを確保
    let mut buffer: Vec<f32> = vec![0.0; setup.block_size as usize];

    // processを呼び出す
    unsafe { mm_process(ctx_ptr, buffer.as_mut_ptr(), setup.block_size, setup.num_channels) };
    println!("Process block executed.");
    
    // バッファに何らかの値が書き込まれているか確認
    let is_silent = buffer.iter().all(|&sample| sample == 0.0);
    assert!(!is_silent, "Buffer should not be silent after a note on.");
    println!("Buffer contains audio data.");

    // Note Offをシミュレート
    unsafe { mm_note_off(ctx_ptr, note) };
    println!("NoteOff event sent.");

    // 後片付け
    unsafe {
        mm_destroy_context(ctx_ptr);
    }
}
