use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use benihora_vst_ui::{benihora::tract::DEFAULT_TONGUE, synth};
use egui::Id;

use crate::param::{FloatParam, FloatRange};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    #[serde(skip)]
    message: String,

    state: Arc<Mutex<State>>,

    #[serde(skip)]
    audio_result: Option<crate::audio::AudioResult>,

    #[serde(skip)]
    event_queue: Arc<Mutex<VecDeque<synth::Event>>>,

    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    midi: Arc<Mutex<crate::midi::MidiState>>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct State {
    synth: benihora_vst_ui::synth::Synth,

    vibrato_amount: FloatParam,
    vibrato_rate: FloatParam,
    frequency_wobble: FloatParam,
    tenseness_wobble: FloatParam,
    tongue_x: FloatParam,
    tongue_y: FloatParam,
}

impl Default for App {
    fn default() -> Self {
        Self {
            message: "".to_owned(),
            state: Arc::new(Mutex::new(State {
                synth: benihora_vst_ui::synth::Synth::new(),

                vibrato_amount: FloatParam::new(
                    "Vibrato Amount",
                    0.0,
                    FloatRange::Linear { min: 0.0, max: 0.1 },
                ),
                vibrato_rate: FloatParam::new(
                    "Vibrato Rate",
                    6.0,
                    FloatRange::Skewed {
                        min: 0.1,
                        max: 20.0,
                        factor: 1.0,
                    },
                ),
                frequency_wobble: FloatParam::new(
                    "Frequency Wobble",
                    0.1,
                    FloatRange::Linear { min: 0.0, max: 5.0 },
                ),
                tenseness_wobble: FloatParam::new(
                    "Tenseness Wobble",
                    1.0,
                    FloatRange::Linear { min: 0.0, max: 5.0 },
                ),

                tongue_x: FloatParam::new(
                    "Tongue X",
                    DEFAULT_TONGUE.0,
                    FloatRange::Linear {
                        min: 12.0,
                        max: 28.0,
                    },
                ),
                tongue_y: FloatParam::new(
                    "Tongue Y",
                    DEFAULT_TONGUE.1,
                    FloatRange::Linear { min: 2.0, max: 4.0 },
                ),
            })),
            audio_result: None,
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            #[cfg(target_arch = "wasm32")]
            midi: Arc::new(Mutex::new(Default::default())),
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        #[allow(unused_mut)]
        let mut this = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        #[cfg(not(target_arch = "wasm32"))]
        this.start_audio();

        #[cfg(target_arch = "wasm32")]
        {
            let event_queue = this.event_queue.clone();
            this.midi
                .lock()
                .unwrap()
                .init_midi_handler(move |data| match data {
                    [144, nn, velocity] => {
                        event_queue.lock().unwrap().push_back(synth::Event::NoteOn {
                            note: *nn,
                            velocity: *velocity as f32 / 127.0,
                        });
                    }
                    [128, nn, _] => {
                        event_queue
                            .lock()
                            .unwrap()
                            .push_back(synth::Event::NoteOff { note: *nn });
                    }
                    _ => {}
                });
        }

        this
    }

    fn start_audio(&mut self) {
        log::info!("start_audio");

        match crate::audio::start_audio() {
            Ok(audio_res) => {
                let channels = audio_res.channels;
                let sample_rate = audio_res.sample_rate as f32;
                let state = self.state.clone();
                let event_queue = self.event_queue.clone();

                {
                    let mut state = state.lock().unwrap();
                    state.synth.ensure_benihora(sample_rate);
                }

                let dtime = 1.0 / sample_rate;

                *audio_res.callback.lock().unwrap() = Box::new(move |len| {
                    let mut buffer = Vec::with_capacity(len);
                    let mut state = state.lock().unwrap();
                    let mut event_queue = event_queue.lock().unwrap();

                    state.synth.ensure_benihora(sample_rate);

                    let mut event: Option<(usize, synth::Event)> =
                        event_queue.pop_front().map(|e| (0, e));

                    for i in 0..len / channels {
                        state.synth.benihora_params.vibrato_amount =
                            state.vibrato_amount.smoothed_next();
                        state.synth.benihora_params.vibrato_rate =
                            state.vibrato_rate.smoothed_next();
                        state.synth.benihora_params.frequency_wobble_amount =
                            state.frequency_wobble.smoothed_next();
                        state.synth.benihora_params.tenseness_wobble_amount =
                            state.tenseness_wobble.smoothed_next();
                        if state.synth.tongue_control == synth::Control::Host {
                            state.synth.benihora.as_mut().unwrap().tract.tongue_target.0 =
                                state.tongue_x.smoothed_next();
                            state.synth.benihora.as_mut().unwrap().tract.tongue_target.1 =
                                state.tongue_y.smoothed_next();
                        }

                        while let Some((timing, e)) = &event {
                            if *timing <= i {
                                state.synth.handle_event(&e);
                                event = event_queue.pop_front().map(|e| (0, e));
                            } else {
                                break;
                            }
                        }

                        let sample = state.synth.process(dtime);
                        for _ in 0..channels {
                            buffer.push(sample);
                        }
                    }

                    buffer
                });

                self.audio_result = Some(audio_res);
            }
            Err(e) => {
                self.message = format!("start_audio failed: {}", e);
            }
        };
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        #[cfg(target_arch = "wasm32")]
        let show_midi_settings_id = Id::new("showMidiSettings");

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.label("Benihora🐚");

                ui.add_space(8.0);

                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            // ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            _frame.close();
                        }
                    });
                    ui.add_space(16.0);
                }

                // egui::widgets::global_dark_light_mode_buttons(ui);

                #[cfg(target_arch = "wasm32")]
                {
                    let mut show_midi_settings = ui.data(|d| {
                        d.get_temp::<bool>(show_midi_settings_id)
                            .unwrap_or_default()
                    });
                    ui.toggle_value(&mut show_midi_settings, "MIDI settings");
                    ui.data_mut(|d| d.insert_temp(show_midi_settings_id, show_midi_settings));
                }

                if self.audio_result.is_some() {
                    if ui.button("Stop audio").clicked() {
                        self.audio_result = None;
                    }
                } else {
                    if ui.button("Start audio").clicked() {
                        self.start_audio();
                    }
                }

                if let Some(ar) = &self.audio_result {
                    ui.label(format!(
                        "Sample rate: {} Hz, channels: {}",
                        ar.sample_rate, ar.channels
                    ));
                } else {
                    ui.label("Audio not started.");
                }

                if !self.message.is_empty() {
                    ui.separator();
                    ui.label(&self.message);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            #[cfg(target_arch = "wasm32")]
            if ui.data(|d| {
                d.get_temp::<bool>(show_midi_settings_id)
                    .unwrap_or_default()
            }) {
                crate::midi::midi_settings_ui(&mut self.midi, ui);
                return;
            }

            if self.audio_result.is_none() {
                if ui.button("Start").clicked() {
                    self.start_audio();
                }
                return;
            }

            let mut state = self.state.lock().unwrap();
            let State {
                synth,
                vibrato_amount,
                vibrato_rate,
                frequency_wobble,
                tenseness_wobble,
                tongue_x,
                tongue_y,
            }: &mut State = &mut *state;
            benihora_vst_ui::ui::show(
                ui,
                synth,
                vibrato_amount,
                vibrato_rate,
                frequency_wobble,
                tenseness_wobble,
                tongue_x,
                tongue_y,
            );
            let current_note = synth.voice_manager.get_voice();

            ui.separator();

            crate::keyboard_ui::show(ui, current_note, &mut |is_up, note| {
                if is_up {
                    self.event_queue
                        .lock()
                        .unwrap()
                        .push_back(synth::Event::NoteOff { note });
                } else {
                    self.event_queue
                        .lock()
                        .unwrap()
                        .push_back(synth::Event::NoteOn {
                            note,
                            velocity: 1.0,
                        });
                }
            });

            handle_input(ctx, &mut *self.event_queue.lock().unwrap());
        });

        ctx.request_repaint();
    }
}

fn handle_input(ctx: &egui::Context, events: &mut VecDeque<synth::Event>) {
    ctx.input(|i| {
        for (k, n) in [
            (egui::Key::Z, 60),
            (egui::Key::S, 61),
            (egui::Key::X, 62),
            (egui::Key::D, 63),
            (egui::Key::C, 64),
            (egui::Key::V, 65),
            (egui::Key::G, 66),
            (egui::Key::B, 67),
            (egui::Key::H, 68),
            (egui::Key::N, 69),
            (egui::Key::J, 70),
            (egui::Key::M, 71),
        ] {
            if i.events
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        egui::Event::Key { key, pressed: true, repeat: false, .. }
                        if *key == k
                    )
                })
                .count()
                > 0
            {
                events.push_back(synth::Event::NoteOn {
                    note: n,
                    velocity: 1.0,
                });
            }
            if i.key_released(k) {
                events.push_back(synth::Event::NoteOff { note: n });
            }
        }
    });
}
