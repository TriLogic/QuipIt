use eframe::egui::{self, Pos2, Rect, Response, Stroke, Ui, Vec2};

pub const BUTTON_SIZE: f32 = 34.0;

pub fn panel_frame(fill: egui::Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, egui::Color32::from_gray(58)))
        .corner_radius(10)
        .inner_margin(8)
        .outer_margin(4)
}

pub fn section_header(ui: &mut Ui, title: &str) -> Response {
    section_header_with_reserved_space(ui, title, 0.0)
}

pub fn section_header_with_reserved_space(ui: &mut Ui, title: &str, reserved: f32) -> Response {
    // The body starts 8px inside the panel. Share that top inset with the
    // header instead of adding it above the title's own padding.
    let (allocated, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::hover());
    let rect = egui::Rect::from_min_max(
        allocated.min - egui::vec2(8.0, 8.0),
        allocated.max + egui::vec2(8.0, 0.0),
    );
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius {
            nw: 9,
            ne: 9,
            sw: 0,
            se: 0,
        },
        egui::Color32::from_gray(72),
    );
    let mut job = egui::text::LayoutJob::simple(
        title.into(),
        egui::FontId::proportional(13.0),
        egui::Color32::from_gray(240),
        (rect.width() - reserved - 16.0).max(0.0),
    );
    job.wrap.max_rows = 1;
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(
        rect.min + egui::vec2(8.0, (rect.height() - galley.size().y) * 0.5),
        galley,
        egui::Color32::from_gray(240),
    );
    Response { rect, ..response }.on_hover_text(title)
}

pub fn numeric<T: egui::emath::Numeric>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    suffix: &str,
    logarithmic: bool,
) -> bool {
    let before = *value;
    let minimum = range.start().to_f64();
    let maximum = range.end().to_f64();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let short_label = if label == "Transparency" {
            "Transp."
        } else {
            label
        };
        ui.add_sized(
            [52.0, 18.0],
            egui::Label::new(format!("{short_label}:")).wrap_mode(egui::TextWrapMode::Extend),
        )
        .on_hover_text(label);
        ui.spacing_mut().slider_width =
            (ui.available_width() - 92.0 - 3.0 * ui.spacing().item_spacing.x).clamp(32.0, 180.0);
        ui.add(
            egui::Slider::new(value, range.clone())
                .logarithmic(logarithmic)
                .show_value(false),
        );
        if ui
            .add_enabled(value.to_f64() > minimum, egui::Button::new("−"))
            .on_hover_text(format!("Decrease {label} by 1{suffix}"))
            .clicked()
        {
            *value = T::from_f64((value.to_f64() - 1.0).clamp(minimum, maximum));
        }
        ui.add_sized(
            [52.0, 18.0],
            egui::DragValue::new(value)
                .range(range)
                .speed(1.0)
                .max_decimals(if T::INTEGRAL { 0 } else { 2 })
                .suffix(suffix),
        );
        if ui
            .add_enabled(value.to_f64() < maximum, egui::Button::new("+"))
            .on_hover_text(format!("Increase {label} by 1{suffix}"))
            .clicked()
        {
            *value = T::from_f64((value.to_f64() + 1.0).clamp(minimum, maximum));
        }
    });
    *value != before
}

pub fn border_size(ui: &mut Ui, value: &mut f32) -> bool {
    numeric(ui, "Border", value, 0.0..=64.0, " px", false)
}

#[derive(Clone, Copy)]
pub enum Icon {
    New,
    Open,
    Insert,
    Save,
    Undo,
    Redo,
    Crop,
    Resize,
    Shapes,
    Text,
    Ai,
    RotateLeft,
    RotateRight,
    FlipVertical,
    FlipHorizontal,
}

