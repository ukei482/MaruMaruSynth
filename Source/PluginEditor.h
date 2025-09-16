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
    MaruMaruAudioProcessor& audioProcessor; // �v���Z�b�T�Q��
    juce::Slider knobSlider;                // GUI�p�X���C�_�[

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(MaruMaruAudioProcessorEditor)
};
