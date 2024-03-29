mod knob;
mod routine;
mod tract;

pub use self::knob::Param;

use self::{
    knob::{knob, knob_log, knob_param},
    tract::benihora_tract_frequency_response,
};
use crate::{
    benihora_managed::Params,
    synth::{Control, Synth},
};
use egui::{self, ScrollArea};

pub fn show<P: Param>(
    ui: &mut egui::Ui,
    synth: &mut Synth,
    vibrato_amount: &mut P,
    vibrato_rate: &mut P,
    frequency_wobble: &mut P,
    tenseness_wobble: &mut P,
    tongue_x: &mut P,
    tongue_y: &mut P,
    gain: &mut P,
) {
    let default_params = Params::default();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add(knob_param(gain));
                if ui
                    .add(
                        egui::widgets::DragValue::new(&mut synth.sound_speed)
                            .clamp_range(1.0..=6.0).speed(0.1),
                    )
                    .changed()
                {
                    synth.request_reset();
                }
                ui.label("Sound speed");
            }).response.on_hover_text("This value determines the character of the voice.\n2: Male, 3: Female, 4~: Child");
            ui.add_space(4.0);

            ui.label("Glottis");
            ui.horizontal(|ui| {
                ui.add(knob(0.0..1.0, &mut synth.benihora_params.noteon_intensity, "Intensity", Some(default_params.noteon_intensity)));
                if synth.benihora.as_mut().unwrap().intensity_pid_enabled {
                    ui.add(knob_log(
                    1.0..1000.0,
                        &mut synth.benihora_params.intensity_pid.kp,
                        "Intensity kp",
                        Some(default_params.intensity_pid.kp)
                    ));
                    ui.add(knob_log(
                        0.1..1000.0,
                        &mut synth.benihora_params.intensity_pid.ki,
                        "Intensity ki",
                        Some(default_params.intensity_pid.ki)
                    ));
                    ui.add(knob(
                        -0.9..0.9,
                        &mut synth.benihora_params.intensity_pid.kd,
                        "Intensity kd",
                        Some(default_params.intensity_pid.kd)
                    ));
                } else {
                    ui.add(knob_log(0.001..3.0, &mut synth.benihora_params.intensity_adsr[0], "Intensity Attack", Some(default_params.intensity_adsr[0])));
                    ui.add(knob_log(0.001..3.0, &mut synth.benihora_params.intensity_adsr[1], "Intensity Decay", Some(default_params.intensity_adsr[1])));
                    ui.add(knob(0.01..1.0, &mut synth.benihora_params.intensity_adsr[2], "Intensity Sustain", Some(default_params.intensity_adsr[2])));
                    ui.add(knob(0.001..3.0, &mut synth.benihora_params.intensity_adsr[3], "Intensity Release", Some(default_params.intensity_adsr[3])));
                }
            });
            ui.horizontal(|ui| {
                ui.add(knob_log(
                    1.0..100.0,
                    &mut synth.benihora_params.frequency_pid.kp,
                    "Frequency kp",
                    Some(default_params.frequency_pid.kp)
                ));
                ui.add(knob_log(
                    0.1..1000.0,
                    &mut synth.benihora_params.frequency_pid.ki,
                    "Frequency ki",
                    Some(default_params.frequency_pid.ki)
                ));
                ui.add(knob(
                    -0.9..0.9,
                    &mut synth.benihora_params.frequency_pid.kd,
                    "Frequency kd",
                    Some(default_params.frequency_pid.kd)
                ));
                ui.add(knob_param(vibrato_amount));
                ui.add(knob_param(vibrato_rate));
                ui.add(knob_param(frequency_wobble));
            });
            ui.horizontal(|ui| {
                ui.add(knob_param(tenseness_wobble));
                ui.add(knob(
                    0.0..10.0,
                    &mut synth.benihora_params.aspiration_level,
                    "Aspiration level",
                    Some(default_params.aspiration_level)
                ));
                ui.add(knob(
                    0.0..1.0,
                    &mut synth.benihora.as_mut().unwrap().tenseness.target_tenseness,
                    "Tensness",
                    None
                ));
                ui.add(knob(
                    0.0..1.0,
                    &mut synth.benihora.as_mut().unwrap().loudness.target,
                    "Loudness",
                    None
                ));
                ui.add(
                    knob(
                        0.0..0.1,
                        &mut synth.noteon_sound_delay,
                        "NoteOn sound delay",
                        Some(0.0)
                ));
                if ui
                    .small_button("F")
                    .on_hover_text("Set frequency to 440Hz")
                    .clicked()
                {
                    synth.benihora.as_mut().unwrap().frequency.set(440.0, true);
                }
            });

            ui.add(egui::widgets::Checkbox::new(
                &mut synth.benihora.as_mut().unwrap().intensity_pid_enabled,
                "Use PID intensity",
            ));

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Tongue");

                {
                    let mut b = synth.tongue_control == Control::Internal;
                    ui.checkbox(&mut b, "Control").on_hover_text("Checked: The tongue is controlled by this plugin\nUnchecked: The tongue is controlled by the host");
                    synth.tongue_control = if b { Control::Internal } else { Control::Host };
                }
            });
            ui.horizontal(|ui| match synth.tongue_control {
                crate::synth::Control::Host => {
                    ui.add(knob_param(tongue_x));
                    ui.add(knob_param(tongue_y));
                }
                crate::synth::Control::Internal => {
                    let tract = &mut synth.benihora.as_mut().unwrap().tract;
                    ui.add(knob(12.0..28.0, &mut tract.tongue_target.0, "Tongue x", None));
                    ui.add(knob(2.0..4.0, &mut tract.tongue_target.1, "Tongue y", None));
                    ui.add(knob_log(
                        0.1..100.0,
                        &mut synth.benihora.as_mut().unwrap().tract.speed,
                        "Tongue speed",
                        Some(20.0)
                    ));
                }
            });

            // ui.horizontal(|ui| {
            //     ui.add(
            //         egui::widgets::DragValue::new(&mut synth.noteon_routine)
            //             .clamp_range(0..=10),
            //     );
            //     ui.label("Noteon routine");
            // });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::widgets::DragValue::new(&mut synth.seed).clamp_range(0..=100),
                        )
                        .changed()
                    {
                        synth.request_reset();
                    }
                    ui.label("Seed");
                })
                .response
                .on_hover_text("Seed changes the wobble pattern");
                ui.checkbox(&mut synth.benihora_params.always_sound, "Always")
                    .on_hover_text("Check to always produce sound");
            });

            ui.label(egui::RichText::new(format!("Build {}", build_time::build_time_utc!("%C%m%d-%H%M%S"))).weak());
        });

        ui.vertical(|ui| {
            let view_id = ui.id().with("view");
            let view_mode = ui
                .data_mut(|d| d.get_persisted::<usize>(view_id).unwrap_or_default());

            let view_mode_name = [
                "Tract",
                "Glottis plot",
                "Glottis waveform",
                "Routines",
                "Key bindings",
                "Frequency response",
            ][view_mode];
            ui.horizontal(|ui| {
                if ui.link(view_mode_name).clicked() {
                    ui.data_mut(|d| {
                        let view = d.get_persisted_mut_or_default::<usize>(view_id);
                        *view = (*view + 1) % 5;
                    });
                }

                if view_mode == 0 {
                    let tract_edit_id = egui::Id::new(tract::TRACT_EDIT_ID);
                    let mut tract_edit = ui.data(|d| {
                        d.get_temp::<bool>(tract_edit_id)
                            .unwrap_or_default()
                    });
                    tract_edit ^= ui
                            .add(
                                egui::Button::new("Edit")
                                    .small()
                                    .fill(if tract_edit {ui.style().visuals.selection.bg_fill} else {
                                        ui.style().visuals.widgets.inactive.bg_fill
                                    }),
                            ).on_hover_text("You can move constriction points and tongue points").clicked();
                    ui.data_mut(|d| d.insert_temp(tract_edit_id, tract_edit));
                }
                if view_mode == 4 {
                    ui.label("ℹ").on_hover_text("You can control each part and\ntrigger routines from MIDI note input.\nBelow are the note numbers and\nthe corresponding actions.");
                }
            });

            match view_mode {
                0 => {
                    tract::show_tract(ui, synth);
                }
                1 => {
                    let history = &synth.benihora.as_ref().unwrap().history;
                    show_history(ui, history);
                }
                2 => {
                    show_waveform(
                        ui,
                        synth
                            .benihora
                            .as_ref()
                            .unwrap()
                            .waveform_recorder
                            .get_waveform(),
                    );
                }
                3 => {
                    ui.separator();
                    routine::show_routines(ui, synth);
                }
                4 => {
                    show_key_bindings(ui, synth);
                }
                5 => {
                    show_frequency_response(
                        ui,
                        &benihora_tract_frequency_response(
                            &synth.benihora.as_ref().unwrap().benihora,
                        ).0,
                    );
                }
                _ => unreachable!(),
            };
        });
    });
}

