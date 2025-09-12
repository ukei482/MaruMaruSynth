use nih_plug::prelude::*;
use std::sync::Arc;

use crate::params::WtParams;
use crate::wavetable::Wavetable;
use crate::voice::Voice;

pub struct WavetablePlugin {
    params       : Arc<WtParams>,
    sample_rate  : f32,
    wavetable    : Wavetable,
    voices       : Vec<Voice>,
}

impl Default for WavetablePlugin {
    fn default() -> Self {
        let table_size = 2048usize;
        let mut wt = Wavetable::new(table_size);


        let sine: Vec<f32> = (0..table_size).map(|i| {
            let t = (i as f32) / (table_size as f32);
            (2.0 * std::f32::consts::PI * t).sin()
        }).collect();

        wt.add_table(sine);

        let saw: Vec<f32> = (0..table_size).map(|i| {
            let t = (i as f32) / (table_size as f32);
            2.0 * (t - 0.5)
        }).collect();
        wt.add_table(saw);

        let square: Vec<f32> = (0..table_size).map(|i| {
            let t = (i as f32) / (table_size as f32);
            if t < 0.5 { 1.0 } else { -1.0 }
        }).collect();
        wt.add_table(square);

        let params = Arc::new(WtParams::default());

        Self {
            params,
            sample_rate: 44100.0,
            wavetable: wt,
            voices: (0..16).map(|_| Voice::new()).collect(),
        }
    }
}

impl Vst3Plugin for WavetablePlugin {

    const VST3_CLASS_ID: [u8; 16]                        = *b"MaruMaruSynth!!!";       // VST識別子　  16 byte固定
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Synth];  //カテゴリ設定　 シンセサイザー

}

impl Plugin for WavetablePlugin {

    type SysExMessage = ();
    type BackgroundTask = ();

    const NAME    : &'static str = "MaruMaruSynthesizer";
    const VENDOR  : &'static str = "UKEI YAMADA";
    const URL     : &'static str = "https://example.org";
    const EMAIL   : &'static str = "";
    const VERSION : &'static str = "0.1.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout::const_default(),
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn process(
        &mut self,


        buffer  : &mut Buffer<'_>,                   //
        _aux    : &mut AuxiliaryBuffers<'_>,         //
        context : &mut impl ProcessContext<Self>,    //



    ) -> ProcessStatus {

        let table_idx_i = self.params.wavetable.unmodulated_plain_value();
        let morph       = self.params.wavetable_pos.unmodulated_plain_value();
        let gain        = self.params.gain.unmodulated_plain_value();

        let mut next_event = context.next_event();

        for (_sample_idx, channel_samples) in buffer.iter_samples().enumerate() {

            while let Some(event) = next_event {
                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
                            v.note_on(note, velocity, self.sample_rate);
                        } else {
                            self.voices[0].note_on(note, velocity, self.sample_rate);
                        }
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        for v in self.voices.iter_mut() {
                            if v.active && v.note == note { v.note_off(); }
                        }
                    }
                    _ => {}
                }
                next_event = context.next_event();
            }

            let table_a = (table_idx_i as usize) % self.wavetable.tables.len();
            let table_b = (table_a + 1) % self.wavetable.tables.len();

            let mut mix = 0.0f32;
            for v in self.voices.iter_mut() {
                mix += v.render(&self.wavetable, table_a, table_b, morph);
            }

            for out in channel_samples {
                *out = mix * gain;
            }
        }

        ProcessStatus::Normal
    }
}
