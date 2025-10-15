#pragma once

#include <JuceHeader.h>
#include "PluginProcessor.h"
#include "JuceLogger.h"

//==============================================================================
// MaruMaruAudioProcessorEditor クラス
// - プラグインの GUI を担当する
// - ファイルロードボタン、テキスト表示用エディタを持つ
//==============================================================================
class MaruMaruAudioProcessorEditor : public juce::AudioProcessorEditor
{
public:
    // コンストラクタ
    // - AudioProcessorEditor の初期化
    // - GUI コンポーネントのセットアップ
    MaruMaruAudioProcessorEditor(MaruMaruAudioProcessor&);
    ~MaruMaruAudioProcessorEditor() override;

    // JUCE が呼ぶ描画処理
    void paint(juce::Graphics&) override;

    // JUCE が呼ぶリサイズ処理
    void resized() override;

private:
    // ファイルロードボタン
    juce::TextButton loadFileButton{ "Load File" };

    // ファイル内容を表示するテキストエディタ
    juce::TextEditor myTextEditor;

    // ThreadPool
    // - 重い処理を別スレッドで走らせるために使用
    juce::ThreadPool threadPool{ 2 }; // 最大2スレッド

    // ファイルロードボタンが押されたときに呼ばれる関数
    void onLoadFileClicked();

    // AudioProcessor の参照
    MaruMaruAudioProcessor& audioProcessor;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(MaruMaruAudioProcessorEditor)
};
