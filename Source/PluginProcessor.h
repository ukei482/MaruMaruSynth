#pragma once

#include <JuceHeader.h>
#include <windows.h>

// Rust DLL ä÷êîå^
using RustProcessFunc = void(*)(float*, size_t, const MidiEvent*, size_t, float, float, float);

HINSTANCE       rustDll      = LoadLibrary("rust_dsp.dll");
RustProcessFunc rust_process = (RustProcessFunc)GetProcAddress(rustDll, "rust_process");

class MaruMaruAudioProcessor : public juce::AudioProcessor
{
public:
    MaruMaruAudioProcessor();
    ~MaruMaruAudioProcessor() override;

    // ïKê{ÉÅÉ\ÉbÉh
    void prepareToPlay(double sampleRate, int samplesPerBlock) override;
    void releaseResources() override;
    void processBlock(juce::AudioBuffer<float>&, juce::MidiBuffer&) override;

    juce::AudioProcessorEditor* createEditor() override;
    bool hasEditor() const override { return true; }

    const juce::String getName() const override { return "MaruMaru"; }
    bool acceptsMidi() const override { return true; }
    bool producesMidi() const override { return false; }
    bool isMidiEffect() const override { return false; }
    double getTailLengthSeconds() const override { return 0.0; }

    int getNumPrograms() override { return 1; }
    int getCurrentProgram() override { return 0; }
    void setCurrentProgram(int) override {}
    const juce::String getProgramName(int) override { return {}; }
    void changeProgramName(int, const juce::String&) override {}

    void getStateInformation(juce::MemoryBlock&) override {}
    void setStateInformation(const void*, int) override {}

private:
    HMODULE rustDllHandle = nullptr;
    ProcessAudioFn rustProcess = nullptr;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(MaruMaruAudioProcessor)
};