fn show_history(ui: &mut egui::Ui, history: &Vec<[f32; 5]>) -> egui::Response {
    let res = egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (_id, rect) = ui.allocate_space(egui::vec2(140.0, 140.0));
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=1.0, -1.2..=0.2),
            rect,
        );

        let stroke = egui::Stroke::new(1.0, egui::Color32::DARK_GRAY);
        ui.painter().line_segment(
            [
                to_screen * egui::pos2(0.0, 0.0),
                to_screen * egui::pos2(1.0, 0.0),
            ],
            stroke,
        );
        ui.painter().line_segment(
            [
                to_screen * egui::pos2(0.0, -1.0),
                to_screen * egui::pos2(1.0, -1.0),
            ],
            stroke,
        );

        let show_glottis_output_level = false;
        for (j, ty) in [1, 0, 0, 0, 0]
            .iter()
            .enumerate()
            .take(if show_glottis_output_level { 5 } else { 4 })
        {
            let mut points = vec![];
            let w = rect.width() as usize;
            for i in 0..=w {
                if i >= history.len() {
                    break;
                }
                let p = i as f32 / w as f32;
                let v = history[history.len() - i - 1][j];
                let v = match *ty {
                    0 => v,
                    1 => ((v / 440.0).log2() + 2.0) / 5.0,
                    _ => unreachable!(),
                };
                points.push(to_screen * egui::pos2(p, -v));
            }

            let color = [
                egui::Color32::from_rgb(0xff, 0x00, 0x00),
                egui::Color32::from_rgb(0xff, 0x88, 0x00),
                egui::Color32::from_rgb(0xdd, 0xff, 0x00),
                egui::Color32::from_rgb(0x00, 0xff, 0x00),
                egui::Color32::from_rgb(0x00, 0x88, 0xff),
            ][j];
            ui.painter().add(egui::Shape::line(
                points,
                egui::Stroke::new(1.0, color.linear_multiply(0.5)),
            ));
        }
    });
    ui.allocate_rect(res.response.rect, egui::Sense::click())
}

