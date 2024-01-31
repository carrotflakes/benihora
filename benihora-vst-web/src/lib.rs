#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod audio;
mod keyboard_ui;
mod midi;
mod param;

pub use app::App;
pub use egui;
