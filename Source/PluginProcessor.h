//PluginProcessor.h

#pragma once

// JUCE の基本ヘッダ
#include <JuceHeader.h>

// Rust ブリッジの宣言をインクルード
#include "RustBridge.h"

/*
  MaruMaruAudioProcessor
  - JUCE 側の AudioProcessor（ホストとのインターフェース）
  - GUI は Editor が担当、音声処理/パラメータ転送はここが仲介して Rust に委譲する
*/

class MaruMaruAudioProcessor : public juce::AudioProcessor
{
public:

    MaruMaruAudioProcessor();
    ~MaruMaruAudioProcessor() override;




    // ===========================================================
    // JUCE AudioProcessor の純粋仮想関数を実装
    // ===========================================================
    bool acceptsMidi() const override { return false; } // MIDI非対応なら false
    bool producesMidi() const override { return false; }
    int getNumPrograms() override { return 1; }
    int getCurrentProgram() override { return 0; }
    void setCurrentProgram(int) override {}
    const juce::String getProgramName(int) override { return {}; }
    void changeProgramName(int, const juce::String&) override {}
    void getStateInformation(juce::MemoryBlock& destData) override {}
    void setStateInformation(const void* data, int sizeInBytes) override {}

    // AudioProcessor の必須オーバーライド
    void prepareToPlay(double sampleRate, int samplesPerBlock) override;
    void releaseResources() override {}
    bool isBusesLayoutSupported(const BusesLayout& layouts) const override;
    void processBlock(juce::AudioBuffer<float>&, juce::MidiBuffer&) override;

    // Editor による GUI を持つ
    juce::AudioProcessorEditor* createEditor() override;
    bool hasEditor() const override { return true; }

    // プラグインの基本情報
    const juce::String getName() const override { return "MaruMaruSynth"; }
    double getTailLengthSeconds() const override { return 0.0; }

    // パラメータ管理（AudioProcessorValueTreeState）
    static juce::AudioProcessorValueTreeState::ParameterLayout createParameters();
    juce::AudioProcessorValueTreeState parameters;

    // Rust ブリッジ（DLL のハンドリング、FFI 呼び出しをラップ）
    RustBridge bridge;

private:
    // APVTS の現在値を収集して一括で Rust に送る
    void sendParamsToRust();

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(MaruMaruAudioProcessor)
};
