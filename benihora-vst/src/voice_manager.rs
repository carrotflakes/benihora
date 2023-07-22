#[derive(Default)]
pub struct VoiceManager {
    voices: Vec<u8>,
}

impl VoiceManager {
    pub fn new() -> Self {
        Self { voices: Vec::new() }
    }

    pub fn get_voice(&mut self) -> Option<u8> {
        self.voices.last().copied()
    }

    pub fn noteon(&mut self, note: u8) {
        self.voices.push(note);
    }

    pub fn noteoff(&mut self, note: u8) {
        self.voices.retain(|&n| n != note)
    }
}
