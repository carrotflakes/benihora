use super::benihora_tract_frequency_response;

use crate::synth::Synth;

use nih_plug_egui::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Tongue,
    Constriction(usize),
    Velum,
}

pub fn show_tract(ui: &mut egui::Ui, synth: &mut Synth) -> egui::Response {
    let Synth {
        benihora,
        tongue_poses,
        other_constrictions,
        ..
    } = synth;
    let benihora = benihora.as_mut().unwrap();

    let pointer = ui.input().pointer.hover_pos();
    let mut hover = None;

    let res = egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let tract = &benihora.benihora.tract;
        let (_id, rect) = ui.allocate_space(egui::vec2(140.0, 140.0));
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=45.0, 0.0..=10.0),
            rect,
        );

        // nose
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

        // tongue
        for pos in tongue_poses {
            ui.painter().circle_filled(
                to_screen * egui::pos2(pos.0 as f32, (pos.1 as f32) + 4.0),
                1.6,
                egui::Color32::RED.linear_multiply(0.25),
            );
        }

        // constriction
        for (i, &oc) in other_constrictions.iter().enumerate() {
            let pos = to_screen * egui::pos2(oc.0 as f32, (oc.1 as f32) + 4.0);
            if pointer.map(|p| (p - pos).length() < 5.0).unwrap_or(false) {
                hover = Some(Part::Constriction(i));
                ui.painter()
                    .circle_filled(pos, 5.0, egui::Color32::YELLOW.linear_multiply(0.1));
                ui.painter().circle_filled(pos, 1.6, egui::Color32::YELLOW);
            } else {
                ui.painter()
                    .circle_filled(pos, 1.6, egui::Color32::YELLOW.linear_multiply(0.25));
            }
        }

        // current tongue
        ui.painter().circle_filled(
            to_screen
                * egui::pos2(
                    tract.source.tongue.0 as f32,
                    (tract.source.tongue.1 as f32) + 4.0,
                ),
            2.0,
            egui::Color32::RED,
        );

        // frequency response
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
    let from_screen = egui::emath::RectTransform::from_to(
        res.rect,
        egui::Rect::from_x_y_ranges(0.0..=45.0, 0.0..=10.0),
    );
    let drag_mode_id = ui.make_persistent_id("tract_drag");
    if res.drag_started() {
        let drag_mode = hover.unwrap_or(Part::Tongue);
        ui.data().insert_temp(drag_mode_id, Some(drag_mode));
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
        let drag_mode = ui
            .data()
            .get_temp::<Option<Part>>(drag_mode_id)
            .unwrap_or(None);
        match drag_mode {
            Some(Part::Constriction(i)) => {
                benihora.benihora.tract.source.other_constrictions[i].1 = 10.0;
            }
            _ => (),
        }
    }
    if res.dragged() {
        let drag_mode = ui
            .data()
            .get_temp::<Option<Part>>(drag_mode_id)
            .unwrap_or(None);
        match drag_mode {
            Some(Part::Tongue) => {
                if let Some(pos) = pointer {
                    let pos = from_screen * pos;
                    benihora.tract.tongue_target = benihora
                        .benihora
                        .tract
                        .source
                        .tongue_clamp(pos.x as f64, (pos.y - 4.0) as f64);
                }
            }
            Some(Part::Constriction(ci)) => {
                if let Some(pos) = pointer {
                    let pos = from_screen * pos;
                    benihora.benihora.tract.source.other_constrictions[ci].1 = (pos.y - 4.0) as f64;
                }
            }
            _ => {}
        }
    }
    res
}
