use nih_plug_egui::egui::{self, Button, ComboBox, ScrollArea};

use crate::{
    routine::{Event, Routine, TongueIndex},
    synth::Synth,
};

use super::knob::{knob, knob_log};

pub fn show_routines(ui: &mut egui::Ui, synth: &mut Synth) {
    let id = ui.make_persistent_id("Routines");
    let selected_routine_id = id.with("selected_routine");
    let selected_event_id = id.with("selected_event");
    let selected = ui.data(|d| {
        d.get_temp::<Option<usize>>(selected_routine_id)
            .unwrap_or_default()
    });
    let mut preview_routine = None;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(index) = selected {
                ui.horizontal(|ui| {
                    if ui.link("List").clicked() {
                        ui.data_mut(|d| d.insert_temp::<Option<usize>>(selected_routine_id, None));
                        ui.data_mut(|d| d.insert_temp::<Option<usize>>(selected_event_id, None));
                    }
                    // ui.text_edit_singleline(&mut synth.routines[index].name);
                    ui.label(format!("Routine {}", index + 1));
                    if ui.button("▶").clicked() {
                        preview_routine = Some(index);
                    }
                    if synth.noteon_routine == index + 1 {
                        if ui
                            .add(
                                Button::new("D")
                                    .small()
                                    .fill(ui.style().visuals.selection.bg_fill),
                            )
                            .clicked()
                        {
                            synth.noteon_routine = 0;
                        }
                    } else {
                        if ui.add(Button::new("D").small()).clicked() {
                            synth.noteon_routine = index + 1;
                        }
                    }
                    if synth.noteoff_routine == index + 1 {
                        if ui
                            .add(
                                Button::new("U")
                                    .small()
                                    .fill(ui.style().visuals.selection.bg_fill),
                            )
                            .clicked()
                        {
                            synth.noteoff_routine = 0;
                        }
                    } else {
                        if ui.add(Button::new("U").small()).clicked() {
                            synth.noteoff_routine = index + 1;
                        }
                    }
                });
                let selected_event = ui.data(|d| {
                    d.get_temp::<Option<usize>>(selected_event_id)
                        .unwrap_or_default()
                });
                let mut remove_event = None;
                for (i, ev) in synth.routines[index].events.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut ev.0)
                                .speed(0.01)
                                .clamp_range(0.0..=10.0),
                        );
                        let res = ui.link(format!("{}", (ev.1).name())).context_menu(|ui| {
                            if ui.button("Remove").clicked() {
                                remove_event = Some(ev as *mut _);
                                ui.close_menu();
                            }
                        });
                        if res.clicked() {
                            ui.data_mut(|d| d.insert_temp(selected_event_id, Some(i)));
                        }
                    });

                    if selected_event == Some(i) {
                        event_ui(
                            &mut ev.1,
                            ui,
                            synth.tongue_poses.len(),
                            synth.other_constrictions.len(),
                        );
                    }
                    ui.separator();
                }

                if let Some(ptr) = remove_event {
                    synth.routines[index]
                        .events
                        .retain(|ev| ev as *const _ != ptr);
                }

                ui.menu_button("Add event", add_event_ui(synth, index));
            } else {
                let mut remove_routine = None;
                let mut duplicate_routine = None;
                let mut merge = None;
                for (i, r) in synth.routines.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let res = &ui
                            .link(&format!("Routine {} ({})", i + 1, r.events.len()))
                            .context_menu(|ui| {
                                if ui.button("Duplicate").clicked() {
                                    duplicate_routine = Some(i);
                                    ui.close_menu();
                                }
                                if ui.button("Remove").clicked() {
                                    remove_routine = Some(i);
                                    ui.close_menu();
                                }
                                for (j, _) in synth.routines.iter().enumerate() {
                                    if i == j {
                                        continue;
                                    }
                                    if ui
                                        .button(&format!("Merge with Routine {}", j + 1,))
                                        .clicked()
                                    {
                                        merge = Some((i, j));
                                        ui.close_menu();
                                    }
                                }
                            });
                        if res.clicked() {
                            ui.data_mut(|d| d.insert_temp(selected_routine_id, Some(i)));
                        }

                        if ui.small_button("▶").clicked() {
                            preview_routine = Some(i);
                        }
                        if synth.noteon_routine == i + 1 {
                            if ui
                                .add(
                                    Button::new("D")
                                        .small()
                                        .fill(ui.style().visuals.selection.bg_fill),
                                )
                                .clicked()
                            {
                                synth.noteon_routine = 0;
                            }
                        } else {
                            if ui.add(Button::new("D").small()).clicked() {
                                synth.noteon_routine = i + 1;
                            }
                        }
                        if synth.noteoff_routine == i + 1 {
                            if ui
                                .add(
                                    Button::new("U")
                                        .small()
                                        .fill(ui.style().visuals.selection.bg_fill),
                                )
                                .clicked()
                            {
                                synth.noteoff_routine = 0;
                            }
                        } else {
                            if ui.add(Button::new("U").small()).clicked() {
                                synth.noteoff_routine = i + 1;
                            }
                        }
                    });
                }

                if let Some(i) = duplicate_routine {
                    synth.routines.push(synth.routines[i].clone());
                }
                if let Some(i) = remove_routine {
                    synth.routines.remove(i);
                }
                if let Some((i, j)) = merge {
                    let mut merged = synth.routines[i].clone();
                    merged.merge(&synth.routines[j]);
                    synth.routines.push(merged);
                }

                ui.menu_button("New routine", |ui| {
                    for r in preset() {
                        if ui.button(&r.name).clicked() {
                            synth.routines.push(r.clone());
                            ui.close_menu();
                        }
                    }
                });
            }
        });
    if let Some(i) = preview_routine {
        synth.trigger_routine(i);
    }
}

