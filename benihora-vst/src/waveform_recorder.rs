pub struct WaveformRecorder {
    waveform: Vec<f32>,
    waveform_: Vec<f32>,
    last_phase: f32,
}

impl WaveformRecorder {
    pub fn new() -> Self {
        Self {
            waveform: Vec::new(),
            waveform_: Vec::new(),
            last_phase: 0.0,
        }
    }

    pub fn record(&mut self, phase: f32, x: f32) {
        if self.last_phase > phase {
            std::mem::swap(&mut self.waveform, &mut self.waveform_);
            self.waveform_.clear();
        }
        self.last_phase = phase;
        self.waveform_.push(x);
    }

    pub fn get_waveform(&self) -> &[f32] {
        &self.waveform
    }
}
