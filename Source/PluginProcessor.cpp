#include "PluginProcessor.h"
#include "PluginEditor.h"
#include <vector>

//==============================================================================
MaruMaruAudioProcessor::MaruMaruAudioProcessor()
    : AudioProcessor(BusesProperties()
        .withInput ("Input", juce::AudioChannelSet::stereo(), true)
        .withOutput("Output", juce::AudioChannelSet::stereo(), true))
{
    rustDllHandle = LoadLibraryA("rust_dsp.dll");
    if (rustDllHandle)
        rustProcess = (ProcessAudioFn)GetProcAddress(rustDllHandle, "process_audio");
}

MaruMaruAudioProcessor::~MaruMaruAudioProcessor()
{
    if (rustDllHandle)
        FreeLibrary(rustDllHandle);
}

void MaruMaruAudioProcessor::prepareToPlay(double, int) {}
void MaruMaruAudioProcessor::releaseResources() {}




//==============================================================================


//以下にオーディオ処理を記述


struct MidiEvent {
    int noteNumber;
    float velocity;
    int samplePosition;
};


void MaruMaruAudioProcessor::processBlock(juce::AudioBuffer<float>& buffer, juce::MidiBuffer& midiMessages)
{
    juce::ignoreUnused(midiMessages);

    if (rustProcess)
    {
        std::vector<MidiEvent> midiVec;

        for (const auto metadata : midiMessages) {
            const auto& msg = metadata.getMessage();
            MidiEvent e;
            e.noteNumber = msg.getNoteNumber();
            e.velocity = msg.getVelocity();
            e.samplePosition = metadata.samplePosition;
            midiVec.push_back(e);
        }

        // 仮パラメータ
        float blend = 0.5f;
        float lfoRate = 2.0f;
        float lfoDepth = 0.1f;

        rust_process(buffer.getWritePointer(0),
            buffer.getNumSamples(),
            midiVec.data(),
            midiVec.size(),
            blend,
            lfoRate,
            lfoDepth);
    }
    else
    {
        buffer.clear();
    }
}

juce::AudioProcessorEditor* MaruMaruAudioProcessor::createEditor()
{
    return new MaruMaruAudioProcessorEditor(*this);
}

















//==============================================================================
juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter()
{
    return new MaruMaruAudioProcessor();
}
