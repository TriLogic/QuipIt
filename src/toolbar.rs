use eframe::egui::{self, Pos2, Rect, Response, Stroke, Ui, Vec2};

pub const BUTTON_SIZE: f32 = 34.0;

pub fn border_size(ui: &mut Ui, value: &mut f32) -> bool {
    ui.label("Border size");
    let mut changed = ui
        .add(egui::Slider::new(value, 0.0..=64.0).show_value(false))
        .changed();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(*value > 0.0, egui::Button::new("−"))
            .on_hover_text("Decrease border size by 1 px")
            .clicked()
        {
            *value = (*value - 1.0).max(0.0);
            changed = true;
        }
        changed |= ui
            .add(
                egui::DragValue::new(value)
                    .range(0.0..=64.0)
                    .speed(1.0)
                    .suffix(" px"),
            )
            .changed();
        if ui
            .add_enabled(*value < 64.0, egui::Button::new("+"))
            .on_hover_text("Increase border size by 1 px")
            .clicked()
        {
            *value = (*value + 1.0).min(64.0);
            changed = true;
        }
    });
    changed
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
        let stroke = Stroke::new(1.6, ui.style().interact(&response).fg_stroke.color);
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
