// no unused imports
#[derive(Clone)]
pub struct Voice {
    pub active    : bool,
    pub note      : u8,
    pub velocity  : f32,
    pub phase     : f32,
    pub phase_inc : f32,
}

impl Voice {
    pub fn new() -> Self {
        Self { active: false, note: 0, velocity: 0.0, phase: 0.0, phase_inc: 0.0 }
    }

    pub fn note_on(&mut self, note: u8, velocity: f32, sample_rate: f32) {
        let freq    = 440.0 * (2.0f32).powf((note as f32 - 69.0) / 12.0);
        self.phase       = 0.0;
        self.phase_inc   = freq / sample_rate;
        self.velocity    = velocity;
        self.note        = note;
        self.active      = true;
    }

    pub fn note_off(&mut self) {
        self.active = false;
    }

    pub fn render(&mut self, wt: &crate::wavetable::Wavetable, table_a: usize, table_b: usize, morph: f32) -> f32 {
        if !self.active { return 0.0; }
        let s = wt.morph_sample(table_a, table_b, morph, self.phase);
        self.phase = (self.phase + self.phase_inc) % 1.0;
        s * self.velocity
    }
}
