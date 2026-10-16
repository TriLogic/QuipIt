use eframe::egui::{self, Color32, Rect, Vec2};

pub fn selection_ui(
    ui: &mut egui::Ui,
    canvas: Rect,
    scale: f32,
    bounds: [u32; 2],
    selection: &mut [u32; 4],
) {
    let [x, y, w, h] = *selection;
    let rect = Rect::from_min_size(
        canvas.min + egui::vec2(x as f32, y as f32) * scale,
        egui::vec2(w as f32, h as f32) * scale,
    );
    let response = ui.interact(rect, ui.id().with("crop_move"), egui::Sense::drag());
    if response.dragged() {
        let delta = response.drag_delta() / scale;
        selection[0] = (x as f32 + delta.x)
            .round()
            .clamp(0.0, (bounds[0] - w) as f32) as u32;
        selection[1] = (y as f32 + delta.y)
            .round()
            .clamp(0.0, (bounds[1] - h) as f32) as u32;
    }
    response.on_hover_cursor(egui::CursorIcon::Move);
    for (index, (hx, hy)) in [
        (0, 0),
        (1, 0),
        (2, 0),
        (0, 1),
        (2, 1),
        (0, 2),
        (1, 2),
        (2, 2),
    ]
    .into_iter()
    .enumerate()
    {
        let center = rect.min + rect.size() * egui::vec2(hx as f32 / 2.0, hy as f32 / 2.0);
        let handle = Rect::from_center_size(center, Vec2::splat(10.0));
        ui.painter().rect_filled(handle, 0.0, Color32::GREEN);
        ui.painter().rect_stroke(
            handle,
            0.0,
            egui::Stroke::new(1.0_f32, Color32::BLACK),
            egui::StrokeKind::Inside,
        );
        let response = ui.interact(
            handle.expand(2.0),
            ui.id().with(("crop_handle", index)),
            egui::Sense::drag(),
        );
        let current = Rect::from_min_size(
            egui::pos2(x as f32, y as f32),
            egui::vec2(w as f32, h as f32),
        );
        let origin = crate::geometry::drag_origin(&response, current);
        if response.dragged() {
            let delta = response.total_drag_delta().unwrap_or_default() / scale;
            if ui.input(|i| i.modifiers.shift) {
                let bounds = Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(bounds[0] as f32, bounds[1] as f32),
                );
                let resized =
                    crate::geometry::resize_with_aspect(origin, hx, hy, delta, Some(bounds));
                let left = resized.left().round() as u32;
                let top = resized.top().round() as u32;
                *selection = [
                    left,
                    top,
                    resized.right().round() as u32 - left,
                    resized.bottom().round() as u32 - top,
                ];
            } else {
                *selection = resize_selection(
                    [
                        origin.left() as u32,
                        origin.top() as u32,
                        origin.width() as u32,
                        origin.height() as u32,
                    ],
                    bounds,
                    hx,
                    hy,
                    delta,
                );
            }
        }
        response.on_hover_cursor(match (hx, hy) {
            (1, _) => egui::CursorIcon::ResizeVertical,
            (_, 1) => egui::CursorIcon::ResizeHorizontal,
            (0, 0) | (2, 2) => egui::CursorIcon::ResizeNwSe,
            _ => egui::CursorIcon::ResizeNeSw,
        });
    }
}

fn resize_selection(
    [x, y, w, h]: [u32; 4],
    bounds: [u32; 2],
    hx: u32,
    hy: u32,
    delta: Vec2,
) -> [u32; 4] {
    let (mut left, mut top, mut right, mut bottom) = (x, y, x + w, y + h);
    if hx == 0 {
        left = (left as f32 + delta.x)
            .round()
            .clamp(0.0, (right - 1) as f32) as u32;
    }
    if hx == 2 {
        right = (right as f32 + delta.x)
            .round()
            .clamp((left + 1) as f32, bounds[0] as f32) as u32;
    }
    if hy == 0 {
        top = (top as f32 + delta.y)
            .round()
            .clamp(0.0, (bottom - 1) as f32) as u32;
    }
    if hy == 2 {
        bottom = (bottom as f32 + delta.y)
            .round()
            .clamp((top + 1) as f32, bounds[1] as f32) as u32;
    }
    [left, top, right - left, bottom - top]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_and_midpoint_handles_clamp_to_image_and_keep_opposite_edge() {
        assert_eq!(
            resize_selection([10, 10, 40, 30], [100, 80], 0, 0, egui::vec2(-30.0, -30.0)),
            [0, 0, 50, 40]
        );
        assert_eq!(
            resize_selection([10, 10, 40, 30], [100, 80], 2, 1, egui::vec2(100.0, 50.0)),
            [10, 10, 90, 30]
        );
        assert_eq!(
            resize_selection([10, 10, 40, 30], [100, 80], 0, 1, egui::vec2(100.0, 0.0)),
            [49, 10, 1, 30]
        );
    }
}
