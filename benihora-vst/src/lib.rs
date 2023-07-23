mod benihora_managed;
mod editor_ui;
mod routine;
mod synth;
mod voice_manager;
mod waveform_recorder;

use benihora::tract::DEFAULT_TONGUE;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

thread_local! {
    pub static FFT_PLANNER: RefCell<rustfft::FftPlanner<f32>> = RefCell::new(rustfft::FftPlanner::new());
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
    #[id = "tongue_x"]
    pub tongue_x: FloatParam,
    #[id = "tongue_y"]
    pub tongue_y: FloatParam,

    #[persist = "synth"]
    pub synth: Arc<Mutex<synth::Synth>>,
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
            editor_state: EguiState::from_size(320, 220),

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

            tongue_x: FloatParam::new(
                "Tongue X",
                DEFAULT_TONGUE.0 as f32,
                FloatRange::Linear {
                    min: 12.0,
                    max: 28.0,
                },
            ),
            tongue_y: FloatParam::new(
                "Tongue Y",
                DEFAULT_TONGUE.1 as f32,
                FloatRange::Linear { min: 2.0, max: 4.0 },
            ),

            synth: Arc::new(Mutex::new(synth::Synth::new())),
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
        synth.ensure_benihora(sample_rate);

        let mut count = 0;
        let mut event = context.next_event();
        let dtime = 1.0 / sample_rate;

        for mut channel_samples in buffer.iter_samples() {
            synth.benihora_params.vibrato_amount =
                self.params.vibrato_amount.smoothed.next() as f64;
            if synth.tongue_control == synth::Control::Host {
                synth.benihora.as_mut().unwrap().tract.tongue_target.0 =
                    self.params.tongue_x.smoothed.next() as f64;
                synth.benihora.as_mut().unwrap().tract.tongue_target.1 =
                    self.params.tongue_y.smoothed.next() as f64;
            }

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
