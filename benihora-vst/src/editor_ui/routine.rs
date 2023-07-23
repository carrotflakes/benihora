use nih_plug_egui::egui::{self, DragValue, ScrollArea};

use crate::{
    routine::{Event, Routine},
    synth::Synth,
};

use super::knob::{knob, knob_log};

pub fn show_routines(ui: &mut egui::Ui, synth: &mut Synth) {
    let id = ui.make_persistent_id("Routines");
    let selected_routine_id = id.with("selected_routine");
    let selected_event_id = id.with("selected_event");
    let selected = ui
        .data()
        .get_temp::<Option<usize>>(selected_routine_id)
        .unwrap_or_default();
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(index) = selected {
                ui.horizontal(|ui| {
                    if ui.link("List").clicked() {
                        ui.data()
                            .insert_temp::<Option<usize>>(selected_routine_id, None);
                        ui.data()
                            .insert_temp::<Option<usize>>(selected_event_id, None);
                    }
                    ui.text_edit_singleline(&mut synth.routines[index].name);
                });
                let selected_event = ui
                    .data()
                    .get_temp::<Option<usize>>(selected_event_id)
                    .unwrap_or_default();
                let mut remove_event = None;
                for (i, ev) in synth.routines[index].events.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut ev.0)
                                    .speed(0.01)
                                    .clamp_range(0.0..=10.0),
                            );
                            let res = ui.link(format!("{:?}", (ev.1).kind())).context_menu(|ui| {
                                if ui.button("Remove").clicked() {
                                    remove_event = Some(ev as *mut _);
                                    ui.close_menu();
                                }
                            });
                            if res.clicked() {
                                ui.data().insert_temp(selected_event_id, Some(i));
                            }
                        });

                        if selected_event != Some(i) {
                            return;
                        }
                        event_ui(
                            &mut ev.1,
                            ui,
                            synth.tongue_poses.len(),
                            synth.other_constrictions.len(),
                        );
                    });
                }

                if let Some(ptr) = remove_event {
                    synth.routines[index]
                        .events
                        .retain(|ev| ev as *const _ != ptr);
                }

                ui.menu_button("Add event", |ui| {
                    if ui.button("Tongue").clicked() {
                        synth.routines[index]
                            .events
                            .push((0.0, Event::Tongue { i: 0, speed: None }));
                        ui.close_menu();
                    }
                    if ui.button("Constriction").clicked() {
                        synth.routines[index].events.push((
                            0.0,
                            Event::Constriction {
                                i: 0,
                                strength: 0.0,
                            },
                        ));
                        ui.close_menu();
                    }
                    if ui.button("Velum").clicked() {
                        synth.routines[index]
                            .events
                            .push((0.0, Event::Velum { openness: 0.4 }));
                        ui.close_menu();
                    }
                    if ui.button("Pitch").clicked() {
                        synth.routines[index]
                            .events
                            .push((0.0, Event::Pitch { value: 0.0 }));
                        ui.close_menu();
                    }
                    if ui.button("Sound").clicked() {
                        synth.routines[index]
                            .events
                            .push((0.0, Event::Sound { sound: true }));
                        ui.close_menu();
                    }
                    if ui.button("Force diameter").clicked() {
                        synth.routines[index]
                            .events
                            .push((0.0, Event::ForceDiameter));
                        ui.close_menu();
                    }
                });
            } else {
                let mut remove_routine = None;
                let mut duplicate_routine = None;
                let mut preview_routine = None;
                for (i, r) in synth.routines.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let res = &ui
                            .link(&format!("{} ({})", &r.name, r.events.len()))
                            .context_menu(|ui| {
                                if ui.button("Duplicate").clicked() {
                                    duplicate_routine = Some(i);
                                    ui.close_menu();
                                }
                                if ui.button("Remove").clicked() {
                                    remove_routine = Some(i);
                                    ui.close_menu();
                                }
                            });
                        if res.clicked() {
                            ui.data().insert_temp(selected_routine_id, Some(i));
                        }

                        if ui.button("▶").clicked() {
                            preview_routine = Some(i);
                        }
                    });
                }

                if let Some(i) = duplicate_routine {
                    synth.routines.push(synth.routines[i].clone());
                }
                if let Some(i) = remove_routine {
                    synth.routines.remove(i);
                }
                if let Some(i) = preview_routine {
                    synth.trigger_routine(i);
                }

                if ui.button("New routine").clicked() {
                    synth.routines.push(Routine {
                        name: new_routine_name(synth),
                        events: Vec::new(),
                    });
                }
            }
        });
}

fn event_ui(ev: &mut Event, ui: &mut egui::Ui, tongue_poses: usize, other_constrictions: usize) {
    match ev {
        Event::Tongue { i, speed } => {
            ui.horizontal(|ui| {
                ui.add(DragValue::new(i).clamp_range(0..=tongue_poses));
                ui.label("Tongue ID");
            });

            ui.horizontal(|ui| {
                let mut remove_speed = false;
                if let Some(speed) = speed {
                    ui.add(knob_log(1.0..200.0, speed, "Speed"))
                        .context_menu(|ui| {
                            if ui.button("Remove").clicked() {
                                remove_speed = true;
                                ui.close_menu();
                            }
                        });
                }
                if remove_speed {
                    *speed = None;
                }
                if speed.is_none() {
                    if ui.button("Set speed").clicked() {
                        *speed = Some(20.0);
                    }
                }
            });
        }
        Event::Constriction { i, strength } => {
            ui.horizontal(|ui| {
                ui.add(DragValue::new(i).clamp_range(0..=other_constrictions));
                ui.label("Constriction ID");
            });

            ui.horizontal(|ui| ui.add(knob(0.0..1.0, strength, "Strength")));
        }
        Event::Velum { openness } => {
            ui.add(knob(0.01..0.4, openness, "Openness"));
        }
        Event::Pitch { value } => {
            ui.add(knob(-12.0..12.0, value, "Pitch"));
        }
        Event::Sound { sound } => {
            ui.checkbox(sound, "Sound");
        }
        Event::ForceDiameter => {}
    }
}

fn new_routine_name(synth: &Synth) -> String {
    let mut i = 1;
    loop {
        let name = format!("Routine {}", i);
        if !synth.routines.iter().any(|r| r.name == name) {
            return name;
        }
        i += 1;
    }
}
