use eframe::egui::{self, Rect, Response, Vec2};

// Capture the rectangle once per drag so modifier changes never accumulate ratio drift.
pub fn drag_origin(response: &Response, current: Rect) -> Rect {
    let id = response.id.with("resize_origin");
    response.ctx.data_mut(|data| {
        if response.drag_started() {
            data.insert_temp(id, current);
        }
        data.get_temp::<Rect>(id).unwrap_or(current)
    })
}

pub fn resize_with_aspect(rect: Rect, hx: u32, hy: u32, delta: Vec2, bounds: Option<Rect>) -> Rect {
    let size = rect.size();
    let dx = if hx == 0 { -delta.x } else { delta.x } / size.x;
    let dy = if hy == 0 { -delta.y } else { delta.y } / size.y;
    let change = if hx == 1 {
        dy
    } else if hy == 1 || dx.abs() >= dy.abs() {
        dx
    } else {
        dy
    };
    let anchor = egui::pos2(
        match hx {
            0 => rect.right(),
            2 => rect.left(),
            _ => rect.center().x,
        },
        match hy {
            0 => rect.bottom(),
            2 => rect.top(),
            _ => rect.center().y,
        },
    );
    let mut maximum = (16384.0 / size.x).min(16384.0 / size.y);
    if let Some(bounds) = bounds {
        let available_width = match hx {
            0 => anchor.x - bounds.left(),
            2 => bounds.right() - anchor.x,
            _ => 2.0 * (anchor.x - bounds.left()).min(bounds.right() - anchor.x),
        };
        let available_height = match hy {
            0 => anchor.y - bounds.top(),
            2 => bounds.bottom() - anchor.y,
            _ => 2.0 * (anchor.y - bounds.top()).min(bounds.bottom() - anchor.y),
        };
        maximum = maximum
            .min(available_width / size.x)
            .min(available_height / size.y);
    }
    let minimum = (1.0 / size.x).max(1.0 / size.y).min(maximum);
    let size = size * (1.0 + change).clamp(minimum, maximum);
    let left = match hx {
        0 => anchor.x - size.x,
        2 => anchor.x,
        _ => anchor.x - size.x / 2.0,
    };
    let top = match hy {
        0 => anchor.y - size.y,
        2 => anchor.y,
        _ => anchor.y - size.y / 2.0,
    };
    Rect::from_min_size(egui::pos2(left, top), size)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_handles_preserve_ratio_and_opposite_anchor() {
        let rect = Rect::from_min_size(egui::pos2(40.0, 40.0), egui::vec2(80.0, 40.0));
        for (x, y) in [
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ] {
            let resized = resize_with_aspect(rect, x, y, egui::vec2(25.0, 18.0), None);
            assert!((resized.width() / resized.height() - 2.0).abs() < 0.0001);
            match x {
                0 => assert_eq!(resized.right(), rect.right()),
                2 => assert_eq!(resized.left(), rect.left()),
                _ => assert_eq!(resized.center().x, rect.center().x),
            }
            match y {
                0 => assert_eq!(resized.bottom(), rect.bottom()),
                2 => assert_eq!(resized.top(), rect.top()),
                _ => assert_eq!(resized.center().y, rect.center().y),
            }
        }
    }
    #[test]
    fn constrained_crop_stays_in_bounds_and_cannot_flip() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(40.0, 20.0));
        let bounds = Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(100.0));
        for delta in [Vec2::splat(1000.0), Vec2::splat(-1000.0)] {
            let resized = resize_with_aspect(rect, 2, 1, delta, Some(bounds));
            assert!(bounds.contains_rect(resized));
            assert!(resized.width() >= 1.0 && resized.height() >= 1.0);
            assert_eq!(resized.width() / resized.height(), 2.0);
        }
    }
}
