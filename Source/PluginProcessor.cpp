//PluginProcessor.cpp

#include "PluginProcessor.h"
#include "PluginEditor.h"

/*
  パラメータの APVTS キー文字列はここで定義。
  文字列名は APVTS と Editor で一致させる必要がある。
*/

static const char* P_ATTACK    = "attack";
static const char* P_DECAY     = "decay";
static const char* P_SUSTAIN   = "sustain";
static const char* P_RELEASE   = "release";
static const char* P_BLEND     = "blend";
static const char* P_CUTOFF    = "cutoff";
static const char* P_RESONANCE = "resonance";
static const char* P_DLL_LOADED = "dll_loaded";  // 新規追加

MaruMaruAudioProcessor::MaruMaruAudioProcessor()
    : AudioProcessor(BusesProperties().withOutput("Output", juce::AudioChannelSet::stereo(), true)),
    parameters(*this, nullptr, "PARAMS", createParameters())
{
    // DLL ロードはコンストラクタではなく必要なタイミングで行っても良いが
    // ここでは簡潔のためロードしておく（DLLの配置場所に注意）
    bridge.load("C:/my_programs/MaruMaru/rust_marumaru/target/release/rust_marumaru.dll");

    // Loggerを初期化
    Logger::init(&bridge);
    Logger::log("PluginProcessor created. Logger initialized.");

}

MaruMaruAudioProcessor::~MaruMaruAudioProcessor()
{
    Logger::log("PluginProcessor destroyed. Shutting down logger.");
    // Loggerをシャットダウン
    Logger::shutdown();

    // アンロードとコンテキスト破棄を確実に行う
    bridge.unload();
}

/*
  プラグインで使用するパラメータを定義する関数。
  ここで定義したパラメータが DAW に表示され、オートメーション可能になる。
*/
juce::AudioProcessorValueTreeState::ParameterLayout MaruMaruAudioProcessor::createParameters()
{
    std::vector<std::unique_ptr<juce::RangedAudioParameter>> params;

    // ADSR
    params.push_back(std::make_unique<juce::AudioParameterFloat>(P_ATTACK , "Attack" , 0.001f, 5.0f , 0.01f));
    params.push_back(std::make_unique<juce::AudioParameterFloat>(P_DECAY  , "Decay"  , 0.001f, 5.0f , 0.1f ));
    params.push_back(std::make_unique<juce::AudioParameterFloat>(P_SUSTAIN, "Sustain", 0.0f  , 1.0f , 0.8f ));
    params.push_back(std::make_unique<juce::AudioParameterFloat>(P_RELEASE, "Release", 0.001f, 10.0f, 0.5f ));

    // 合成パラメータ
    params.push_back(std::make_unique<juce::AudioParameterFloat>(P_BLEND    , "Blend"    , 0.0f , 1.0f    , 0.5f    ));
    params.push_back(std::make_unique<juce::AudioParameterFloat>(P_CUTOFF   , "Cutoff"   , 20.0f, 20000.0f, 20000.0f));
    params.push_back(std::make_unique<juce::AudioParameterFloat>(P_RESONANCE, "Resonance", 0.1f , 20.0f   , 1.0f    ));

    params.push_back(std::make_unique<juce::AudioParameterFloat>(P_DLL_LOADED, "DLL Loaded", 0.0f, 1.0f, 0.0f));

    return { params.begin(), params.end() };
}









/*
  prepareToPlay:
  - サンプルレートやブロックサイズがここで与えられる（ホストから）
  - Rust 側にコンテキストを作らせ、サンプルレート等を渡す
*/

void MaruMaruAudioProcessor::prepareToPlay(double sampleRate, int samplesPerBlock)
{
    // Rust 側にコンテキストを作成（内部で AudioThread 安全な構成にする）
    bridge.create_context((float)sampleRate, samplesPerBlock, (int)getTotalNumOutputChannels());

    bool ok = bridge.create_context((float)sampleRate, samplesPerBlock, getTotalNumOutputChannels());
    if (ok)
        parameters.getParameter(P_DLL_LOADED)->setValueNotifyingHost(1.0f);
    else
        parameters.getParameter(P_DLL_LOADED)->setValueNotifyingHost(0.0f);

}











