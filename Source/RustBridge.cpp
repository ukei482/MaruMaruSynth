//RustBridge.cpp

#include "RustBridge.h"
#include <iostream>
#include <cstring>

/*
  RustBridge 実装（Windows 用）
  - LoadLibrary / GetProcAddress でシンボルを解決する
  - ctx（void*）は Rust が返してくる不透明ポインタ。呼び出し先は常に ctx を最初の引数に取る。
*/

RustBridge::RustBridge() 
    : dll(nullptr), ctx(nullptr),
    fn_create_ctx(nullptr), 
    fn_destroy_ctx(nullptr),
    fn_analyze_file(nullptr),
    fn_set_params(nullptr),
    fn_note_on(nullptr), 
    fn_note_off(nullptr),
    fn_process(nullptr) ,
    fn_log_message(nullptr)
{
}

RustBridge::~RustBridge() { unload(); }

/*
  DLL をロードし、必要なシンボルを取得する。
  戻り値は全シンボルが揃ったかどうか。
*/
bool RustBridge::load(const char* dllPath) //PluginProcessor.cppで呼ばれている
{
    dll = LoadLibraryA(dllPath);
    if (!dll) {
        std::cerr << "RustBridge: DLL load failed: DLLの読み込みに失敗しました " << dllPath << std::endl;
        return false;
    }

    // シンボル名は Rust 側で pub extern "C" として公開した名前と一致させる
    fn_create_ctx    = (FnCreateCtx)GetProcAddress   (dll, "mm_create_context");
    fn_destroy_ctx   = (FnDestroyCtx)GetProcAddress  (dll, "mm_destroy_context");
    fn_analyze_file  = (FnAnalyzeFile)GetProcAddress (dll, "mm_analyze_file");
    fn_set_params    = (FnSetParams)GetProcAddress   (dll, "mm_set_params");
    fn_note_on       = (FnNoteOn)GetProcAddress      (dll, "mm_note_on");
    fn_note_off      = (FnNoteOff)GetProcAddress     (dll, "mm_note_off");
    fn_process       = (FnProcess)GetProcAddress     (dll, "mm_process");
    fn_log_message   = (FnLogMessage)GetProcAddress  (dll, "mm_log_message");

    bool ok = fn_create_ctx && fn_destroy_ctx && fn_analyze_file && fn_set_params && fn_note_on && fn_note_off && fn_process && fn_process && fn_log_message;
    if (!ok) {
        std::cerr << "RustBridge: failed to get all required symbols.　DLL内に必要な関数が存在しません" << std::endl;
    }
    return ok;
}

/*
  DLL とコンテキストを安全に破棄する
*/
void RustBridge::unload()  //PluginProcessor.cppで呼ばれている
{
    if (ctx && fn_destroy_ctx) {
        fn_destroy_ctx(ctx);
        ctx = nullptr;
    }

    if (dll) {
        FreeLibrary(dll);
        dll = nullptr;
    }
}




/*
  Rust にコンテキストを作らせ、内部に保持するポインタを得る
  - 戻り値 true: ctx が得られた
*/
bool RustBridge::create_context(float sample_rate, int block_size, int num_channels) //PrepareToPlayで呼ばれている
{
    if (!fn_create_ctx) return false;
	ctx = fn_create_ctx(sample_rate, block_size, num_channels); //fn_create_ctxにはmm_create_contextのアドレスが入っている。mm_create_contextはRust側で定義されている関数
    return ctx != nullptr;
}




void RustBridge::destroy_context()
{
    if (ctx && fn_destroy_ctx) {
        fn_destroy_ctx(ctx);
        ctx = nullptr;
    }
}

/*
  ファイル解析（blocking）
  - 呼び出し元でバックグラウンドスレッドを使うこと
*/
int RustBridge::analyzeFile(const char* path)
{
    if (!fn_analyze_file || !ctx) return -1;
    return fn_analyze_file(ctx, path);
}

/*
  パラメータの一括設定
  - ParamBundle をそのまま Rust に渡す（Rust 側でコピーされる実装）
*/
void RustBridge::set_params(const ParamBundle& p)
{
    if (!fn_set_params || !ctx) return;
    fn_set_params(ctx, &p);
}

/*
  MIDI note on / off
*/
void RustBridge::note_on(int note, int velocity)
{
    if (!fn_note_on || !ctx) return;
    fn_note_on(ctx, note, velocity);
}

void RustBridge::note_off(int note)
{
    if (!fn_note_off || !ctx) return;
    fn_note_off(ctx, note);
}

/*
  オーディオ処理呼び出し
  - out_buffer_channel0: チャンネル0 の生ポインタ（num_samples 個分）
  - Rust はこの領域に num_samples 個のサンプルを書き込む前提
  - プロダクトでマルチチャンネルを正確に扱うなら、float** の方式や
    事前にインターリーブバッファを用意する方式に変えること
*/
void RustBridge::process(float* out_buffer_channel0, int num_samples, int num_channels)
{
    if (!fn_process || !ctx) return;
    fn_process(ctx, out_buffer_channel0, num_samples, num_channels);
}

void RustBridge::log(const std::string& message)
{
    if (fn_log_message) {
        fn_log_message(message.c_str());
    }
}