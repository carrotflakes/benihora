mod benihora_managed;
mod editor_ui;
mod routine;
mod voice_manager;
mod waveform_recorder;

use benihora_managed::{BenihoraManaged, Params as BenihoraParams};
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState};
use routine::{Routine, Runtime};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
use voice_manager::VoiceManager;

thread_local! {
    pub static FFT_PLANNER: RefCell<rustfft::FftPlanner<f32>> = RefCell::new(rustfft::FftPlanner::new());
}

#[derive(Serialize, Deserialize)]
pub struct Synth {
    // Don't forget to add serde default to new fields
    sound_speed: f64,
    seed: u32,
    benihora_params: BenihoraParams,
    tongue_poses: Vec<(f64, f64)>,
    other_constrictions: Vec<(f64, f64)>,
    routines: Vec<Routine>,
    default_routine: usize,

    #[serde(skip)]
    time: f64,
    #[serde(skip)]
    note_off_time: f64,
    #[serde(skip)]
    benihora: Option<BenihoraManaged>,
    #[serde(skip)]
    voice_manager: VoiceManager,
    #[serde(skip)]
    routine_runtime: Runtime,
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
                    events: vec![
                        (
                            0.0,
                            routine::Event::Tongue {
                                i: 0,
                                speed: Some(200.0),
                            },
                        ),
                        (
                            0.1,
                            routine::Event::Tongue {
                                i: 2,
                                speed: Some(20.0),
                            },
                        ),
                    ],
                },
                Routine {
                    events: vec![
                        (0.0, routine::Event::Sound { sound: false }),
                        (
                            0.0,
                            routine::Event::Constriction {
                                i: 1,
                                strength: 0.7,
                            },
                        ),
                        (
                            0.08,
                            routine::Event::Constriction {
                                i: 1,
                                strength: -5.0,
                            },
                        ),
                        (0.01, routine::Event::Sound { sound: true }),
                    ],
                },
            ],
            default_routine: 0,
            time: 0.0,
            note_off_time: 0.0,
            benihora: None,
            voice_manager: VoiceManager::new(),
            routine_runtime: Runtime::new(),
        }
    }

    pub fn process(&mut self, dtime: f64) -> f64 {
        let benihora = self.benihora.as_mut().unwrap();
        self.routine_runtime.process(dtime, |e| match e {
            routine::Event::Tongue { i, speed } => {
                benihora.tract.tongue_target = (self.tongue_poses[i].0, self.tongue_poses[i].1);
                if let Some(speed) = speed {
                    benihora.tract.speed = speed;
                }
            }
            routine::Event::Constriction { i, strength } => {
                let diameter = self.other_constrictions[i].1 * (1.0 - strength);
                benihora.benihora.tract.source.other_constrictions[i].1 = diameter;
            }
            routine::Event::Velum { openness } => {
                benihora
                    .benihora
                    .tract
                    .set_velum_target(0.01 + (0.4 - 0.01) * openness);
            }
            routine::Event::Pitch { value } => {
                benihora.frequency.pitchbend = value;
            }
            routine::Event::Sound { sound } => {
                benihora.sound = sound;
            }
        });

        benihora.process(&self.benihora_params)
    }

    pub fn handle_event(&mut self, time: f64, event: &NoteEvent<()>) {
        let base = 0;
        #[allow(unused_variables)]
        match event {
            NoteEvent::NoteOn {
                channel,
                note,
                velocity,
                ..
            } => {
                self.ensure_other_constriction();
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
                    let diameter = self.other_constrictions[i].1 * (1.0 - *velocity as f64);
                    benihora.benihora.tract.source.other_constrictions[i].1 = diameter;
                    benihora.benihora.tract.update_diameter();
                    return;
                }
                let base = base + self.other_constrictions.len() as u8;
                if *note == base {
                    benihora.benihora.tract.set_velum_target(0.4);
                    return;
                }
                let base = base + 1;
                if *note < base + self.routines.len() as u8 {
                    self.routine_runtime
                        .push_routine(&self.routines[(*note - base) as usize]);
                    return;
                }

                let frequency_reset_time = 0.25;
                let muted = benihora.intensity.get() < 0.01
                    && self.note_off_time + frequency_reset_time < time;
                self.voice_manager.noteon(*note);
                if let Some(note) = self.voice_manager.get_voice() {
                    benihora
                        .frequency
                        .set(440.0 * 2.0f64.powf((note as f64 - 69.0) / 12.0), muted);
                    benihora.set_tenseness(*velocity as f64);
                    benihora.sound = true;
                    if (1..=self.routines.len()).contains(&self.default_routine) {
                        self.routine_runtime
                            .push_routine(&self.routines[self.default_routine - 1]);
                    }
                }
            }
            NoteEvent::NoteOff {
                channel,
                note,
                velocity,
                ..
            } => {
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
                        .set(440.0 * 2.0f64.powf((note as f64 - 69.0) / 12.0), false);
                    benihora.sound = true;
                } else {
                    benihora.sound = false;
                    self.note_off_time = time;
                }
            }
            NoteEvent::PolyPressure {
                channel,
                note,
                pressure,
                ..
            } => {} // = aftertouch
            NoteEvent::MidiChannelPressure {
                timing,
                channel,
                pressure,
            } => {} // = channel aftertouch
            NoteEvent::MidiPitchBend {
                timing,
                channel,
                value,
            } => {
                let pitchbend = 2.0f64.powf((*value as f64 * 2.0 - 1.0) / 12.0);
                self.benihora.as_mut().unwrap().frequency.pitchbend = pitchbend;
            }
            NoteEvent::MidiCC {
                timing,
                channel,
                cc,
                value,
            } => {}
            NoteEvent::MidiProgramChange {
                timing,
                channel,
                program,
            } => {}
            _ => {}
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
}