/*
  パラメータを 1 ブロックごとにまとめて Rust に転送する。
  - 毎サンプルごとに送るのはオーバーヘッドになるのでブロック単位で更新する設計
  - APVTS の raw pointer 経由で現在値を取得する（スレッド安全ではない呼び出しはここで）
*/

void MaruMaruAudioProcessor::sendParamsToRust()
{
    // Rust 側とレイアウトを合わせた構造体 ParamBundle（RustBridge.h）を使用
    ParamBundle p;
    p.attack    = parameters.getRawParameterValue(P_ATTACK)   ->load();
    p.decay     = parameters.getRawParameterValue(P_DECAY)    ->load();
    p.sustain   = parameters.getRawParameterValue(P_SUSTAIN)  ->load();
    p.release   = parameters.getRawParameterValue(P_RELEASE)  ->load();
    p.blend     = parameters.getRawParameterValue(P_BLEND)    ->load();
    p.cutoff    = parameters.getRawParameterValue(P_CUTOFF)   ->load();
    p.resonance = parameters.getRawParameterValue(P_RESONANCE)->load();

    // Rust に一括転送（内部で atomics を使って安全に差し替える）
    bridge.set_params(p);
}












/*
  processBlock:
  - MIDI を受け取り、note on/off を Rust に通知
  - バッファ（チャンネル0 のポインタ）を Rust に渡してサンプルを書き出してもらい、
    必要なら他チャンネルにコピーする（この例ではステレオ対応を簡易実装）
*/

void MaruMaruAudioProcessor::processBlock(juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages)
{
    buffer.clear();
    juce::ScopedNoDenormals noDenormals;

    // ブロック開始時にパラメータを更新して渡す（1 回で十分）
    sendParamsToRust();

    // MIDI の簡易転送（NoteOn / NoteOff のみ）
    for (const auto& metadata : midiMessages)
    {
        const auto m = metadata.getMessage();
        if (m.isNoteOn()) 
        {
            bridge.note_on(m.getNoteNumber(), m.getVelocity());
            DBG("NoteOn: " << m.getNoteNumber() << " vel=" << m.getVelocity());
        }
        else if (m.isNoteOff())
            bridge.note_off(m.getNoteNumber());
        // pitch bend / modwheel は必要に応じて追加
    }

    // 出力バッファの準備
    int numChannels = buffer.getNumChannels();
    int numSamples  = buffer.getNumSamples();

    // ここでは「チャンネル0 のポインタ」を Rust に渡し、Rust はその領域に numSamples 個分の値を書き込む設計
    // （Rust は必ずチャンネル0分のみ書き、こちらで他チャンネルへコピーする）
    float* out0     = buffer.getWritePointer(0);

    bridge.process(out0, numSamples, numChannels);

    // 他チャンネルへコピー（簡易ステレオ対応）
    if (numChannels > 1)
    {
        float* out1 = buffer.getWritePointer(1);
        // 単純にチャンネル0の内容をコピーして左右同じ音にする
        std::memcpy(out1, out0, sizeof(float) * (size_t)numSamples);

        // 多チャンネル対応が必要ならここを拡張する
    }
}







// Editor の生成（Editor クラスは PluginEditor.cpp で定義）
juce::AudioProcessorEditor* MaruMaruAudioProcessor::createEditor()
{
    return new MaruMaruAudioProcessorEditor(*this);
}



//==============================================================================
// createPluginFilter()
//   - VST/AU/Standalone のエントリポイント
//   - 自作の AudioProcessor を返す必要がある
//==============================================================================
juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new MaruMaruAudioProcessor();
}

bool MaruMaruAudioProcessor::isBusesLayoutSupported(const BusesLayout& layouts) const
{
    // 入力がステレオの場合のみ OK
    if (layouts.getMainInputChannelSet() != juce::AudioChannelSet::stereo()
        || layouts.getMainOutputChannelSet() != juce::AudioChannelSet::stereo())
        return false;

    return true;
}