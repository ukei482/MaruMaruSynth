#[repr(C)]
pub struct MidiEvent {
    note_number: i32,
    velocity: f32,
    sample_position: i32,
}

#[unsafe(no_mangle)]

pub extern "C" fn rust_process
(


    buffer_ptr: *mut f32,
    num_samples: usize,
    midi_events: *const MidiEvent,
    num_midi_events: usize,
    blend: f32,
    lfo_rate: f32,
    lfo_depth: f32,


) 
{
    let buffer   = unsafe { std::slice::from_raw_parts_mut(buffer_ptr, num_samples) };
    let events = unsafe { std::slice::from_raw_parts(midi_events, num_midi_events) };

    // 仮: MIDIイベントをログ出力（開発中用）
    for e in events {
        println!(
            "MIDI note={} vel={} pos={}",
            e.note_number, e.velocity, e.sample_position
        );
    }

    // 仮: 簡単なオーディオ出力（blendパラメータで左右を切替）
    for i in 0..num_samples {
        let natural    = 0.2 * (i as f32 * 0.01).sin();    // 仮自然音
        let electronic = 0.2 * (i as f32 * 0.02).sin();    // 仮電子音
        buffer[i]           = natural * (1.0 - blend) + electronic * blend;
    }
}