pub fn button(ui: &mut Ui, icon: Icon, label: &str, enabled: bool, selected: bool) -> Response {
    ui.add_enabled_ui(enabled, |ui| {
        let response = ui.add(
            egui::Button::new("")
                .min_size(Vec2::splat(BUTTON_SIZE))
                .selected(selected),
        );
        let painter = ui.painter().with_clip_rect(response.rect);
        let origin = response.rect.center() - Vec2::splat(9.0);
        let point = |x: f32, y: f32| origin + egui::vec2(x, y);
        let stroke = Stroke::new(1.6_f32, ui.style().interact(&response).fg_stroke.color);
        let line = |a: (f32, f32), b: (f32, f32)| {
            painter.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);
        };
        let outline = |min: (f32, f32), max: (f32, f32)| {
            painter.rect_stroke(
                Rect::from_min_max(point(min.0, min.1), point(max.0, max.1)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        };
        match icon {
            Icon::Ai => {
                painter.text(
                    response.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "AI",
                    egui::FontId::proportional(17.0),
                    stroke.color,
                );
            }
            Icon::New => {
                outline((3.0, 1.0), (15.0, 17.0));
                line((6.0, 9.0), (12.0, 9.0));
                line((9.0, 6.0), (9.0, 12.0));
            }
            Icon::Open => {
                for (a, b) in [
                    ((1.0, 16.0), (1.0, 3.0)),
                    ((1.0, 3.0), (7.0, 3.0)),
                    ((7.0, 3.0), (9.0, 6.0)),
                    ((9.0, 6.0), (17.0, 6.0)),
                    ((17.0, 6.0), (17.0, 16.0)),
                    ((17.0, 16.0), (1.0, 16.0)),
                    ((1.0, 8.0), (17.0, 8.0)),
                ] {
                    line(a, b);
                }
            }
            Icon::Insert => {
                outline((1.0, 2.0), (16.0, 16.0));
                line((3.0, 13.0), (7.0, 8.0));
                line((7.0, 8.0), (10.0, 11.0));
                painter.circle_stroke(point(12.0, 6.0), 1.5, stroke);
                line((13.0, 14.0), (19.0, 14.0));
                line((16.0, 11.0), (16.0, 17.0));
            }
            Icon::Save => {
                outline((1.0, 1.0), (17.0, 17.0));
                outline((5.0, 1.0), (13.0, 7.0));
                outline((5.0, 11.0), (13.0, 17.0));
            }
            Icon::Undo | Icon::Redo => {
                let mirror = matches!(icon, Icon::Redo);
                let p = |x: f32, y: f32| -> Pos2 { point(if mirror { 18.0 - x } else { x }, y) };
                painter.add(egui::Shape::line(
                    vec![p(2.0, 7.0), p(10.0, 7.0), p(15.0, 10.0), p(15.0, 16.0)],
                    stroke,
                ));
                painter.add(egui::Shape::line(
                    vec![p(7.0, 2.0), p(2.0, 7.0), p(7.0, 12.0)],
                    stroke,
                ));
            }
            Icon::Crop => {
                for (a, b) in [
                    ((4.0, 0.0), (4.0, 14.0)),
                    ((4.0, 14.0), (18.0, 14.0)),
                    ((0.0, 4.0), (14.0, 4.0)),
                    ((14.0, 4.0), (14.0, 18.0)),
                ] {
                    line(a, b);
                }
            }
            Icon::RotateLeft | Icon::RotateRight => {
                let p = |x: f32, y: f32| {
                    point(
                        if matches!(icon, Icon::RotateRight) {
                            18.0 - x
                        } else {
                            x
                        },
                        y,
                    )
                };
                let arc: Vec<_> = (0..=28)
                    .map(|i| {
                        let angle = -std::f32::consts::FRAC_PI_2
                            + i as f32 / 28.0 * std::f32::consts::PI * 1.6;
                        p(9.0 + 7.0 * angle.cos(), 9.0 + 7.0 * angle.sin())
                    })
                    .collect();
                painter.add(egui::Shape::line(arc, stroke));
                painter.add(egui::Shape::line(
                    vec![p(12.0, 0.0), p(8.0, 2.0), p(12.0, 6.0)],
                    stroke,
                ));
            }
            Icon::FlipVertical | Icon::FlipHorizontal => {
                let p = |x: f32, y: f32| {
                    if matches!(icon, Icon::FlipVertical) {
                        point(y, x)
                    } else {
                        point(x, y)
                    }
                };
                painter.line_segment([p(9.0, 0.0), p(9.0, 18.0)], stroke);
                for points in [
                    vec![p(1.0, 3.0), p(6.0, 9.0), p(1.0, 15.0)],
                    vec![p(17.0, 3.0), p(12.0, 9.0), p(17.0, 15.0)],
                ] {
                    painter.add(egui::Shape::closed_line(points, stroke));
                }
            }
            Icon::Text => {
                line((1.0, 2.0), (17.0, 2.0));
                line((9.0, 2.0), (9.0, 17.0));
                line((5.0, 17.0), (13.0, 17.0));
            }
            Icon::Shapes => {
                painter.circle_stroke(point(4.0, 4.0), 3.5, stroke);
                outline((10.0, 1.0), (18.0, 9.0));
                painter.add(egui::Shape::closed_line(
                    vec![point(9.0, 10.0), point(3.0, 18.0), point(15.0, 18.0)],
                    stroke,
                ));
            }
            Icon::Resize => {
                outline((1.0, 7.0), (11.0, 17.0));
                line((7.0, 11.0), (17.0, 1.0));
                line((11.0, 1.0), (17.0, 1.0));
                line((17.0, 1.0), (17.0, 7.0));
            }
        }
        response.on_hover_text(label)
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numeric_frame(
        ctx: &egui::Context,
        value: &mut f32,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, bool) {
        let mut changed = false;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(230.0, 100.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    changed = numeric(ui, "X", value, -1.0..=1.0, " px", false);
                });
            },
        );
        (output, changed)
    }

    #[test]
    fn numeric_buttons_step_negative_values_and_stop_at_bounds_in_a_narrow_panel() {
        let ctx = egui::Context::default();
        let mut value = 0.0;
        numeric_frame(&ctx, &mut value, vec![]);
        let (output, changed) = numeric_frame(&ctx, &mut value, vec![]);
        assert!(!changed);
        let text_rect = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.job.text == label
                    {
                        Some(text.galley.rect.translate(text.pos.to_vec2()))
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let decrement = text_rect("−").center();
        let increment = text_rect("+").center();
        assert!(increment.x < 222.0);
        assert!((decrement.y - increment.y).abs() < 1.0);
        for (position, expected, changed_expected) in [
            (decrement, -1.0, true),
            (decrement, -1.0, false),
            (increment, 0.0, true),
            (increment, 1.0, true),
            (increment, 1.0, false),
        ] {
            numeric_frame(
                &ctx,
                &mut value,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            let (_, changed) = numeric_frame(
                &ctx,
                &mut value,
                vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(value, expected);
            assert_eq!(changed, changed_expected);
        }
    }
}
