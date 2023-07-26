mod knob;
mod routine;
mod tract;

use self::knob::{knob, knob_log, knob_param};
use crate::{synth::Control, MyPluginParams, FFT_PLANNER};
use benihora::tract::Tract;
use nih_plug::prelude::*;
use nih_plug_egui::egui;
use rustfft::num_complex::Complex32;
use std::sync::Arc;

pub(crate) fn editor_ui(
    egui_ctx: &egui::Context,
    setter: &ParamSetter<'_>,
    state: &mut Arc<MyPluginParams>,
) {
    egui::CentralPanel::default().show(egui_ctx, |ui| {
        let mut synth = state.synth.lock().unwrap();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::widgets::DragValue::new(&mut synth.sound_speed)
                                .clamp_range(1.0..=6.0),
                        )
                        .changed()
                    {
                        synth.request_reset();
                    }
                    ui.label("Sound speed");
                }).response.on_hover_text("This value determines the character of the voice.\n2: Male, 3: Female, 4~: Child");

                ui.label("Glottis");
                ui.horizontal(|ui| {
                    ui.add(knob_log(
                        0.1..100.0,
                        &mut synth.benihora_params.frequency_pid.kp,
                        "Frequency kp",
                    ));
                    ui.add(knob_log(
                        0.1..1000.0,
                        &mut synth.benihora_params.frequency_pid.ki,
                        "Frequency ki",
                    ));
                    ui.add(knob(
                        -0.9..0.9,
                        &mut synth.benihora_params.frequency_pid.kd,
                        "Frequency kd",
                    ));
                    ui.add(knob_param(&state.vibrato_amount, setter));
                    ui.add(knob_log(
                        0.1..20.0,
                        &mut synth.benihora_params.vibrato_frequency,
                        "Vibrato frequency",
                    ));
                    ui.add(knob(
                        0.0..5.0,
                        &mut synth.benihora_params.wobble_amount,
                        "Frequency wobble",
                    ));
                });
                ui.horizontal(|ui| {
                    ui.add(knob_log(
                    1.0..1000.0,
                        &mut synth.benihora_params.intensity_pid.kp,
                        "Intensity kp",
                    ));
                    ui.add(knob_log(
                        0.1..1000.0,
                        &mut synth.benihora_params.intensity_pid.ki,
                        "Intensity ki",
                    ));
                    ui.add(knob(
                        -0.9..0.9,
                        &mut synth.benihora_params.intensity_pid.kd,
                        "Intensity kd",
                    ));
                    ui.add(knob(
                        0.0..5.0,
                        &mut synth.benihora_params.tenseness_wobble_amount,
                        "Tensness wobble",
                    ))
                });
                ui.horizontal(|ui| {
                    ui.add(knob(
                        0.0..10.0,
                        &mut synth.benihora_params.aspiration_level,
                        "Aspiration level",
                    ));
                    ui.add(knob(
                        0.0..1.0,
                        &mut synth.benihora.as_mut().unwrap().tenseness.target_tenseness,
                        "Tensness",
                    ));
                    ui.add(knob(
                        0.0..1.0,
                        &mut synth.benihora.as_mut().unwrap().loudness.target,
                        "Loudness",
                    ));
                    if ui
                        .small_button("F")
                        .on_hover_text("Set frequency to 440Hz")
                        .clicked()
                    {
                        synth.benihora.as_mut().unwrap().frequency.set(440.0, true);
                    }
                });

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
                        ui.add(knob_param(&state.tongue_x, setter));
                        ui.add(knob_param(&state.tongue_y, setter));
                    }
                    crate::synth::Control::Internal => {
                        let tract = &mut synth.benihora.as_mut().unwrap().tract;
                        ui.add(knob(12.0..28.0, &mut tract.tongue_target.0, "Tongue x"));
                        ui.add(knob(2.0..4.0, &mut tract.tongue_target.1, "Tongue y"));
                        ui.add(knob_log(
                            0.1..100.0,
                            &mut synth.benihora.as_mut().unwrap().tract.speed,
                            "Tongue speed",
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
            });

            ui.vertical(|ui| {
                let view_id = ui.id().with("view");
                let view_mode = ui
                    .data()
                    .get_persisted::<usize>(view_id)
                    .unwrap_or_default();

                let view_mode_name = [
                    "Tract",
                    "Glottis plot",
                    "Glottis waveform",
                    "Routines",
                    "Frequency response",
                ][view_mode];
                if ui.link(view_mode_name).clicked() {
                    let data = &mut ui.data();
                    let view = data.get_persisted_mut_or_default::<usize>(view_id);
                    *view = (*view + 1) % 4;
                }

                match view_mode {
                    0 => {
                        tract::show_tract(ui, &mut synth);
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
                        routine::show_routines(ui, &mut synth);
                    }
                    4 => {
                        show_frequency_response(
                            ui,
                            &benihora_tract_frequency_response(
                                &synth.benihora.as_ref().unwrap().benihora.tract,
                            ),
                        );
                    }
                    _ => unreachable!(),
                };
            });
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
                let p: f64 = i as f64 / w as f64;
                let v = history[history.len() - i - 1][j] as f32;
                let v = match *ty {
                    0 => v,
                    1 => ((v / 440.0).log2() + 2.0) / 5.0,
                    _ => unreachable!(),
                };
                points.push(to_screen * egui::pos2(p as f32, -v));
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

pub fn benihora_tract_frequency_response(tract: &Tract) -> Vec<f32> {
    let res = 1024;
    let fft = FFT_PLANNER.with(|planner| planner.borrow_mut().plan_fft_forward(res));
    let buf = benihora::tract_impulse_response(res, tract);
    let mut buf = buf
        .iter()
        .map(|c| Complex32::from(*c as f32))
        .collect::<Vec<_>>();
    fft.process(&mut buf);
    let buf = buf
        .iter()
        .map(|c| c.norm())
        .skip(1)
        .take(res / 2)
        .collect::<Vec<_>>();
    let buf = buf
        .iter()
        .map(|v| v / (res as f32).sqrt())
        .collect::<Vec<_>>();
    buf
}
