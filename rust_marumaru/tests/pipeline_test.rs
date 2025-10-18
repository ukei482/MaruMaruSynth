// tests/pipeline_test.rs

// このテストは、ライブラリ全体を外部から使うように動作します。
// そのため、ライブラリ名（Cargo.tomlの[package] name）を使ってモジュールをインポートします。
// ここでは仮に `rust_marumaru` としています。
use rust_marumaru::analyzer;

#[test]
fn test_full_pipeline_on_pure_sine() {
    // 1. Arrange: テスト用の440Hzサイン波を生成
    const SAMPLE_RATE: u32 = 48000;
    const SIGNAL_FREQ: f32 = 440.0;
    const SIGNAL_LEN: usize = SAMPLE_RATE as usize; // 1秒

    let mut signal = Vec::new();
    for i in 0..SIGNAL_LEN {
        let time = i as f32 / SAMPLE_RATE as f32;
        let sample = (2.0 * std::f32::consts::PI * SIGNAL_FREQ * time).sin() * 0.7;
        signal.push(sample);
    }

    // 2. Act: メインの `analyze_audio` 関数を実行！
    let result = analyzer::analyze_audio(&signal, SAMPLE_RATE);
    assert!(result.is_ok(), "解析パイプライン全体がエラーを返しました: {:?}", result.err());
    let analysis_result = result.unwrap();

    // 3. Assert: 最終的な解析結果の各項目を検証
    
    // a) F0カーブが正しく推定されているか
    let valid_f0s: Vec<f32> = analysis_result.f0_curve.iter().filter(|&&f| f > 0.0).cloned().collect();
    assert!(!valid_f0s.is_empty(), "有効なF0が一つも見つかりませんでした");
    let average_f0: f32 = valid_f0s.iter().sum::<f32>() / valid_f0s.len() as f32;
    assert!(
        (average_f0 - SIGNAL_FREQ).abs() < 2.0,
        "F0の平均値が期待値から外れています。期待値: {}, 実際値: {}", SIGNAL_FREQ, average_f0
    );

    // b) モード選択が正しいか (サイン波なのでTimeモードが選択されるはず)
    let average_confidence = analysis_result.confidence.iter().sum::<f32>() / analysis_result.confidence.len() as f32;
    assert!(average_confidence > 0.6, "周期性が低く、Timeモードが選択されなかった可能性があります");

    // c) 生成されたウェーブテーブルが正しいか
    assert_eq!(analysis_result.tables.len(), 1, "ウェーブテーブルは1つだけ生成されるべきです");
    let table = &analysis_result.tables[0];
    let expected_len = (SAMPLE_RATE as f32 / average_f0).round() as usize;
    assert_eq!(table.len(), expected_len, "ウェーブテーブルの長さが正しくありません");
    
    // d) 波形の形がサイン波に近いか
    let peak_idx = (expected_len as f32 * 0.25).round() as usize;
    let trough_idx = (expected_len as f32 * 0.75).round() as usize;
    assert!(
        (table[peak_idx] - 1.0).abs() < 0.1,
        "波形のピークが正しくありません"
    );
    assert!(
        (table[trough_idx] - -1.0).abs() < 0.1,
        "波形のトラフが正しくありません"
    );
}