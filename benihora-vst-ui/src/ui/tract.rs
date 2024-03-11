use crate::{synth::Synth, FFT_PLANNER};
use rustfft::num_complex::Complex32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Tongue,
    TonguePoint(usize),
    Constriction(usize),
    Velum,
    TongueArea,
}

pub fn show_tract(ui: &mut egui::Ui, synth: &mut Synth) -> egui::Response {
    let Synth {
        benihora,
        tongue_poses,
        other_constrictions,
        ..
    } = synth;
    let benihora = benihora.as_mut().unwrap();

    let tract_edit_id = egui::Id::new(TRACT_EDIT_ID);
    let tract_edit = ui.data(|d| d.get_temp::<bool>(tract_edit_id).unwrap_or_default());
    let drag_mode_id = ui.make_persistent_id("tract_drag");
    let mut drag_mode = ui.data(|d| d.get_temp::<Option<Part>>(drag_mode_id).unwrap_or_default());

    let pointer = ui.input(|i| i.pointer.hover_pos());
    let mut hover = None;

    let res = egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let tract = &benihora.benihora.tract;
        let (_id, rect) = ui.allocate_space(egui::vec2(180.0, 180.0));
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=45.0, 0.0..=10.0),
            rect,
        );

        // nose
        let dx = tract.source.nose_start as f32;
        let dy = 3.25;
        let mut points = vec![];
        for (i, d) in tract.current_diameter.nose.iter().enumerate() {
            points.push(to_screen * egui::pos2(dx + i as f32, dy - *d));
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
        {
            let pos = to_screen * egui::pos2(dx, dy);
            if pointer.map(|p| (p - pos).length() < 5.0).unwrap_or(false) {
                hover = Some(Part::Velum);
                ui.painter()
                    .circle_filled(pos, 5.0, egui::Color32::GREEN.linear_multiply(0.1));
                ui.painter().circle_filled(pos, 1.6, egui::Color32::GREEN);
            } else {
                ui.painter()
                    .circle_filled(pos, 1.6, egui::Color32::GREEN.linear_multiply(0.25));
            }
        }

        // mouth
        let dy = 3.5;
        let mut points = vec![];
        for (i, d) in tract.current_diameter.mouth.iter().enumerate() {
            points.push(to_screen * egui::pos2(i as f32, *d + dy));
        }
        let stroke = egui::Stroke::new(1.0, egui::Color32::GRAY);
        ui.painter().add(egui::Shape::line(points, stroke));
        ui.painter().line_segment(
            [
                to_screen * egui::pos2(0.0, dy),
                to_screen * egui::pos2((tract.current_diameter.mouth.len() - 1) as f32, dy),
            ],
            stroke,
        );

        let tongue_area_rect = egui::Rect::from_min_max(
            to_screen * egui::pos2(TONGUE_X_RANGE.start, TONGUE_Y_RANGE.start + dy),
            to_screen * egui::pos2(TONGUE_X_RANGE.end, TONGUE_Y_RANGE.end + dy),
        );
        if !tract_edit
            && tongue_area_rect
                .expand(2.5)
                .contains(pointer.unwrap_or_default())
        {
            hover = Some(Part::TongueArea);
        }
        if hover == Some(Part::TongueArea) || drag_mode == Some(Part::Tongue) {
            ui.painter().rect_filled(
                tongue_area_rect.expand(2.5),
                5.0,
                egui::Color32::RED.linear_multiply(0.05),
            );
        }

        // tongue
        for (i, pos) in tongue_poses.iter().enumerate() {
            let pos = to_screen * egui::pos2(pos.0, (pos.1) + dy);
            if pointer.map(|p| (p - pos).length() < 5.0).unwrap_or(false) {
                if tract_edit {
                    hover = Some(Part::TonguePoint(i));
                    ui.painter()
                        .circle_filled(pos, 5.0, egui::Color32::RED.linear_multiply(0.1));
                }

                egui::containers::show_tooltip_for(
                    ui.ctx(),
                    ui.id().with("__tooltip"),
                    &rect,
                    |ui| {
                        ui.label(format!("Tongue {}", i));
                    },
                );
            }

            ui.painter()
                .circle_filled(pos, 1.6, egui::Color32::RED.linear_multiply(0.25));
        }

        // constriction
        for (i, &oc) in other_constrictions.iter().enumerate() {
            let pos = to_screen * egui::pos2(oc.0, (oc.1) + dy);
            if pointer.map(|p| (p - pos).length() < 5.0).unwrap_or(false) {
                hover = Some(Part::Constriction(i));
                ui.painter()
                    .circle_filled(pos, 5.0, egui::Color32::YELLOW.linear_multiply(0.1));
                ui.painter().circle_filled(pos, 1.6, egui::Color32::YELLOW);

                egui::containers::show_tooltip_for(
                    ui.ctx(),
                    ui.id().with("__tooltip"),
                    &rect,
                    |ui| {
                        ui.label(format!("Constriction {}", i));
                    },
                );
            } else {
                ui.painter()
                    .circle_filled(pos, 1.6, egui::Color32::YELLOW.linear_multiply(0.25));
            }
        }

        // current tongue
        ui.painter().circle_filled(
            to_screen * egui::pos2(tract.source.tongue.0, (tract.source.tongue.1) + dy),
            2.0,
            egui::Color32::RED,
        );

        // frequency response
        let (mut response, sample_rate) = benihora_tract_frequency_response(&benihora.benihora);
        let max_frequency = 6000.0;
        response.resize(
            (response.len() as f32 * max_frequency / sample_rate) as usize,
            0.0,
        );
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
            rect,
        );

        // Draw line every 1000 Hz
        for i in 1..=5 {
            ui.painter().line_segment(
                [
                    to_screen * egui::pos2(i as f32 * 1000.0 / max_frequency, 0.85),
                    to_screen * egui::pos2(i as f32 * 1000.0 / max_frequency, 1.0),
                ],
                egui::Stroke::new(1.0, egui::Color32::DARK_GRAY.linear_multiply(0.25)),
            );
        }

        let points: Vec<_> = response
            .iter()
            .enumerate()
            .map(|(i, v)| to_screen * egui::pos2(i as f32 / response.len() as f32, 1.0 - v * 0.04))
            .collect();
        let stroke = egui::Stroke::new(1.0, egui::Color32::DARK_GRAY);
        ui.painter().add(egui::Shape::line(points, stroke));
    });

    let res = ui.allocate_rect(res.response.rect, egui::Sense::click_and_drag());
    let from_screen = egui::emath::RectTransform::from_to(
        res.rect,
        egui::Rect::from_x_y_ranges(0.0..=45.0, 0.0..=10.0),
    );
    if res.drag_started() {
        drag_mode = match hover {
            Some(Part::TongueArea) => Some(Part::Tongue),
            part => part,
        };
    }
    if res.clicked() {
        match hover {
            Some(Part::Velum) => {
                benihora.benihora.tract.set_velum_target(
                    if benihora.benihora.tract.velum_target() == 0.01 {
                        0.4
                    } else {
                        0.01
                    },
                );
            }
            _ => (),
        }
    }
    if res.drag_released() {
        match drag_mode {
            Some(Part::Constriction(i)) => {
                benihora.benihora.tract.source.other_constrictions[i].1 = 10.0;
            }
            _ => (),
        }
        drag_mode = None;
    }
    if res.dragged() {
        let dy = 3.5;
        match drag_mode {
            Some(Part::Tongue) => {
                if let Some(pos) = pointer {
                    let pos = from_screen * pos;
                    // benihora.tract.tongue_target = benihora
                    //     .benihora
                    //     .tract
                    //     .source
                    //     .tongue_clamp(pos.x as f32, (pos.y - dy) as f32);
                    benihora.tract.tongue_target = (
                        (pos.x).clamp(TONGUE_X_RANGE.start, TONGUE_X_RANGE.end),
                        (pos.y - dy).clamp(TONGUE_Y_RANGE.start, TONGUE_Y_RANGE.end),
                    );
                }
            }
            Some(Part::TonguePoint(ti)) => {
                if let Some(pos) = pointer {
                    let pos = from_screen * pos;
                    tongue_poses[ti] = (pos.x, pos.y - dy);
                }
            }
            Some(Part::Constriction(ci)) => {
                if let Some(pos) = pointer {
                    let pos = from_screen * pos;
                    if tract_edit {
                        other_constrictions[ci] = (pos.x, pos.y - dy);
                    } else {
                        let x = other_constrictions[ci].0;
                        benihora.benihora.tract.source.other_constrictions[ci] = (x, pos.y - dy);
                    }
                }
            }
            _ => {}
        }
    }
    ui.data_mut(|d| d.insert_temp(drag_mode_id, drag_mode));

    res
}

const TONGUE_X_RANGE: std::ops::Range<f32> = 12.0..28.0;
const TONGUE_Y_RANGE: std::ops::Range<f32> = 2.0..4.0;

pub const TRACT_EDIT_ID: &str = "benihora_tract_edit";

pub fn benihora_tract_frequency_response(benihora: &benihora::Benihora) -> (Vec<f32>, f32) {
    let frequency = 1000.0f32;
    let res = (frequency * 2.0).log2().ceil().exp2() as usize;
    let fft = FFT_PLANNER.with(|planner| planner.borrow_mut().plan_fft_forward(res));
    let (buf, sample_rate) = benihora::tract_impulse_response(res, benihora);
    let mut buf = buf.iter().map(|c| Complex32::from(*c)).collect::<Vec<_>>();
    fft.process(&mut buf);
    let buf = buf
        .iter()
        .map(|c| c.norm())
        .skip(1)
        .take(res / 2)
        .collect::<Vec<_>>();
    (buf, sample_rate / 2.0)
}
