use nih_plug_egui::egui;

use crate::synth::Synth;

pub fn show_routines(ui: &mut egui::Ui, synth: &mut Synth) {
    let id = ui.make_persistent_id("routines");
    let selected = ui
        .data()
        .get_temp::<Option<usize>>(id.with("selected"))
        .unwrap_or_default();
    if let Some(index) = selected {
        if ui.button("back").clicked() {
            ui.data()
                .insert_temp::<Option<usize>>(id.with("selected"), None);
        }
        for ev in &mut synth.routines[index].events {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut ev.0)
                            .speed(0.01)
                            .clamp_range(0.0..=10.0),
                    );
                    ui.label(format!("{:?}", (ev.1).kind()));
                });
            });
        }
    } else {
        for (i, r) in synth.routines.iter().enumerate() {
            if ui.button(&format!("{} ({})", i, r.events.len())).clicked() {
                ui.data().insert_temp(id.with("selected"), Some(i));
            }
        }
        if ui.button("add").clicked() {
            synth.routines.push(Default::default());
        }
    }
}