fn add_event_ui<'a>(synth: &'a mut Synth, index: usize) -> impl FnMut(&mut egui::Ui) + 'a {
    move |ui| {
        if ui.button("Tongue").clicked() {
            synth.routines[index].events.push((
                0.0,
                Event::Tongue {
                    index: TongueIndex::Index(0),
                    speed: None,
                },
            ));
            ui.close_menu();
        }
        if ui.button("Constriction").clicked() {
            synth.routines[index].events.push((
                0.0,
                Event::Constriction {
                    i: 0,
                    strength: Some(1.0),
                },
            ));
            ui.close_menu();
        }
        if ui.button("Velum").clicked() {
            synth.routines[index]
                .events
                .push((0.0, Event::Velum { openness: 1.0 }));
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
    }
}

fn event_ui(ev: &mut Event, ui: &mut egui::Ui, tongue_poses: usize, other_constrictions: usize) {
    match ev {
        Event::Tongue { index, speed } => {
            ui.horizontal(|ui| {
                ComboBox::from_id_source("tongue")
                    .selected_text(index.name())
                    .show_ui(ui, |ui| {
                        for j in (0..tongue_poses)
                            .map(TongueIndex::Index)
                            .chain([TongueIndex::Random])
                        {
                            ui.selectable_value(index, j, j.name());
                        }
                    });

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
                ComboBox::from_id_source("other_constriction")
                    .selected_text(format!("Constriction {}", *i))
                    .show_ui(ui, |ui| {
                        for j in 0..other_constrictions {
                            ui.selectable_value(i, j, format!("Constriction {}", j));
                        }
                    });

                if let Some(value) = strength {
                    ui.add(knob(0.0..1.0, value, "Strength"))
                        .context_menu(|ui| {
                            if ui.button("Release").clicked() {
                                *strength = None;
                                ui.close_menu();
                            }
                        });
                }
                if strength.is_none() {
                    if ui.button("Set").clicked() {
                        *strength = Some(1.0);
                    }
                }
            });
        }
        Event::Velum { openness } => {
            ui.add(knob(0.0..1.0, openness, "Openness"));
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

#[allow(dead_code)]
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

use std::sync::OnceLock;

static PRESET: OnceLock<Vec<Routine>> = OnceLock::new();

fn preset() -> &'static [Routine] {
    PRESET.get_or_init(|| {
        vec![
            Routine {
                name: "Empty".to_string(),
                events: vec![],
            },
            Routine {
                name: "Tongue move".to_string(),
                events: vec![
                    (
                        0.0,
                        Event::Tongue {
                            index: TongueIndex::Index(0),
                            speed: Some(200.0),
                        },
                    ),
                    (
                        0.1,
                        Event::Tongue {
                            index: TongueIndex::Index(2),
                            speed: Some(20.0),
                        },
                    ),
                ],
            },
            Routine {
                name: "Tap".to_string(),
                events: vec![
                    (0.0, Event::Sound { sound: false }),
                    (
                        0.0,
                        Event::Constriction {
                            i: 1,
                            strength: Some(0.7),
                        },
                    ),
                    (0.0, Event::ForceDiameter),
                    (
                        0.0,
                        Event::Constriction {
                            i: 1,
                            strength: None,
                        },
                    ),
                    (0.01, Event::Sound { sound: true }),
                ],
            },
            Routine {
                name: "Nasal".to_string(),
                events: vec![
                    (0.0, Event::Velum { openness: 1.0 }),
                    (
                        0.0,
                        Event::Constriction {
                            i: 0,
                            strength: Some(1.0),
                        },
                    ),
                    (0.2, Event::Velum { openness: 0.0 }),
                    (
                        0.0,
                        Event::Constriction {
                            i: 0,
                            strength: None,
                        },
                    ),
                ],
            },
            Routine {
                name: "Humming".to_string(),
                events: vec![
                    (0.0, Event::Velum { openness: 1.0 }),
                    (
                        0.0,
                        Event::Constriction {
                            i: 0,
                            strength: Some(1.0),
                        },
                    ),
                ],
            },
            Routine {
                name: "Humming off".to_string(),
                events: vec![
                    (0.0, Event::Velum { openness: 0.0 }),
                    (
                        0.0,
                        Event::Constriction {
                            i: 0,
                            strength: None,
                        },
                    ),
                ],
            },
            Routine {
                name: "Trill".to_string(),
                events: vec![
                    (0.0, Event::Pitch { value: 0.0 }),
                    (0.06, Event::Pitch { value: 1.0 }),
                    (0.06, Event::Pitch { value: 0.0 }),
                ],
            },
            Routine {
                name: "Random tongue".to_string(),
                events: vec![(
                    0.0,
                    Event::Tongue {
                        index: TongueIndex::Random,
                        speed: None,
                    },
                )],
            },
        ]
    })
}

impl TongueIndex {
    fn name(&self) -> String {
        match self {
            TongueIndex::Index(i) => format!("Tongue {}", i),
            TongueIndex::Random => "Random".to_owned(),
        }
    }
}
