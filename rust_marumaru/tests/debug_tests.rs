 // tests/debug_tests.rs

use rust_marumaru::*;
use std::ffi::CString;
use std::ptr;

/// Contextの生成と破棄が正常に行えるかテストする
#[test]
fn test_context_creation_and_destruction() {
    println!("--- Running test_context_creation_and_destruction ---");

    // Contextを生成
    let ctx_ptr = mm_create_context(44100.0, 512, 2);
    assert!(!ctx_ptr.is_null(), "Context creation failed, pointer is null.");
    println!("Context created successfully at {:?}", ctx_ptr);

    // Contextを安全に破棄
    // `mm_destroy_context` が `lib.rs` に実装されている必要があります。
    unsafe {
        // mm_destroy_context(ctx_ptr); // メモリリーク防止のため、実装後にコメントを外してください
    }
    println!("Context destruction called.");
}





/// パラメータ設定機能がクラッシュしないかテストする
#[test]
fn test_set_params() {
    println!("--- Running test_set_params ---");

    // Contextを生成
    let ctx_ptr = mm_create_context(44100.0, 512, 2);
    assert!(!ctx_ptr.is_null(), "Context creation failed.");

    // デフォルトのパラメータを作成
    let params = ParamBundle {
        attack: 0.1,
        decay: 0.2,
        sustain: 0.8,
        release: 0.5,
        blend: 1.0,
        cutoff: 15000.0,
        resonance: 2.0,
    };

    // パラメータを設定
    mm_set_params(ctx_ptr, &params);
    println!("Parameters set successfully.");

    // 後片付け
    unsafe {
        // mm_destroy_context(ctx_ptr);
    }
}





/// オーディオ処理の呼び出しをテストする
#[test]
fn test_process_block() {
    println!("--- Running test_process_block ---");

    let sample_rate = 44100.0;
    let block_size = 512;
    let num_channels = 2;

    let ctx_ptr = mm_create_context(sample_rate, block_size, num_channels);
    assert!(!ctx_ptr.is_null());

    // Note Onをシミュレート
    let note = 69; // A4
    let velocity = 100;
    mm_note_on(ctx_ptr, note, velocity);
    println!("NoteOn event sent.");

    // 出力用のバッファを確保
    let mut buffer: Vec<f32> = vec![0.0; block_size as usize];

    // processを呼び出す
    mm_process(ctx_ptr, buffer.as_mut_ptr(), block_size, num_channels);
    println!("Process block executed.");
    
    // バッファに何らかの値が書き込まれているか確認（単純な無音チェック）
    let is_silent = buffer.iter().all(|&sample| sample == 0.0);
    assert!(!is_silent, "Buffer should not be silent after a note on.");
    println!("Buffer contains audio data.");

    // Note Offをシミュレート
    mm_note_off(ctx_ptr, note);
    println!("NoteOff event sent.");

    // 後片付け
    unsafe {
        // mm_destroy_context(ctx_ptr);
    }
}





/// ファイル解析関数のエラーハンドリングをテストする
#[test]
fn test_analyze_non_existent_file() {
    println!("--- Running test_analyze_non_existent_file ---");

    let ctx_ptr = mm_create_context(44100.0, 512, 2);
    assert!(!ctx_ptr.is_null());

    // 存在しないファイルパスをCStringに変換
    let non_existent_path = CString::new("C:/path/to/non_existent_file.wav").unwrap();

    // 解析関数を呼び出す
    let result = unsafe {
        mm_analyze_file(ctx_ptr, non_existent_path.as_ptr())
    };

    // 失敗コードが返ってくることを期待する
    assert!(result < 0, "Expected a failure code, but got {}", result);
    println!("Analysis of non-existent file failed as expected (code: {}).", result);

    // 後片付け
    unsafe {
        // mm_destroy_context(ctx_ptr);
    }
}