fn show_waveform(ui: &mut egui::Ui, waveform: &[f32]) -> egui::Response {
    let res = egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (_id, rect) = ui.allocate_space(egui::vec2(140.0, 140.0));
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0),
            rect,
        );

        let points: Vec<_> = waveform
            .iter()
            .enumerate()
            .map(|(i, v)| to_screen * egui::pos2(i as f32 / waveform.len() as f32, -v))
            .collect();
        let stroke = egui::Stroke::new(1.0, egui::Color32::GRAY);
        ui.painter().add(egui::Shape::line(points, stroke));
    });
    ui.allocate_rect(res.response.rect, egui::Sense::click())
}

fn show_frequency_response(ui: &mut egui::Ui, response: &[f32]) -> egui::Response {
    let res = egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (_id, rect) = ui.allocate_space(egui::vec2(140.0, 140.0));
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=1.0, -1.0..=0.0),
            rect,
        );

        let points: Vec<_> = response
            .iter()
            .enumerate()
            .map(|(i, v)| to_screen * egui::pos2(i as f32 / response.len() as f32, -v))
            .collect();
        let stroke = egui::Stroke::new(1.0, egui::Color32::GRAY);
        ui.painter().add(egui::Shape::line(points, stroke));
    });
    ui.allocate_rect(res.response.rect, egui::Sense::click())
}

fn show_key_bindings(ui: &mut egui::Ui, synth: &mut Synth) {
    ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let mut n = 0;
            for (i, _) in synth.tongue_poses.iter().enumerate() {
                ui.label(format!("{:>3} Tongue {}", n, i + 1));
                n += 1;
            }
            for (i, _) in synth.other_constrictions.iter().enumerate() {
                ui.label(format!("{:>3} Constriction {}", n, i + 1));
                n += 1;
            }
            ui.label(format!("{:>3} Velum", n));
            n += 1;
            for (i, _) in synth.routines.iter().enumerate() {
                ui.label(format!("{:>3} Routine {}", n, i + 1));
                n += 1;
            }
            ui.label("...");
        });
}
