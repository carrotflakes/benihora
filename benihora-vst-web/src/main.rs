#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let mut native_options = eframe::NativeOptions::default();
    native_options.default_theme = eframe::Theme::Dark;
    native_options.follow_system_theme = false;
    native_options.initial_window_size = Some(egui::vec2(800.0, 600.0));

    eframe::run_native(
        "Benihora VST standalone",
        native_options,
        Box::new(|cc| Box::new(benihora_vst_web::App::new(cc))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let mut web_options = eframe::WebOptions::default();
    web_options.default_theme = eframe::Theme::Dark;
    web_options.follow_system_theme = false;

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(benihora_vst_web::App::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
