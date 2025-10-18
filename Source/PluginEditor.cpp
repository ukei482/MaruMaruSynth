#include "PluginEditor.h"
#include "JuceLogger.h"
#include <juce_core/juce_core.h>

//==============================================================================
// PluginEditor.cpp
//==============================================================================

/*
  onLoadFileClicked():
  - ユーザーがファイルを選択したときに呼ばれるコールバック
  - 非同期でファイル処理を行い、メインスレッドをブロックしないようにする
*/

// ThreadPool に渡すためのラッパークラス
// - JUCE8 では ThreadPool::addJob に std::function を直接渡せない
// - そのため、ThreadPoolJob を継承したクラスを作って対応する
class LambdaThreadPoolJob : public juce::ThreadPoolJob
{
public:
    // コンストラクタで処理内容を受け取る
    LambdaThreadPoolJob(std::function<void()> f)
        : juce::ThreadPoolJob("LambdaJob"), func(std::move(f)) {}

    // ThreadPool から呼ばれる処理本体
    JobStatus runJob() override
    {
        func(); // 受け取ったラムダを実行
        return jobHasFinished; // ジョブ完了を通知
    }

private:
    std::function<void()> func; // 実行する処理
};

//==============================================================================
// コンストラクタ：
// - コンポーネントの初期化とレイアウト登録
//==============================================================================
MaruMaruAudioProcessorEditor::MaruMaruAudioProcessorEditor(MaruMaruAudioProcessor& p)
    : AudioProcessorEditor(&p), audioProcessor(p)
{
    Logger::log("Editor created.");

    // ボタンのクリック時に onLoadFileClicked() を呼ぶように設定
    loadFileButton.onClick = [this]() { onLoadFileClicked(); };

    // コンポーネントをエディタに追加
    addAndMakeVisible(loadFileButton);
    addAndMakeVisible(myTextEditor);

    // ウィンドウサイズを設定
    setSize(400, 300);
}

//==============================================================================
// デストラクタ
//==============================================================================
MaruMaruAudioProcessorEditor::~MaruMaruAudioProcessorEditor()
{
}

//==============================================================================
// 背景描画
//==============================================================================
void MaruMaruAudioProcessorEditor::paint(juce::Graphics& g)
{
    // 背景を塗りつぶす
    g.fillAll(getLookAndFeel().findColour(juce::ResizableWindow::backgroundColourId));
}

//==============================================================================
// レイアウト調整
//==============================================================================
void MaruMaruAudioProcessorEditor::resized()
{
    // ボタンとテキストエディタの位置・サイズを指定
    loadFileButton.setBounds(10, 10, 100, 30);
    myTextEditor.setBounds(10, 50, getWidth() - 20, getHeight() - 60);
}

//==============================================================================
// ファイルロード処理
//==============================================================================
void MaruMaruAudioProcessorEditor::onLoadFileClicked()
{
    // ユーザーにファイルを選択させるダイアログを表示（非同期版）
    juce::FileChooser chooser("Select a file to open...", {}, "*.*");

    chooser.launchAsync(juce::FileBrowserComponent::openMode | juce::FileBrowserComponent::canSelectFiles,
        [this](const juce::FileChooser& fc)
        {
            auto file = fc.getResult();
            if (file.existsAsFile())
            {
                Logger::log("File selected: " + file.getFullPathName().toStdString());

                // バックグラウンド処理を ThreadPool に投げる
                auto job = std::make_unique<LambdaThreadPoolJob>([this, file]()
                    {
                        // バックグラウンド処理（例：ファイル読み込み）
                        juce::String fileContent = file.loadFileAsString();

                        // メインスレッドに戻って UI を更新する
                        juce::MessageManager::callAsync([this, fileContent]()
                            {
                                myTextEditor.setText(fileContent);
                            });
                    });

                // ThreadPool にジョブを追加
                threadPool.addJob(job.release(), true);
            }
        });
}
