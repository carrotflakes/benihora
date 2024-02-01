#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod audio;
mod keyboard_ui;
#[cfg(target_arch = "wasm32")]
mod midi;
mod param;

pub use app::App;
pub use egui;
