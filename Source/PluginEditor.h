#pragma once

#include <JuceHeader.h>
#include "PluginProcessor.h"

class MaruMaruAudioProcessorEditor : public juce::AudioProcessorEditor
{
public:
    MaruMaruAudioProcessorEditor(MaruMaruAudioProcessor&);
    ~MaruMaruAudioProcessorEditor() override;

    void paint(juce::Graphics&) override;
    void resized() override;

private:
    MaruMaruAudioProcessor& audioProcessor; // プロセッサ参照
    juce::Slider knobSlider;                // GUI用スライダー

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(MaruMaruAudioProcessorEditor)
};
