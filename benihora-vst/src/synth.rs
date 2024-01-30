use crate::benihora_managed::{BenihoraManaged, Params as BenihoraParams};
use crate::routine::{self, Routine, Runtime};
use crate::voice_manager::VoiceManager;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Synth {
    // Don't forget to add serde default to new fields
    pub sound_speed: f32,
    pub seed: u32,
    pub benihora_params: BenihoraParams,
    pub tongue_poses: Vec<(f32, f32)>,
    pub other_constrictions: Vec<(f32, f32)>,
    pub routines: Vec<Routine>,
    pub noteon_routine: usize,
    pub noteoff_routine: usize,
    pub tongue_control: Control,

    #[serde(skip)]
    pub time: f32,
    #[serde(skip)]
    pub note_off_time: f32,
    #[serde(skip)]
    pub benihora: Option<BenihoraManaged>,
    #[serde(skip)]
    pub voice_manager: VoiceManager,
    #[serde(skip)]
    pub routine_runtime: Runtime,
    #[serde(skip)]
    reset_required: bool,
    #[serde(skip)]
    random_tongue: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Control {
    Host,
    Internal,
}

#[derive(Debug, Clone)]
pub enum Event {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
    PitchBend { value: f32 },
}

impl Synth {
    pub fn new() -> Self {
        Synth {
            sound_speed: 3.0,
            seed: 0,
            benihora_params: BenihoraParams::new(),
            tongue_poses: vec![
                (27.2, 2.20), // i
                (19.4, 3.43), // e
                (12.9, 2.43), // a
                (14.0, 2.09), // o
                (22.8, 2.05), // u
            ],
            other_constrictions: vec![(25.0, 1.0), (30.0, 1.0), (35.0, 1.0), (41.0, 1.6)],
            routines: vec![
                Routine {
                    name: "Tongue move".to_string(),
                    events: vec![
                        (
                            0.0,
                            routine::Event::Tongue {
                                index: routine::TongueIndex::Index(0),
                                speed: Some(200.0),
                            },
                        ),
                        (
                            0.1,
                            routine::Event::Tongue {
                                index: routine::TongueIndex::Index(1),
                                speed: Some(20.0),
                            },
                        ),
                    ],
                },
                Routine {
                    name: "Tap".to_string(),
                    events: vec![
                        (0.0, routine::Event::Sound { sound: false }),
                        (
                            0.0,
                            routine::Event::Constriction {
                                i: 1,
                                strength: Some(0.7),
                            },
                        ),
                        (0.0, routine::Event::ForceDiameter),
                        (
                            0.0,
                            routine::Event::Constriction {
                                i: 1,
                                strength: None,
                            },
                        ),
                        (0.01, routine::Event::Sound { sound: true }),
                    ],
                },
            ],
            noteon_routine: 0,
            noteoff_routine: 0,
            time: 0.0,
            note_off_time: 0.0,
            benihora: None,
            voice_manager: VoiceManager::new(),
            routine_runtime: Runtime::new(),
            tongue_control: Control::Internal,
            reset_required: true,
            random_tongue: 1,
        }
    }

    pub fn trigger_routine(&mut self, index: usize) {
        if self.routines.len() <= index {
            return;
        }
        self.routine_runtime.push_routine(&self.routines[index]);
    }

    pub fn process(&mut self, dtime: f32) -> f32 {
        let benihora = self.benihora.as_mut().unwrap();
        self.routine_runtime.process(dtime, |e| match e {
            routine::Event::Tongue { index, speed } => {
                match index {
                    routine::TongueIndex::Index(i) => {
                        if self.tongue_poses.len() <= i {
                            return;
                        }
                        benihora.tract.tongue_target = self.tongue_poses[i];
                    }
                    routine::TongueIndex::Random => {
                        let seed = &mut self.random_tongue;
                        *seed = seed.overflowing_mul(48271).0 % ((1 << 31) - 1);

                        benihora.tract.tongue_target =
                            self.tongue_poses[*seed as usize % self.tongue_poses.len()];
                    }
                }
                if let Some(speed) = speed {
                    benihora.tract.speed = speed;
                }
            }
            routine::Event::Constriction { i, strength } => {
                if self.other_constrictions.len() <= i {
                    return;
                }
                let diameter = if let Some(strength) = strength {
                    self.other_constrictions[i].1 * (1.0 - strength)
                } else {
                    10.0
                };
                benihora.benihora.tract.source.other_constrictions[i].1 = diameter;
            }
            routine::Event::Velum { openness } => {
                benihora
                    .benihora
                    .tract
                    .set_velum_target(0.01 + (0.4 - 0.01) * openness);
            }
            routine::Event::Pitch { value } => {
                benihora.frequency.pitchbend = 2.0f32.powf((value as f32 * 2.0 - 1.0) / 12.0);
            }
            routine::Event::Sound { sound } => {
                benihora.sound = sound;
            }
            routine::Event::ForceDiameter => {
                benihora.benihora.tract.update_diameter();
                benihora.benihora.tract.current_diameter =
                    benihora.benihora.tract.target_diameter.clone();
            }
        });

        benihora.process(&self.benihora_params)
    }

    pub fn handle_event(&mut self, time: f32, event: &Event) {
        let base = 0;

        match event {
            Event::NoteOn { note, velocity } => {
                let benihora = self.benihora.as_mut().unwrap();
                if (base..base + self.tongue_poses.len() as u8).contains(note) {
                    let (index, diameter) = self.tongue_poses[*note as usize - base as usize];
                    benihora.tract.tongue_target =
                        benihora.benihora.tract.source.tongue_clamp(index, diameter);
                    return;
                }
                let base = base + self.tongue_poses.len() as u8;
                if (base..base + self.other_constrictions.len() as u8).contains(note) {
                    let i = *note as usize - base as usize;
                    let diameter = self.other_constrictions[i].1 * (1.0 - *velocity as f32);
                    benihora.benihora.tract.source.other_constrictions[i].1 = diameter;
                    benihora.benihora.tract.update_diameter();
                    return;
                }
                let base = base + self.other_constrictions.len() as u8;
                if *note == base {
                    benihora
                        .benihora
                        .tract
                        .set_velum_target(0.01 + (0.4 - 0.01) * *velocity as f32);
                    return;
                }
                let base = base + 1;
                if *note < base + self.routines.len() as u8 {
                    self.trigger_routine((*note - base) as usize);
                    return;
                }

                let frequency_reset_time = 0.25;
                let muted = benihora.intensity.get() < 0.01
                    && self.note_off_time + frequency_reset_time < time;
                self.voice_manager.noteon(*note);
                if let Some(note) = self.voice_manager.get_voice() {
                    benihora
                        .frequency
                        .set(440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0), muted);
                    benihora.set_tenseness(*velocity as f32);
                    benihora.sound = true;
                    if (1..=self.routines.len()).contains(&self.noteon_routine) {
                        self.trigger_routine(self.noteon_routine - 1);
                    }
                }
            }
            Event::NoteOff { note } => {
                let benihora = self.benihora.as_mut().unwrap();
                let base = base + self.tongue_poses.len() as u8;
                if (base..base + self.other_constrictions.len() as u8).contains(note) {
                    let i = *note as usize - base as usize;
                    benihora.benihora.tract.source.other_constrictions[i].1 = 10.0;
                    benihora.benihora.tract.update_diameter();
                    return;
                }
                let base = base + self.other_constrictions.len() as u8;
                if *note == base {
                    benihora.benihora.tract.set_velum_target(0.01);
                    return;
                }
                let base = base + 1;
                if *note < base + self.routines.len() as u8 {
                    return;
                }

                self.voice_manager.noteoff(*note);
                if let Some(note) = self.voice_manager.get_voice() {
                    benihora
                        .frequency
                        .set(440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0), false);
                    benihora.sound = true;
                } else {
                    benihora.sound = false;
                    self.note_off_time = time;
                    if (1..=self.routines.len()).contains(&self.noteoff_routine) {
                        self.trigger_routine(self.noteoff_routine - 1);
                    }
                }
            }
            Event::PitchBend { value } => {
                self.benihora.as_mut().unwrap().frequency.pitchbend = 2.0f32.powf(*value);
            }
        }
    }

    pub fn ensure_benihora(&mut self, sample_rate: f32) {
        if self.benihora.is_none() || self.reset_required {
            self.benihora = Some(BenihoraManaged::new(
                self.sound_speed,
                sample_rate,
                1.0,
                self.seed,
            ));
            self.ensure_other_constriction();
            self.random_tongue = self.seed + 1;
            self.reset_required = false;
        }
    }

    pub fn ensure_other_constriction(&mut self) {
        let benihora = self.benihora.as_mut().unwrap();
        if benihora
            .benihora
            .tract
            .source
            .other_constrictions
            .is_empty()
        {
            benihora.benihora.tract.source.other_constrictions = self
                .other_constrictions
                .iter()
                .map(|x| (x.0, 10.0))
                .collect();
        }
    }

    pub fn request_reset(&mut self) {
        self.reset_required = true;
    }
}
