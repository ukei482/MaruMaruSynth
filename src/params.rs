use nih_plug::prelude::*;

// NOTE: do NOT derive Clone here because IntParam/FloatParam do not implement Clone.
#[derive(Params)]
pub struct WtParams {
    // table index as integer (0..N-1)
    #[id = "wavetable"]
    pub wavetable: IntParam,

    #[id = "pos"]
    pub wavetable_pos: FloatParam,

    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "cutoff"]
    pub cutoff: FloatParam,

    #[id = "res"]
    pub resonance: FloatParam,
}

impl Default for WtParams {
    fn default() -> Self {
        Self {
            wavetable     : IntParam  ::new("Table"    , 0      , IntRange  ::Linear { min: 0, max: 2 }),
            wavetable_pos : FloatParam::new("Position" , 0.0    , FloatRange::Linear { min: 0.0, max: 1.0 }),
            gain          : FloatParam::new("Gain"     , 0.8    , FloatRange::Linear { min: 0.0, max: 1.0 }),
            cutoff        : FloatParam::new("Cutoff"   , 2000.0 , FloatRange::Linear { min: 20.0, max: 20000.0 }),
            resonance     : FloatParam::new("Res"      , 0.2    , FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}
