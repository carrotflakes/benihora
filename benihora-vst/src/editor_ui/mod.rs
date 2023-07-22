mod knob;

use self::knob::{knob, knob_log, knob_param};
use crate::{MyPluginParams, Synth, FFT_PLANNER};
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
            if ui
                .add(egui::widgets::DragValue::new(&mut synth.sound_speed).clamp_range(1.0..=6.0))
                .changed()
            {
                synth.benihora = None;
            }
            ui.label("sound speed");
            if ui
                .add(egui::widgets::DragValue::new(&mut synth.seed).clamp_range(0..=100))
                .changed()
            {
                synth.benihora = None;
            }
            ui.label("seed");
            ui.checkbox(&mut synth.benihora_params.always_sound, "always");
        });
        if synth.benihora.is_some() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.add(knob_log(
                            0.1..1000.0,
                            &mut synth.benihora_params.frequency_pid.kp,
                            "frequency kp",
                        ));
                        ui.add(knob_log(
                            0.1..1000.0,
                            &mut synth.benihora_params.frequency_pid.ki,
                            "frequency ki",
                        ));
                        ui.add(knob(
                            -0.9..0.9,
                            &mut synth.benihora_params.frequency_pid.kd,
                            "frequency kd",
                        ));
                        ui.add(knob_param(&state.vibrato_amount, setter));
                        ui.add(knob_log(
                            0.1..20.0,
                            &mut synth.benihora_params.vibrato_frequency,
                            "vibrato frequency",
                        ));
                        ui.add(knob(
                            0.0..5.0,
                            &mut synth.benihora_params.wobble_amount,
                            "wobble amount",
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.add(knob_log(
                            0.1..1000.0,
                            &mut synth.benihora_params.intensity_pid.kp,
                            "intensity kp",
                        ));
                        ui.add(knob_log(
                            0.1..1000.0,
                            &mut synth.benihora_params.intensity_pid.ki,
                            "intensity ki",
                        ));
                        ui.add(knob(
                            -0.9..0.9,
                            &mut synth.benihora_params.intensity_pid.kd,
                            "intensity kd",
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.add(knob(
                            0.0..10.0,
                            &mut synth.benihora_params.aspiration_level,
                            "aspiration level",
                        ));
                        ui.add(knob(
                            0.0..1.0,
                            &mut synth.benihora.as_mut().unwrap().tenseness.target_tenseness,
                            "tensness",
                        ));
                        ui.add(knob(
                            0.0..1.0,
                            &mut synth.benihora.as_mut().unwrap().loudness.target,
                            "loudness",
                        ));
                        if ui.button("reset freq").clicked() {
                            synth.benihora.as_mut().unwrap().frequency.set(440.0, true);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::widgets::DragValue::new(&mut synth.default_routine)
                                .clamp_range(0..=10),
                        );
                        ui.label("default routine");
                    });
                });

                ui.vertical(|ui| {
                    let view_id = ui.id().with("view");
                    let view_mode = ui
                        .data()
                        .get_persisted::<usize>(view_id)
                        .unwrap_or_default();
                    match view_mode {
                        0 => show_tract(ui, &mut synth),
                        1 => {
                            let history = &synth.benihora.as_ref().unwrap().history;
                            show_history(ui, history)
                        }
                        2 => show_waveform(
                            ui,
                            synth
                                .benihora
                                .as_ref()
                                .unwrap()
                                .waveform_recorder
                                .get_waveform(),
                        ),
                        3 => show_frequency_response(
                            ui,
                            &benihora_tract_frequency_response(
                                &synth.benihora.as_ref().unwrap().benihora.tract,
                            ),
                        ),
                        _ => unreachable!(),
                    };
                    let view_mode_name = match view_mode {
                        0 => "tract",
                        1 => "glottis graph",
                        2 => "glottis waveform",
                        3 => "frequency response",
                        _ => unreachable!(),
                    };
                    if ui.button(view_mode_name).clicked() {
                        let data = &mut ui.data();
                        let view = data.get_persisted_mut_or_default::<usize>(view_id);
                        *view = (*view + 1) % 3;
                    }
                });
            });
        }
    });
}

