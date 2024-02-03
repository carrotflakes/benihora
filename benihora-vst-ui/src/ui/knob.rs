use egui::{self, CursorIcon, Label};

const SIZE: f32 = 18.0;

pub fn knob_ui(
    ui: &mut egui::Ui,
    range: std::ops::Range<f32>,
    value: &mut f32,
    default_value: Option<f32>,
    name: Option<&str>,
    printer: impl Fn(f32) -> String,
) -> egui::Response {
    let desired_size = egui::vec2(SIZE, SIZE);

    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());

    response = response.on_hover_cursor(CursorIcon::ResizeVertical);

    let mut show_tip = false;

    if response.double_clicked() {
        if let Some(default_value) = default_value {
            *value = default_value;
            response.mark_changed();
        }
    }
    if response.dragged() {
        let amount = if ui.input(|i| i.modifiers.shift) {
            0.1
        } else {
            1.0
        };
        let delta = -response.drag_delta().y * amount;
        *value = (*value + delta * (range.end - range.start) / 100.0).clamp(range.start, range.end);
        response.mark_changed();
        ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
        show_tip = true;
    }
    if response.hovered() {
        show_tip = true;
    }

    response.widget_info(|| egui::WidgetInfo::drag_value(*value as f64));

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, true);
        let rect = rect.expand(visuals.expansion);
        let center = rect.center();
        let radius = 0.5 * rect.height();

        for r in [0.6, -0.1] {
            let a = std::f32::consts::TAU * r;
            let v = egui::vec2(a.cos(), -a.sin()) * (radius * 1.2);
            ui.painter().circle(
                center + v,
                1.0,
                visuals.fg_stroke.color.linear_multiply(0.1),
                egui::Stroke::NONE,
            );
        }

        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        let a = (*value - range.start) / (range.end - range.start);
        let a = std::f32::consts::TAU * (0.6 - a * 0.7);
        let v = egui::vec2(a.cos(), -a.sin()) * (radius * 0.75);
        ui.painter()
            .line_segment([center, center + v], visuals.fg_stroke);

        if show_tip {
            egui::containers::show_tooltip_for(
                &response.ctx,
                response.id.with("__tooltip"),
                &response.rect,
                |ui| {
                    ui.add(Label::new(if let Some(name) = name {
                        format!("{} {}", name, printer(*value))
                    } else {
                        format!("{}", printer(*value))
                    }))
                },
            );
        }
    }

    response
}

pub fn knob<'a>(
    range: std::ops::Range<f32>,
    value: &'a mut f32,
    name: &'a str,
    default_value: Option<f32>,
) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| {
        knob_ui(ui, range, value, default_value, Some(name), |v| {
            format!("{:.2}", v)
        })
    }
}

pub fn knob_log<'a>(
    range: std::ops::Range<f32>,
    value: &'a mut f32,
    name: &'a str,
    default_value: Option<f32>,
) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| {
        let mut v = value.log10();
        let range = range.start.log10()..range.end.log10();
        let default_value = default_value.map(|v| v.log10());
        let res = knob_ui(ui, range, &mut v, default_value, Some(name), |v| {
            format!("{:.2}", 10.0f32.powf(v))
        });
        *value = 10.0f32.powf(v);
        res
    }
}

pub trait Param {
    fn set(&mut self, value: f32);
    fn modulated_normalized_value(&self) -> f32;
    fn default_plain_value(&self) -> f32;
    fn preview_plain(&self, normalized: f32) -> f32;
    fn name(&self) -> &str;
    fn to_string(&self) -> String;
}

pub fn knob_param<'a, P: Param>(param: &'a mut P) -> impl egui::Widget + 'a {
    move |ui: &mut egui::Ui| {
        let desired_size = egui::vec2(SIZE, SIZE);

        let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());

        response = response.on_hover_cursor(CursorIcon::ResizeVertical);

        let mut show_tip = false;

        if response.double_clicked() {
            param.set(param.default_plain_value());
            response.mark_changed();
        }
        if response.dragged() {
            let amount = if ui.input(|i| i.modifiers.shift) {
                0.1
            } else {
                1.0
            };
            let delta = -response.drag_delta().y * amount;

            let value = (param.modulated_normalized_value() + delta * 0.01).clamp(0.0, 1.0);
            param.set(param.preview_plain(value));
            response.mark_changed();
            ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
            show_tip = true;
        }
        if response.hovered() {
            show_tip = true;
        }

        response.widget_info(|| {
            egui::WidgetInfo::drag_value(param.modulated_normalized_value() as f64)
        });

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact_selectable(&response, true);
            let rect = rect.expand(visuals.expansion);
            let center = rect.center();
            let radius = 0.5 * rect.height();

            for r in [0.6, -0.1] {
                let a = std::f32::consts::TAU * r;
                let v = egui::vec2(a.cos(), -a.sin()) * (radius * 1.2);
                ui.painter().circle(
                    center + v,
                    1.0,
                    visuals.fg_stroke.color.linear_multiply(0.1),
                    egui::Stroke::NONE,
                );
            }

            ui.painter()
                .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
            let a = param.modulated_normalized_value();
            let a = std::f32::consts::TAU * (0.6 - a * 0.7);
            let v = egui::vec2(a.cos(), -a.sin()) * (radius * 0.75);
            ui.painter()
                .line_segment([center, center + v], visuals.fg_stroke);

            if show_tip {
                egui::containers::show_tooltip_for(
                    &response.ctx,
                    response.id.with("__tooltip"),
                    &response.rect,
                    |ui| {
                        ui.add(Label::new(format!(
                            "{} {}",
                            param.name(),
                            param.to_string()
                        )))
                    },
                );
            }
        }
        response
    }
}
