#include "PluginProcessor.h"
#include "PluginEditor.h"

//==============================================================================
MaruMaruAudioProcessorEditor::MaruMaruAudioProcessorEditor(MaruMaruAudioProcessor& p)
    : AudioProcessorEditor(&p), audioProcessor(p)
{
    setSize(400, 300);

    // knobSlider ‚ğİ’è
    addAndMakeVisible(knobSlider);
    knobSlider.setSliderStyle(juce::Slider::Rotary);
    knobSlider.setRange(0.0, 1.0, 0.01);
    knobSlider.setValue(0.5);
    knobSlider.onValueChange = [this]
        {
            // Rust ‚É’l‚ğ“n‚·ˆ—‚ğŒã‚Å’Ç‰Á—\’è
        };
}

MaruMaruAudioProcessorEditor::~MaruMaruAudioProcessorEditor() {}

void MaruMaruAudioProcessorEditor::paint(juce::Graphics& g)
{
    g.fillAll(juce::Colours::black);
    g.setColour(juce::Colours::white);
    g.setFont(15.0f);
}

void MaruMaruAudioProcessorEditor::resized()
{
    knobSlider.setBounds(150, 100, 100, 100);
}
