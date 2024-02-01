use egui::{pos2, vec2, Color32, Rect, Ui};

pub fn show(ui: &mut Ui, current_note: Option<u8>, on_touch: &mut impl FnMut(bool, u8)) {
    let offset = 24;

    let res = egui::Frame::canvas(ui.style()).show(ui, |ui| {
        let (_id, rect) = ui.allocate_space(egui::vec2(480.0, 40.0));
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
            rect,
        );
        for i in 0..49 {
            let x = i as f32 / 49.0;
            let rect = Rect::from_min_size(pos2(x, 0.0), vec2(0.9 / 49.0, 1.0));
            let n = ((i as f32 + 0.45) * 12.0 / 7.0) as u8 + offset;
            let color = if &Some(n) == &current_note {
                Color32::RED
            } else {
                Color32::WHITE
            };
            ui.painter()
                .rect_filled(to_screen.transform_rect(rect), 1.0, color);
        }
        for i in 0..49 {
            if [0, 1, 3, 4, 5].contains(&(i % 7)) {
                let x = (i as f32 + 0.5) / 49.0;
                let rect = Rect::from_min_size(pos2(x, 0.0), vec2(0.9 / 49.0, 0.6));
                let n = ((i as f32 + 0.45) * 12.0 / 7.0) as u8 + offset + 1;
                let color = if &Some(n) == &current_note {
                    Color32::RED
                } else {
                    Color32::DARK_GRAY
                };
                ui.painter()
                    .rect_filled(to_screen.transform_rect(rect), 1.0, color);
            }
        }
    });

    let note_id = ui.id().with("note");
    let mut note = ui.data(|d| d.get_temp::<Option<u8>>(note_id).unwrap_or_default());

    let res = ui.allocate_rect(res.response.rect, egui::Sense::click_and_drag());
    let from_screen = egui::emath::RectTransform::from_to(
        res.rect,
        egui::Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
    );
    let pos = ui.input(|i| i.pointer.hover_pos());
    if pos.is_some() && res.dragged() {
        let x = (from_screen * pos.unwrap()).x.clamp(0.0, 0.99);
        let y = (from_screen * pos.unwrap()).y.clamp(0.0, 1.0);
        let n = if y < 0.6 && [0, 1, 3, 4, 5].contains(&((x * 49.0 - 0.5 + 7.0) as i32 % 7)) {
            ((x * 49.0 - 0.5).floor() + 0.45) * 12.0 / 7.0 + 1.0
        } else {
            ((x * 49.0).floor() + 0.45) * 12.0 / 7.0
        } as u8
            + offset;
        if note != Some(n) {
            if let Some(note) = note {
                on_touch(true, note);
            }
            on_touch(false, n);
            note = Some(n);
        }
    }
    if pos.is_none() || res.drag_released() {
        if let Some(n) = note {
            on_touch(true, n);
        }
        note = None;
    }

    ui.data_mut(|d| d.insert_temp(note_id, note));
}