struct MyPlugin {
    params: Arc<MyPluginParams>,
}

#[derive(Params)]
struct MyPluginParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "vibrato_amount"]
    pub vibrato_amount: FloatParam,

    #[persist = "synth"]
    pub synth: Arc<Mutex<Synth>>,
}

impl Default for MyPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(MyPluginParams::default()),
        }
    }
}

impl Default for MyPluginParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(300, 180),

            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            vibrato_amount: FloatParam::new(
                "Vibrato Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 0.1 },
            ),

            synth: Arc::new(Mutex::new(Synth::new())),
        }
    }
}

impl Plugin for MyPlugin {
    const NAME: &'static str = "benihora";
    const VENDOR: &'static str = "carrotflakes";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "carrotflakes@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(0),
        main_output_channels: NonZeroU32::new(1),

        aux_input_ports: &[],
        aux_output_ports: &[],

        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        create_egui_editor(
            self.params.editor_state.clone(),
            self.params.clone(),
            |_, _| {},
            editor_ui::editor_ui,
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut synth = self.params.synth.lock().unwrap();

        let sample_rate = context.transport().sample_rate as f64;
        if synth.benihora.is_none() {
            synth.benihora = Some(BenihoraManaged::new(
                synth.sound_speed,
                sample_rate,
                1.0,
                synth.seed,
            ));
        }

        let mut count = 0;
        let mut event = context.next_event();
        let dtime = 1.0 / sample_rate;

        for mut channel_samples in buffer.iter_samples() {
            synth.benihora_params.vibrato_amount =
                self.params.vibrato_amount.smoothed.next() as f64;

            let current_time = synth.time;

            while let Some(e) = event {
                if e.timing() <= count {
                    synth.handle_event(current_time, &e);
                    event = context.next_event();
                } else {
                    break;
                }
            }
            count += 1;

            *channel_samples.get_mut(0).unwrap() = synth.process(dtime) as f32;
            synth.time += dtime;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for MyPlugin {
    const CLAP_ID: &'static str = "benihora";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for MyPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"benihora\0\0\0\0\0\0\0\0";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument];
}

// nih_export_clap!(MyPlugin);
nih_export_vst3!(MyPlugin);