fn show_tract(ui: &mut egui::Ui, synth: &mut Synth) -> egui::Response {
    let Synth {
        benihora,
        tongue_poses,
        other_constrictions,
        ..
    } = synth;
    let benihora = benihora.as_mut().unwrap();

    let res = egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let tract = &benihora.benihora.tract;
        let (_id, rect) = ui.allocate_space(egui::vec2(100.0, 100.0));
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=45.0, 0.0..=10.0),
            rect,
        );

        let dx = tract.source.nose_start as f32;
        let dy = 3.75;
        let mut points = vec![];
        for (i, d) in tract.current_diameter.nose.iter().enumerate() {
            points.push(to_screen * egui::pos2(dx + i as f32, dy - *d as f32));
        }
        let stroke = egui::Stroke::new(1.0, egui::Color32::GRAY);
        ui.painter().add(egui::Shape::line(points, stroke));
        ui.painter().line_segment(
            [
                to_screen * egui::pos2(dx, dy),
                to_screen * egui::pos2(dx + (tract.current_diameter.nose.len() - 1) as f32, dy),
            ],
            stroke,
        );

        let mut points = vec![];
        for (i, d) in tract.current_diameter.mouth.iter().enumerate() {
            points.push(to_screen * egui::pos2(i as f32, (*d + 4.0) as f32));
        }
        let stroke = egui::Stroke::new(1.0, egui::Color32::GRAY);
        ui.painter().add(egui::Shape::line(points, stroke));
        ui.painter().line_segment(
            [
                to_screen * egui::pos2(0.0, 4.0),
                to_screen * egui::pos2((tract.current_diameter.mouth.len() - 1) as f32, 4.0),
            ],
            stroke,
        );

        for pos in tongue_poses {
            ui.painter().circle_filled(
                to_screen * egui::pos2(pos.0 as f32, (pos.1 as f32) + 4.0),
                1.6,
                egui::Color32::RED.linear_multiply(0.25),
            );
        }
        for oc in other_constrictions {
            ui.painter().circle_filled(
                to_screen * egui::pos2(oc.0 as f32, (oc.1 as f32) + 4.0),
                1.6,
                egui::Color32::YELLOW.linear_multiply(0.25),
            );
        }
        ui.painter().circle_filled(
            to_screen
                * egui::pos2(
                    tract.source.tongue.0 as f32,
                    (tract.source.tongue.1 as f32) + 4.0,
                ),
            2.0,
            egui::Color32::RED,
        );

        let response = benihora_tract_frequency_response(&tract);
        let response = &response[..response.len() / 2];
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=1.0, -1.0..=0.0),
            rect,
        );

        let points: Vec<_> = response
            .iter()
            .enumerate()
            .map(|(i, v)| to_screen * egui::pos2(i as f32 / response.len() as f32, -v))
            .collect();
        let stroke = egui::Stroke::new(1.0, egui::Color32::DARK_GRAY);
        ui.painter().add(egui::Shape::line(points, stroke));
    });

    let res = ui.allocate_rect(res.response.rect, egui::Sense::click_and_drag());
    if res.dragged() {
        if let Some(pos) = res.interact_pointer_pos() {
            let from_screen = egui::emath::RectTransform::from_to(
                res.rect,
                egui::Rect::from_x_y_ranges(0.0..=45.0, 0.0..=10.0),
            );
            let pos = from_screen * pos;
            benihora.tract.tongue_target = benihora
                .benihora
                .tract
                .source
                .tongue_clamp(pos.x as f64, (pos.y - 4.0) as f64);
        }
    }
    res
}

fn show_history(ui: &mut egui::Ui, history: &Vec<[f32; 5]>) -> egui::Response {
    let res = egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (_id, rect) = ui.allocate_space(egui::vec2(100.0, 100.0));
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=1.0, -1.1..=0.2),
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
        let (_id, rect) = ui.allocate_space(egui::vec2(100.0, 100.0));
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
        let (_id, rect) = ui.allocate_space(egui::vec2(100.0, 100.0));
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
