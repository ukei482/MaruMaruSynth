pub struct Wavetable {
    pub tables: Vec<Vec<f32>>,
    pub table_size: usize,
}

impl Wavetable {
    pub fn new(table_size: usize) -> Self {
        Self { tables: Vec::new(), table_size }
    }

    pub fn add_table(&mut self, table: Vec<f32>) {
        assert_eq!(table.len(), self.table_size);
        self.tables.push(table);
    }

    fn table_for_index(&self, idx: usize) -> &Vec<f32> {
        &self.tables[idx % self.tables.len()]
    }

    pub fn sample_from_table(&self, table_idx: usize, phase: f32) -> f32 {
        let size = self.table_size as f32;
        let mut p = phase.fract();
        if p < 0.0 { p += 1.0; }
        p *= size;
        let i = p.floor() as usize % self.table_size;
        let next = (i + 1) % self.table_size;
        let frac = p - (i as f32);
        let table = self.table_for_index(table_idx);
        let a = table[i];
        let b = table[next];
        a + (b - a) * frac
    }

    pub fn morph_sample(&self, idx0: usize, idx1: usize, pos: f32, phase: f32) -> f32 {
        let s0 = self.sample_from_table(idx0, phase);
        let s1 = self.sample_from_table(idx1, phase);
        s0 + (s1 - s0) * pos
    }
    
}
