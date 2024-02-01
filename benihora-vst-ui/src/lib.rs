mod benihora_managed;
mod routine;
pub mod synth;
pub mod ui;
mod voice_manager;
mod waveform_recorder;

pub use benihora;
pub use egui;

use std::cell::RefCell;

thread_local! {
    pub(crate) static FFT_PLANNER: RefCell<rustfft::FftPlanner<f32>> = RefCell::new(rustfft::FftPlanner::new());
}
