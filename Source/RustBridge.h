//RustBridge.h

#pragma once

// Windows の DLL API を使うために windows.h をインクルード
#include <windows.h>
#include <string>

/*
  ParamBundle のレイアウトは Rust 側の #[repr(C)] ParamBundle と **完全に一致** させる必要があります。
  順序・型（ここでは float）が一致していないと未定義動作になります。
*/
struct ParamBundle {
    float attack;
    float decay;
    float sustain;
    float release;
    float blend;
    float cutoff;
    float resonance;
};

/*
  RustBridge クラス
  - DLL のロード／関数シンボルの解決
  - Rust 側 ctx（不透明ポインタ）を保持
  - JUCE 側からはこのクラスを通じて Rust を呼び出す
*/
class RustBridge {
public:
    RustBridge();
    ~RustBridge();

    // DLL のロード / アンロード
    bool load(const char* dllPath);
    void unload();

    // コンテキスト作成 / 破棄（Rust 側にインスタンスを作成させる）
    bool create_context(float sample_rate, int block_size, int num_channels);
    void destroy_context();

    // 解析（blocking：呼び出し元でバックグラウンドスレッドで使うこと）
    int analyzeFile(const char* path);

    // パラメータ一括設定（ParamBundle を構造体で渡す）
    void set_params(const ParamBundle& p);

    // MIDI note on/off シンプルインターフェース
    void note_on(int note, int velocity);
    void note_off(int note);

    // オーディオ処理：チャンネル0 のポインタに num_samples 分の PCM を書き込む想定
    // （呼び出し側で他チャンネルへコピーする実装にしている）
    void process(float* out_buffer_channel0, int num_samples, int num_channels);

    void log(const std::string& message);

private:
    HMODULE dll;   // DLL ハンドル
    void* ctx;     // Rust 側で作られたコンテキスト（不透明ポインタ）

    // FFI 関数ポインタ型（Rust 側のシンボル名に合わせる）
    typedef void*   (*FnCreateCtx)(float, int, int);
    typedef void    (*FnDestroyCtx)(void*);
    typedef int     (*FnAnalyzeFile)(void*, const char*);
    typedef void    (*FnSetParams)(void*, const ParamBundle*);
    typedef void    (*FnNoteOn)(void*, int, int);
    typedef void    (*FnNoteOff)(void*, int);
    typedef void    (*FnProcess)(void*, float*, int, int);
    typedef void    (*FnLogMessage)(const char*);

    // 実際の関数ポインタ
    FnCreateCtx    fn_create_ctx;
    FnDestroyCtx   fn_destroy_ctx;
    FnAnalyzeFile  fn_analyze_file;
    FnSetParams    fn_set_params;
    FnNoteOn       fn_note_on;
    FnNoteOff      fn_note_off;
    FnProcess      fn_process;
    FnLogMessage   fn_log_message;
    
};
