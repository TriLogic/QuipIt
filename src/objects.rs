use crate::{insert::InsertedImage, shapes, text};

/// Objects are stored from back to front. The background remains a raster image.
#[derive(Clone)]
pub struct Object {
    pub name: String,
    pub visible: bool,
    pub object: InsertedImage,
    pub data: Option<text::TextData>,
    pub shape: Option<(shapes::ShapeKind, shapes::Style)>,
    pub rotation: f32,
    pub curve_edit: bool,
}

pub struct ObjectDrag(pub usize);

/// A visibility toggle drawn with an eye so it works with every bundled font.
pub fn visibility_button(ui: &mut eframe::egui::Ui, visible: bool) -> eframe::egui::Response {
    use eframe::egui::{self, Vec2};
    let response = ui.add(
        egui::Button::new("")
            .min_size(Vec2::splat(24.0))
            .selected(visible),
    );
    let center = response.rect.center();
    let stroke = ui.style().interact(&response).fg_stroke;
    let points = (0..=24)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 24.0;
            center + egui::vec2(angle.cos() * 8.0, angle.sin() * 4.5)
        })
        .collect();
    ui.painter().add(egui::Shape::line(points, stroke));
    ui.painter().circle_filled(center, 2.0, stroke.color);
    if !visible {
        ui.painter().line_segment(
            [
                center + egui::vec2(-9.0, 7.0),
                center + egui::vec2(9.0, -7.0),
            ],
            stroke,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled(),
            visible,
            "Visible",
        )
    });
    response.on_hover_text(if visible {
        "Hide object"
    } else {
        "Show object"
    })
}
