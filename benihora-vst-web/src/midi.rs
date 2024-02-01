use std::{
    cell::OnceCell,
    sync::{Arc, Mutex},
};
use web_sys::wasm_bindgen::JsCast;

#[derive(Default)]
pub struct MidiState {
    pub inputs: Vec<(bool, web_sys::MidiInput)>,
    pub midi_handler: OnceCell<
        web_sys::wasm_bindgen::closure::Closure<dyn FnMut(web_sys::wasm_bindgen::JsValue)>,
    >,
}

impl MidiState {
    pub fn init_midi_handler(&self, mut callback: impl FnMut(&[u8]) + 'static) {
        self.midi_handler
            .set(web_sys::wasm_bindgen::closure::Closure::<
                dyn FnMut(web_sys::wasm_bindgen::JsValue),
            >::new(
                move |event: web_sys::wasm_bindgen::JsValue| {
                    let event = event.dyn_into::<web_sys::MidiMessageEvent>().unwrap();
                    if let Ok(data) = event.data() {
                        // log::info!("MIDI event: {:?}", data);
                        callback(data.as_slice());
                    }
                },
            ))
            .unwrap()
    }
}

pub fn midi_settings_ui(state: &mut Arc<Mutex<MidiState>>, ui: &mut egui::Ui) {
    ui.heading("MIDI input devices:");

    if ui.button("Update").clicked() {
        let midi_ref = state.clone();
        let closure = web_sys::wasm_bindgen::closure::Closure::new(
            move |midi_access: web_sys::wasm_bindgen::JsValue| {
                log::info!("midi_access: {:?}", midi_access);
                let midi_access = midi_access.dyn_into::<web_sys::MidiAccess>().unwrap();
                let mut midi = midi_ref.lock().unwrap();
                midi.inputs
                    .iter_mut()
                    .for_each(|(exists, _)| *exists = false);
                for input in midi_access.inputs().values() {
                    let input = input.unwrap();
                    let input = input.dyn_into::<web_sys::MidiInput>().unwrap();
                    let mi = midi.inputs.iter_mut().find(|(_, i)| i.id() == input.id());
                    if let Some(mi) = mi {
                        mi.0 = true;
                    } else {
                        midi.inputs.push((true, input));
                    }
                }
            },
        );
        let promise = web_sys::window()
            .unwrap()
            .navigator()
            .request_midi_access()
            .unwrap()
            .then(&closure);
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        wasm_bindgen_futures::spawn_local(async {
            future.await.unwrap();
            drop(closure);
        });
    }

    let midi = state.lock().unwrap();
    if midi.inputs.is_empty() {
        ui.label("No devices found.");
        return;
    }

    for (i, (exists, midi_input)) in midi.inputs.iter().enumerate() {
        ui.group(|ui| {
            ui.label(format!(
                "{}: {}",
                i + 1,
                midi_input.name().unwrap_or_default(),
            ));
            if *exists {
                let mut connected = midi_input.onmidimessage().is_some();
                if ui.checkbox(&mut connected, "Connect").changed() {
                    if connected {
                        let closure = midi.midi_handler.get().unwrap().as_ref().unchecked_ref();
                        midi_input.set_onmidimessage(Some(closure));
                    } else {
                        midi_input.set_onmidimessage(None);
                    }
                }
            } else {
                ui.label("Disconnected");
            }
        });
    }
}
