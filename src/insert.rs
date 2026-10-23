use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, Vec2};
use image::RgbaImage;

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ImageStyle {
    pub shadow: bool,
    pub shadow_container: Option<crate::text::Container>,
    pub shadow_color: Color32,
    pub shadow_offset: Vec2,
    pub shadow_blur: f32,
    pub border_width: f32,
    pub border: Color32,
    pub border_transparency: f32,
    pub image_transparency: f32,
    pub(crate) transform: [f32; 4],
}
impl Default for ImageStyle {
    fn default() -> Self {
        Self {
            shadow: false,
            shadow_container: None,
            shadow_color: Color32::BLACK,
            shadow_offset: egui::vec2(6.0, 6.0),
            shadow_blur: 4.0,
            border_width: 0.0,
            border: Color32::BLACK,
            border_transparency: 0.0,
            image_transparency: 0.0,
            transform: [1.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Clone)]
pub struct InsertedImage {
    pub id: crate::commands::ObjectId,
    pub pixels: RgbaImage,
    texture: TextureHandle,
    pub rect: Rect,
    fitted_selection_bounds: Option<Rect>,
    selection_shape: Option<(crate::shapes::ShapeKind, crate::shapes::Style)>,
    pub image_style: Option<ImageStyle>,
    prepared_key: Option<(ImageStyle, [u32; 2])>,
}

impl InsertedImage {
    pub fn new(pixels: RgbaImage, canvas: Vec2, ctx: &egui::Context) -> Self {
        let size = egui::vec2(pixels.width() as f32, pixels.height() as f32);
        let scale = (canvas.x / size.x).min(canvas.y / size.y).min(1.0);
        let rect = Rect::from_center_size(egui::pos2(canvas.x / 2.0, canvas.y / 2.0), size * scale);
        let texture = ctx.load_texture(
            "insert",
            egui::ColorImage::from_rgba_unmultiplied(
                [pixels.width() as usize, pixels.height() as usize],
                pixels.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        Self {
            id: crate::commands::ObjectId::new(),
            pixels,
            texture,
            rect,
            fitted_selection_bounds: None,
            selection_shape: None,
            image_style: None,
            prepared_key: None,
        }
    }

    pub fn transform_image(&mut self, degrees: f32, horizontal: bool, vertical: bool) {
        let Some(style) = &mut self.image_style else {
            return;
        };
        let [a, b, c, d] = style.transform;
        let size = egui::vec2(a.abs() + b.abs(), c.abs() + d.abs());
        let scale = self.rect.size() / size;
        let (sin, cos) = degrees.to_radians().sin_cos();
        let fx = if horizontal { -1.0 } else { 1.0 };
        let fy = if vertical { -1.0 } else { 1.0 };
        let mut matrix = [
            fx * (cos * scale.x * a - sin * scale.y * c),
            fx * (cos * scale.x * b - sin * scale.y * d),
            fy * (sin * scale.x * a + cos * scale.y * c),
            fy * (sin * scale.x * b + cos * scale.y * d),
        ];
        let magnitude = matrix.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
        for v in &mut matrix {
            if v.abs() < 0.00001 {
                *v = 0.0;
            }
            *v /= magnitude;
        }
        style.transform = matrix;
        let size = egui::vec2(
            matrix[0].abs() + matrix[1].abs(),
            matrix[2].abs() + matrix[3].abs(),
        ) * magnitude;
        let factor = (16384.0 / size.x).min(16384.0 / size.y).min(1.0);
        self.rect = Rect::from_center_size(self.rect.center(), size * factor);
    }

    /// Keep text within 45 degrees of horizontal. The container's quarter-turn
    /// lives in the text raster; the remaining tilt transforms text and box together.
    pub(crate) fn transform_text(
        &mut self,
        data: &mut crate::text::TextData,
        degrees: f32,
        horizontal: bool,
        vertical: bool,
    ) {
        let [a, b, c, d] = self.axes();
        let [e, f, g, h] = data.container_transform;
        let x = e.abs() + f.abs();
        let y = g.abs() + h.abs();
        self.image_style.get_or_insert_default().transform = [
            a * e / x + b * g / y,
            a * f / x + b * h / y,
            c * e / x + d * g / y,
            c * f / x + d * h / y,
        ];
        self.transform_image(degrees, horizontal, vertical);
        // Flip controls also toggle the source-container flags. Compensate
        // those source-axis reflections here so the net flip uses canvas axes.
        let style = self.image_style.as_mut().unwrap();
        if horizontal {
            style.transform[0] = -style.transform[0];
            style.transform[2] = -style.transform[2];
        }
        if vertical {
            style.transform[1] = -style.transform[1];
            style.transform[3] = -style.transform[3];
        }
        let [a, b, c, d] = self.axes();
        let angle = c.atan2(a).to_degrees();
        let tilt = angle - (angle / 90.0).round() * 90.0;
        let (sin, cos) = tilt.to_radians().sin_cos();
        let snap = |v: f32| if v.abs() < 0.0001 { 0.0 } else { v };
        let inner = [
            snap(cos * a + sin * c),
            snap(cos * b + sin * d),
            snap(-sin * a + cos * c),
            snap(-sin * b + cos * d),
        ];
        data.container_transform = inner;
        let width = inner[0].abs() + inner[1].abs();
        let height = inner[2].abs() + inner[3].abs();
        self.image_style.as_mut().unwrap().transform = [
            snap(cos * width),
            snap(-sin * height),
            snap(sin * width),
            snap(cos * height),
        ];
    }

    fn axes(&self) -> [f32; 4] {
        let [a, b, c, d] = self
            .image_style
            .map_or([1.0, 0.0, 0.0, 1.0], |style| style.transform);
        let scale = self.rect.size() / egui::vec2(a.abs() + b.abs(), c.abs() + d.abs());
        [a * scale.x, b * scale.x, c * scale.y, d * scale.y]
    }
    pub fn content_size(&self) -> [u32; 2] {
        let [a, b, c, d] = self.axes();
        [
            (a.hypot(c).round() as u32).max(1),
            (b.hypot(d).round() as u32).max(1),
        ]
    }
    pub fn source_to_screen(&self, p: Pos2, canvas: Rect, scale: f32) -> Pos2 {
        let [a, b, c, d] = self.axes();
        let u = p.x / self.pixels.width() as f32;
        let v = p.y / self.pixels.height() as f32;
        canvas.min
            + (self.rect.min.to_vec2()
                + egui::vec2(
                    a * u + b * v - a.min(0.0) - b.min(0.0),
                    c * u + d * v - c.min(0.0) - d.min(0.0),
                ))
                * scale
    }
    pub fn screen_to_source(&self, p: Pos2, canvas: Rect, scale: f32) -> Pos2 {
        let [a, b, c, d] = self.axes();
        let p = (p - canvas.min) / scale - self.rect.min.to_vec2()
            + egui::vec2(a.min(0.0) + b.min(0.0), c.min(0.0) + d.min(0.0));
        let det = a * d - b * c;
        egui::pos2(
            (d * p.x - b * p.y) / det * self.pixels.width() as f32,
            (-c * p.x + a * p.y) / det * self.pixels.height() as f32,
        )
    }

    pub fn restore_texture(&mut self, ctx: &egui::Context) {
        self.texture = ctx.load_texture(
            "restored_object",
            egui::ColorImage::from_rgba_unmultiplied(
                [self.pixels.width() as usize, self.pixels.height() as usize],
                self.pixels.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        self.prepared_key = None;
        self.prepare();
    }

    pub fn texture_id(&self) -> egui::TextureId {
        self.texture.id()
    }

    pub(crate) fn set_selection_shape(
        &mut self,
        shape: (crate::shapes::ShapeKind, crate::shapes::Style),
    ) {
        if self.selection_shape != Some(shape) {
            self.selection_shape = Some(shape);
            self.prepared_key = None;
        }
    }

    pub fn prepare(&mut self) {
        let Some(style) = self.image_style else {
            return;
        };
        let size = [
            self.rect.width().round().max(1.0) as u32,
            self.rect.height().round().max(1.0) as u32,
        ];
        if self.prepared_key == Some((style, size)) {
            return;
        }
        let silhouette = style
            .shadow_container
            .map(|container| {
                container.selection_silhouette(self.pixels.width(), self.pixels.height())
            })
            .or_else(|| {
                self.selection_shape.map(|(kind, mut shape_style)| {
                    shape_style.fill = Color32::WHITE;
                    shape_style.border = Color32::WHITE;
                    shape_style.transparency = 0.0;
                    crate::shapes::render(
                        kind,
                        shape_style,
                        self.pixels.width(),
                        self.pixels.height(),
                    )
                })
            });
        self.fitted_selection_bounds = silhouette.and_then(|silhouette| {
            let mut silhouette_style = style;
            silhouette_style.shadow = false;
            silhouette_style.image_transparency = 0.0;
            silhouette_style.border_width = 0.0;
            let silhouette = render_image(&silhouette, silhouette_style, size[0], size[1]);
            let mut min = egui::pos2(size[0] as f32, size[1] as f32);
            let mut max = Pos2::ZERO;
            for (x, y, pixel) in silhouette.enumerate_pixels() {
                if pixel[3] > 0 {
                    min.x = min.x.min(x as f32);
                    min.y = min.y.min(y as f32);
                    max.x = max.x.max((x + 1) as f32);
                    max.y = max.y.max((y + 1) as f32);
                }
            }
            (max.x > min.x && max.y > min.y).then(|| {
                Rect::from_min_max(
                    Pos2::ZERO + min.to_vec2() / egui::vec2(size[0] as f32, size[1] as f32),
                    Pos2::ZERO + max.to_vec2() / egui::vec2(size[0] as f32, size[1] as f32),
                )
            })
        });
        let pixels = render_with_shadow(&self.pixels, style, size[0], size[1]);
        self.texture.set(
            egui::ColorImage::from_rgba_unmultiplied(
                [pixels.width() as usize, pixels.height() as usize],
                pixels.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        self.prepared_key = Some((style, size));
    }

    pub fn update_pixels(&mut self, pixels: RgbaImage) {
        self.prepared_key = None;
        self.texture.set(
            egui::ColorImage::from_rgba_unmultiplied(
                [pixels.width() as usize, pixels.height() as usize],
                pixels.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        self.pixels = pixels;
    }

    fn selection_rect(&self, rect: Rect) -> Rect {
        self.fitted_selection_bounds.map_or(rect, |bounds| {
            Rect::from_min_max(
                rect.min + bounds.min.to_vec2() * rect.size(),
                rect.min + bounds.max.to_vec2() * rect.size(),
            )
        })
    }

    pub fn screen_rect(&self, canvas: Rect, scale: f32) -> Rect {
        Rect::from_min_size(
            canvas.min + self.rect.min.to_vec2() * scale,
            self.rect.size() * scale,
        )
    }

    pub fn hit_test(&self, canvas: Rect, scale: f32, point: Pos2) -> bool {
        let local = self.screen_to_source(point, canvas, scale);
        local.x >= 0.0
            && local.y >= 0.0
            && local.x < self.pixels.width() as f32
            && local.y < self.pixels.height() as f32
    }

    /// Keep a generous screen-space rim available for moving text objects.
    pub fn text_edit_hit_test(
        &self,
        point: Pos2,
        canvas: Rect,
        scale: f32,
        bounds: Rect,
        alt: bool,
    ) -> bool {
        !alt && self.screen_rect(canvas, scale).shrink(12.0).contains(point)
            && bounds.contains(self.screen_to_source(point, canvas, scale))
    }

    pub fn contains_pointer(&self, canvas: Rect, scale: f32, point: Pos2) -> bool {
        self.screen_rect(canvas, scale).expand(8.0).contains(point)
    }

    pub fn preview(&self, ui: &egui::Ui, canvas: Rect, scale: f32) {
        ui.painter().with_clip_rect(canvas).image(
            self.texture.id(),
            self.screen_rect(canvas, scale)
                .expand2(self.image_style.map_or(Vec2::ZERO, shadow_padding) * scale),
            Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        canvas: Rect,
        scale: f32,
        rotation_allowed: bool,
        text_bounds: Option<Rect>,
    ) -> f32 {
        let mut rotation = 0.0;
        let rotate = rotation_allowed && ui.input(|i| i.modifiers.ctrl);
        let rect = self.selection_rect(self.screen_rect(canvas, scale));
        let move_response = ui.interact(
            rect.intersect(canvas),
            ui.id().with("insert_move"),
            egui::Sense::drag(),
        );
        let selects_text_at = |point| {
            text_bounds.is_some_and(|bounds| {
                self.text_edit_hit_test(point, canvas, scale, bounds, ui.input(|i| i.modifiers.alt))
            })
        };
        let gesture_id = ui.id().with("insert_text_selection_drag");
        if ui.input(|i| i.pointer.primary_pressed()) {
            let selecting = ui
                .input(|i| i.pointer.press_origin())
                .is_some_and(selects_text_at);
            ui.data_mut(|state| state.insert_temp(gesture_id, selecting));
        }
        let selecting_text = ui
            .data(|state| state.get_temp::<bool>(gesture_id))
            .unwrap_or(false);
        let hovering_text = ui
            .input(|i| i.pointer.hover_pos())
            .is_some_and(selects_text_at);
        if move_response.dragged() && !selecting_text {
            self.rect = self.rect.translate(move_response.drag_delta() / scale);
        }
        let text_cursor = if move_response.dragged() {
            selecting_text
        } else {
            hovering_text
        };
        move_response
            .on_hover_and_drag_cursor(if text_cursor {
                egui::CursorIcon::Text
            } else {
                egui::CursorIcon::Move
            })
            .on_hover_text(if text_bounds.is_some() {
                "Drag the edge to move. Alt+drag anywhere to move. Drag inside to select text."
            } else {
                "Drag to move."
            });
        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.5_f32, Color32::LIGHT_BLUE),
            egui::StrokeKind::Outside,
        );
        for (index, (x, y)) in [
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
            let point = rect.min + rect.size() * egui::vec2(x as f32 / 2.0, y as f32 / 2.0);
            let handle = Rect::from_center_size(point, Vec2::splat(9.0));
            ui.painter().rect_filled(handle, 0.0, Color32::GREEN);
            ui.painter().rect_stroke(
                handle,
                0.0,
                egui::Stroke::new(1.0_f32, Color32::BLACK),
                egui::StrokeKind::Inside,
            );
            let response = ui.interact(
                handle,
                ui.id().with(("insert_handle", index)),
                egui::Sense::click_and_drag(),
            );
            let total_delta = response.total_drag_delta().unwrap_or_default();
            let drag_id = response.id.with("handle_drag_origin");
            let (origin, delta_start) = ui.data_mut(|state| {
                let previous = state.get_temp::<(bool, Rect, Vec2)>(drag_id);
                let entry = if response.drag_started() {
                    (rotate, self.rect, Vec2::ZERO)
                } else if previous.is_some_and(|(mode, _, _)| mode != rotate) {
                    // Rebase when Ctrl changes during a drag so resizing starts
                    // from the current geometry rather than the pre-rotation box.
                    (rotate, self.rect, total_delta)
                } else {
                    previous.unwrap_or((rotate, self.rect, Vec2::ZERO))
                };
                state.insert_temp(drag_id, entry);
                (entry.1, entry.2)
            });
            if response.dragged_by(egui::PointerButton::Primary) && rotate {
                if let Some(current) = response.interact_pointer_pos() {
                    let previous = current - ui.input(|i| i.pointer.delta());
                    rotation += rotation_delta(rect.center(), previous, current);
                }
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let delta = (total_delta - delta_start) / scale;
                let selection_origin = self.selection_rect(origin);
                let resized = if ui.input(|i| i.modifiers.shift) {
                    crate::geometry::resize_with_aspect(
                        selection_origin,
                        x as u32,
                        y as u32,
                        delta,
                        None,
                    )
                } else {
                    resize_rect(selection_origin, x, y, delta)
                };
                let factor = resized.size() / selection_origin.size();
                self.rect = Rect::from_min_size(
                    resized.min + (origin.min - selection_origin.min) * factor,
                    origin.size() * factor,
                );
            }
            let cursor = if rotate {
                egui::CursorIcon::None
            } else {
                match (x, y) {
                    (1, _) => egui::CursorIcon::ResizeVertical,
                    (_, 1) => egui::CursorIcon::ResizeHorizontal,
                    (0, 0) | (2, 2) => egui::CursorIcon::ResizeNwSe,
                    _ => egui::CursorIcon::ResizeNeSw,
                }
            };
            if rotate && (response.hovered() || response.dragged_by(egui::PointerButton::Primary)) {
                paint_rotate_cursor(ui);
            }
            let response = response.on_hover_and_drag_cursor(cursor);
            response.on_hover_text(if rotate {
                "Ctrl+drag to rotate.
Release Ctrl to resize. Shift+drag keeps proportions."
            } else if rotation_allowed {
                "Drag to resize.
Shift+drag keeps proportions. Ctrl+drag rotates."
            } else {
                "Drag to resize.
Shift+drag keeps proportions."
            });
        }
        rotation
    }

    #[cfg(test)]
    pub fn composite(&self, canvas: &RgbaImage) -> RgbaImage {
        let mut output = canvas.clone();
        self.composite_into(&mut output);
        output
    }
    pub fn composite_into(&self, output: &mut RgbaImage) {
        let width = self.rect.width().round().max(1.0) as u32;
        let height = self.rect.height().round().max(1.0) as u32;
        let resized = if let Some(style) = self.image_style {
            render_with_shadow(&self.pixels, style, width, height)
        } else {
            image::imageops::resize(
                &self.pixels,
                width,
                height,
                image::imageops::FilterType::Lanczos3,
            )
        };
        let padding = self.image_style.map_or(Vec2::ZERO, shadow_padding);
        let left = self.rect.min.x.round() as i64 - padding.x as i64;
        let top = self.rect.min.y.round() as i64 - padding.y as i64;
        for y in top.max(0)..(top + resized.height() as i64).min(output.height() as i64) {
            for x in left.max(0)..(left + resized.width() as i64).min(output.width() as i64) {
                let src = resized.get_pixel((x - left) as u32, (y - top) as u32).0;
                let dst = output.get_pixel_mut(x as u32, y as u32);
                let sa = src[3] as u32;
                let da = dst[3] as u32;
                let alpha = sa * 255 + da * (255 - sa);
                if alpha == 0 {
                    continue;
                }
                for channel in 0..3 {
                    dst[channel] = ((src[channel] as u32 * sa * 255
                        + dst[channel] as u32 * da * (255 - sa)
                        + alpha / 2)
                        / alpha) as u8;
                }
                dst[3] = ((alpha + 127) / 255) as u8;
            }
        }
    }
}

fn shadow_padding(style: ImageStyle) -> Vec2 {
    if !style.shadow {
        return Vec2::ZERO;
    }
    let blur = style.shadow_blur.clamp(0.0, 32.0) * 3.0;
    egui::vec2(
        blur + style.shadow_offset.x.clamp(-64.0, 64.0).abs(),
        blur + style.shadow_offset.y.clamp(-64.0, 64.0).abs(),
    )
    .ceil()
}

fn render_with_shadow(source: &RgbaImage, style: ImageStyle, width: u32, height: u32) -> RgbaImage {
    let frame = render_image(source, style, width, height);
    if !style.shadow {
        return frame;
    }
    let padding = shadow_padding(style);
    let (px, py) = (padding.x as u32, padding.y as u32);
    let mut mask = image::GrayImage::new(width + 2 * px, height + 2 * py);
    let offset = style
        .shadow_offset
        .clamp(Vec2::splat(-64.0), Vec2::splat(64.0));
    let silhouette = style.shadow_container.map(|container| {
        render_image(
            &container.render(source.width(), source.height()),
            style,
            width,
            height,
        )
    });
    let shadow_source = silhouette.as_ref().unwrap_or(&frame);
    for (x, y, pixel) in shadow_source.enumerate_pixels() {
        let x = x as i64 + px as i64 + offset.x.round() as i64;
        let y = y as i64 + py as i64 + offset.y.round() as i64;
        if x >= 0 && y >= 0 && x < mask.width() as i64 && y < mask.height() as i64 {
            mask.put_pixel(x as u32, y as u32, image::Luma([pixel[3]]));
        }
    }
    let blur = style.shadow_blur.clamp(0.0, 32.0);
    let mask = if blur > 0.0 {
        image::imageops::blur(&mask, blur)
    } else {
        mask
    };
    let color = style.shadow_color.to_srgba_unmultiplied();
    let mut output = RgbaImage::from_fn(mask.width(), mask.height(), |x, y| {
        image::Rgba([
            color[0],
            color[1],
            color[2],
            ((mask.get_pixel(x, y)[0] as u32 * color[3] as u32 + 127) / 255) as u8,
        ])
    });
    image::imageops::overlay(&mut output, &frame, px as i64, py as i64);
    output
}

pub(crate) fn render_image(
    source: &RgbaImage,
    style: ImageStyle,
    width: u32,
    height: u32,
) -> RgbaImage {
    if style.transform == [1.0, 0.0, 0.0, 1.0] {
        return render_frame(source, style, width, height);
    }
    let [a, b, c, d] = style.transform;
    let bounds = egui::vec2(a.abs() + b.abs(), c.abs() + d.abs());
    let scale = egui::vec2(width as f32, height as f32) / bounds;
    let (a, b, c, d) = (a * scale.x, b * scale.x, c * scale.y, d * scale.y);
    let base_width = (a.hypot(c).round() as u32).max(1);
    let base_height = (b.hypot(d).round() as u32).max(1);
    let frame = render_frame(source, style, base_width, base_height);
    let mut data = frame.into_raw();
    for pixel in data.as_chunks_mut::<4>().0 {
        for i in 0..3 {
            pixel[i] = ((pixel[i] as u32 * pixel[3] as u32 + 127) / 255) as u8;
        }
    }
    let frame = tiny_skia::Pixmap::from_vec(
        data,
        tiny_skia::IntSize::from_wh(base_width, base_height).unwrap(),
    )
    .unwrap();
    let mut output = tiny_skia::Pixmap::new(width, height).unwrap();
    let transform = tiny_skia::Transform::from_row(
        a / base_width as f32,
        c / base_width as f32,
        b / base_height as f32,
        d / base_height as f32,
        -a.min(0.0) - b.min(0.0),
        -c.min(0.0) - d.min(0.0),
    );
    output.draw_pixmap(
        0,
        0,
        frame.as_ref(),
        &tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
        },
        transform,
        None,
    );
    RgbaImage::from_fn(width, height, |x, y| {
        let p = output.pixel(x, y).unwrap().demultiply();
        image::Rgba([p.red(), p.green(), p.blue(), p.alpha()])
    })
}

fn render_frame(source: &RgbaImage, style: ImageStyle, width: u32, height: u32) -> RgbaImage {
    let border = (style.border_width.round().max(0.0) as u32)
        .min(width.saturating_sub(1) / 2)
        .min(height.saturating_sub(1) / 2);
    let mut color = style.border.to_srgba_unmultiplied();
    color[3] = (color[3] as f32 * (1.0 - style.border_transparency / 100.0)).round() as u8;
    let mut output = RgbaImage::from_pixel(width, height, image::Rgba(color));
    let mut interior = image::imageops::resize(
        source,
        width - 2 * border,
        height - 2 * border,
        image::imageops::FilterType::Lanczos3,
    );
    for pixel in interior.pixels_mut() {
        pixel[3] = (pixel[3] as f32 * (1.0 - style.image_transparency / 100.0)).round() as u8;
    }
    // Border and image occupy separate regions: their transparencies are independent.
    image::imageops::replace(&mut output, &interior, border as i64, border as i64);
    output
}

fn paint_rotate_cursor(ui: &egui::Ui) {
    let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("rotate_cursor"),
    ));
    let center = pointer + egui::vec2(9.0, 9.0);
    let start = -std::f32::consts::PI * 0.8;
    let end = std::f32::consts::PI * 0.65;
    let arc: Vec<_> = (0..=32)
        .map(|i| {
            let angle = start + (end - start) * i as f32 / 32.0;
            center + egui::vec2(angle.cos(), angle.sin()) * 7.0
        })
        .collect();
    painter.add(egui::Shape::line(
        arc.clone(),
        egui::Stroke::new(4.0_f32, Color32::BLACK),
    ));
    painter.add(egui::Shape::line(
        arc,
        egui::Stroke::new(2.0_f32, Color32::WHITE),
    ));
    let tip = center + egui::vec2(end.cos(), end.sin()) * 7.0;
    let tangent = egui::vec2(-end.sin(), end.cos());
    let normal = egui::vec2(end.cos(), end.sin());
    painter.add(egui::Shape::convex_polygon(
        vec![
            tip + tangent * 4.0,
            tip - tangent * 3.0 + normal * 4.0,
            tip - tangent * 3.0 - normal * 4.0,
        ],
        Color32::WHITE,
        egui::Stroke::new(1.0_f32, Color32::BLACK),
    ));
}

fn rotation_delta(center: Pos2, previous: Pos2, current: Pos2) -> f32 {
    let a = previous - center;
    let b = current - center;
    if a.length_sq() < 1.0 || b.length_sq() < 1.0 {
        return 0.0;
    }
    (a.x * b.y - a.y * b.x).atan2(a.dot(b)).to_degrees()
}

fn resize_rect(mut rect: Rect, x: i32, y: i32, delta: Vec2) -> Rect {
    if x == 0 {
        rect.min.x = (rect.min.x + delta.x).clamp(rect.max.x - 16384.0, rect.max.x - 1.0);
    }
    if x == 2 {
        rect.max.x = (rect.max.x + delta.x).clamp(rect.min.x + 1.0, rect.min.x + 16384.0);
    }
    if y == 0 {
        rect.min.y = (rect.min.y + delta.y).clamp(rect.max.y - 16384.0, rect.max.y - 1.0);
    }
    if y == 2 {
        rect.max.y = (rect.max.y + delta.y).clamp(rect.min.y + 1.0, rect.min.y + 16384.0);
    }
    rect
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_shadow_blur_keeps_single_pixel_silhouette_sharp() {
        let mut source = RgbaImage::new(10, 10);
        source.put_pixel(5, 5, image::Rgba([255; 4]));
        let style = ImageStyle {
            shadow: true,
            shadow_color: Color32::from_black_alpha(128),
            shadow_offset: Vec2::splat(3.0),
            shadow_blur: 0.0,
            ..Default::default()
        };
        let rendered = render_with_shadow(&source, style, 10, 10);
        assert_eq!(rendered.get_pixel(11, 11)[3], 128);
        for (x, y) in [(10, 11), (12, 11), (11, 10), (11, 12)] {
            assert_eq!(rendered.get_pixel(x, y)[3], 0);
        }
    }

    #[test]
    fn image_shadow_extends_outside_object_without_changing_source_or_handles() {
        let ctx = egui::Context::default();
        let source = RgbaImage::from_pixel(10, 10, image::Rgba([255, 0, 0, 255]));
        let mut object = InsertedImage::new(source.clone(), Vec2::splat(40.0), &ctx);
        object.rect = Rect::from_min_size(egui::pos2(10.0, 10.0), Vec2::splat(10.0));
        object.image_style = Some(ImageStyle {
            shadow: true,
            shadow_color: Color32::from_black_alpha(128),
            shadow_offset: egui::vec2(4.0, 3.0),
            shadow_blur: 0.0,
            ..Default::default()
        });
        let rect = object.rect;
        object.prepare();
        let output = object.composite(&RgbaImage::new(40, 40));
        assert_eq!(output.get_pixel(22, 21).0, [0, 0, 0, 128]);
        assert_eq!(output.get_pixel(15, 15).0, [255, 0, 0, 255]);
        assert_eq!(output.get_pixel(9, 9)[3], 0);
        assert_eq!(object.pixels, source);
        assert_eq!(object.rect, rect);
        assert_eq!(object.texture.size(), [18, 16]);
        object.image_style.as_mut().unwrap().image_transparency = 100.0;
        assert!(
            object
                .composite(&RgbaImage::new(40, 40))
                .pixels()
                .all(|p| p[3] == 0)
        );
    }

    #[test]
    fn all_shapes_fit_selection_handles_to_their_transformed_outline() {
        let ctx = egui::Context::default();
        for kind in crate::shapes::ShapeKind::SHAPES
            .into_iter()
            .chain(crate::shapes::ShapeKind::LINES)
        {
            let mut style = crate::shapes::Style::default();
            let mut object = InsertedImage::new(
                crate::shapes::render(kind, style, 120, 80),
                Vec2::splat(400.0),
                &ctx,
            );
            object.image_style = Some(ImageStyle::default());
            for (degrees, horizontal, vertical) in [
                (30.0, false, false),
                (0.0, true, false),
                (0.0, false, true),
                (60.0, false, false),
            ] {
                crate::shapes::transform_shape(
                    kind,
                    &mut style,
                    &mut object.rect,
                    degrees,
                    horizontal,
                    vertical,
                );
                let width = object.rect.width().round().max(1.0) as u32;
                let height = object.rect.height().round().max(1.0) as u32;
                object.update_pixels(crate::shapes::render(kind, style, width, height));
                object.set_selection_shape((kind, style));
                let original_rect = object.rect;
                object.prepare();
                assert_eq!(object.rect, original_rect);
                let fitted = object.fitted_selection_bounds.unwrap();
                let mut min = egui::pos2(width as f32, height as f32);
                let mut max = Pos2::ZERO;
                for (x, y, pixel) in object.pixels.enumerate_pixels() {
                    if pixel[3] > 0 {
                        min.x = min.x.min(x as f32);
                        min.y = min.y.min(y as f32);
                        max.x = max.x.max((x + 1) as f32);
                        max.y = max.y.max((y + 1) as f32);
                    }
                }
                let size = egui::vec2(width as f32, height as f32);
                assert!(
                    (fitted.min.to_vec2() * size - min.to_vec2()).length() < 0.001,
                    "{}",
                    kind.name()
                );
                assert!(
                    (fitted.max.to_vec2() * size - max.to_vec2()).length() < 0.001,
                    "{}",
                    kind.name()
                );
            }
        }
    }

    #[test]
    fn fitted_selection_bounds_follow_rotated_and_flipped_container_outline() {
        let ctx = egui::Context::default();
        for kind in [
            crate::text::Kind::Area,
            crate::text::Kind::Speech,
            crate::text::Kind::Thought,
        ] {
            let mut data = crate::text::TextData::new(kind, 18.0);
            data.fill = Color32::TRANSPARENT;
            data.border = Color32::TRANSPARENT;
            let mut object = InsertedImage::new(
                crate::text::render(&data, 240, 160).0,
                Vec2::splat(600.0),
                &ctx,
            );
            object.image_style = Some(ImageStyle::default());
            for (degrees, horizontal, vertical) in [
                (30.0, false, false),
                (0.0, true, false),
                (0.0, false, true),
                (60.0, false, false),
            ] {
                data.flip_horizontal ^= horizontal;
                data.flip_vertical ^= vertical;
                object.transform_text(&mut data, degrees, horizontal, vertical);
                let size = object.content_size();
                object.update_pixels(crate::text::render(&data, size[0], size[1]).0);
                let container = crate::text::Container::from_text(&data);
                object.image_style.as_mut().unwrap().shadow_container = Some(container);
                let original_rect = object.rect;
                object.prepare();
                assert_eq!(object.rect, original_rect);
                let bounds = object.fitted_selection_bounds.unwrap();
                let width = object.rect.width().round() as u32;
                let height = object.rect.height().round() as u32;
                let silhouette = render_image(
                    &container.selection_silhouette(size[0], size[1]),
                    object.image_style.unwrap(),
                    width,
                    height,
                );
                let pixels = Rect::from_min_max(
                    Pos2::ZERO + bounds.min.to_vec2() * egui::vec2(width as f32, height as f32),
                    Pos2::ZERO + bounds.max.to_vec2() * egui::vec2(width as f32, height as f32),
                );
                let mut edges = [false; 4];
                for (x, y, p) in silhouette.enumerate_pixels() {
                    if p[3] > 0 {
                        assert!(
                            pixels
                                .expand(0.001)
                                .contains(egui::pos2(x as f32, y as f32))
                        );
                        edges[0] |= (x as f32 - pixels.left()).abs() < 0.01;
                        edges[1] |= ((x + 1) as f32 - pixels.right()).abs() < 0.01;
                        edges[2] |= (y as f32 - pixels.top()).abs() < 0.01;
                        edges[3] |= ((y + 1) as f32 - pixels.bottom()).abs() < 0.01;
                    }
                }
                assert!(edges.into_iter().all(|edge| edge));
            }
        }
    }

    #[test]
    fn old_project_image_styles_load_without_shadows() {
        let mut value = serde_json::to_value(ImageStyle::default()).unwrap();
        for name in ["shadow", "shadow_color", "shadow_offset", "shadow_blur"] {
            value.as_object_mut().unwrap().remove(name);
        }
        let style: ImageStyle = serde_json::from_value(value).unwrap();
        assert!(!style.shadow);
        assert_eq!(style.shadow_blur, 4.0);
    }

    #[test]
    fn text_caret_coordinate_mapping_survives_rotation_reflection_and_scale() {
        let ctx = egui::Context::default();
        let mut object = InsertedImage::new(RgbaImage::new(120, 70), Vec2::splat(400.0), &ctx);
        object.image_style = Some(ImageStyle::default());
        object.transform_image(37.0, true, false);
        let canvas = Rect::from_min_size(egui::pos2(30.0, 40.0), Vec2::splat(300.0));
        for source in [
            egui::pos2(0.0, 0.0),
            egui::pos2(60.0, 35.0),
            egui::pos2(120.0, 70.0),
        ] {
            let screen = object.source_to_screen(source, canvas, 0.65);
            let restored = object.screen_to_source(screen, canvas, 0.65);
            assert!((restored - source).length() < 0.001);
        }
    }

    #[test]
    fn image_rotation_and_reflection_use_original_pixels_and_preserve_opacity() {
        let ctx = egui::Context::default();
        let source = RgbaImage::from_fn(4, 2, |x, y| {
            image::Rgba([x as u8 * 50, y as u8 * 100, 25, 255])
        });
        let mut object = InsertedImage::new(source.clone(), Vec2::splat(100.0), &ctx);
        object.image_style = Some(ImageStyle::default());
        object.transform_image(90.0, false, false);
        assert_eq!(object.rect.size(), egui::vec2(2.0, 4.0));
        let rotated = render_image(&source, object.image_style.unwrap(), 2, 4);
        assert_eq!(rotated, image::imageops::rotate90(&source));
        object.transform_image(-90.0, true, false);
        assert_eq!(
            render_image(&source, object.image_style.unwrap(), 4, 2),
            image::imageops::flip_horizontal(&source)
        );
        assert_eq!(object.pixels, source);
        object.image_style.as_mut().unwrap().image_transparency = 50.0;
        object.transform_image(35.0, false, false);
        let size = object.rect.size();
        let translucent = render_image(
            &source,
            object.image_style.unwrap(),
            size.x.round() as u32,
            size.y.round() as u32,
        );
        assert!(translucent.pixels().any(|p| p[3] > 0));
        assert!(translucent.pixels().all(|p| p[3] <= 128));
        assert!(translucent.pixels().any(|p| p[3] == 0));
    }

    #[test]
    fn image_and_border_have_independent_transparency() {
        let source = RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 255, 128]));
        let style = ImageStyle {
            border_width: 2.0,
            border: Color32::RED,
            border_transparency: 50.0,
            image_transparency: 50.0,
            ..Default::default()
        };
        let output = render_image(&source, style, 12, 10);
        assert_eq!(output.get_pixel(0, 0).0, [255, 0, 0, 128]);
        assert_eq!(output.get_pixel(6, 5).0, [0, 0, 255, 64]);
        let no_border = render_image(
            &source,
            ImageStyle {
                border_transparency: 100.0,
                ..style
            },
            12,
            10,
        );
        assert_eq!(no_border.get_pixel(0, 0)[3], 0);
        assert_eq!(no_border.get_pixel(6, 5)[3], 64);
        let no_image = render_image(
            &source,
            ImageStyle {
                image_transparency: 100.0,
                ..style
            },
            12,
            10,
        );
        assert_eq!(no_image.get_pixel(6, 5)[3], 0);
        assert_eq!(no_image.get_pixel(0, 0)[3], 128);
    }

    #[test]
    fn styled_image_resize_preserves_source_and_composite_matches_preview() {
        let ctx = egui::Context::default();
        let source = RgbaImage::from_pixel(20, 20, image::Rgba([0, 0, 255, 255]));
        let mut object = InsertedImage::new(source.clone(), Vec2::splat(100.0), &ctx);
        let style = ImageStyle {
            border_width: 2.0,
            border: Color32::RED,
            border_transparency: 50.0,
            image_transparency: 50.0,
            ..Default::default()
        };
        object.image_style = Some(style);
        object.rect = Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(30.0, 25.0));
        object.prepare();
        object.rect = Rect::from_min_size(egui::pos2(10.0, 10.0), Vec2::splat(20.0));
        object.prepare();
        assert_eq!(object.pixels, source);
        let result = object.composite(&RgbaImage::new(50, 50));
        let expected = render_image(&source, style, 20, 20);
        for (x, y, pixel) in expected.enumerate_pixels() {
            assert_eq!(result.get_pixel(x + 10, y + 10), pixel);
        }
    }

    #[test]
    fn ctrl_on_handles_rotates_and_right_click_does_not_toggle_rotation() {
        for ctrl in [false, true] {
            let ctx = egui::Context::default();
            let mut object = InsertedImage::new(RgbaImage::new(80, 80), Vec2::splat(200.0), &ctx);
            let original = object.rect;
            let canvas = Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::splat(200.0));
            let point = object.screen_rect(canvas, 1.0).min;
            let modifiers = if ctrl {
                egui::Modifiers::CTRL
            } else {
                egui::Modifiers::NONE
            };
            let frame = |object: &mut InsertedImage, events, modifiers| {
                let mut rotation = 0.0;
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0))),
                        events,
                        modifiers,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            rotation = object.ui(ui, canvas, 1.0, true, None);
                        });
                    },
                );
                (rotation, output.platform_output.cursor_icon)
            };
            frame(&mut object, vec![], modifiers);
            // Right-clicking must neither transform the box nor affect the next drag.
            for pressed in [true, false] {
                let (rotation, _) = frame(
                    &mut object,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Secondary,
                            pressed,
                            modifiers,
                        },
                    ],
                    modifiers,
                );
                assert_eq!(rotation, 0.0);
                assert_eq!(object.rect, original);
            }
            let (_, cursor) = frame(
                &mut object,
                vec![egui::Event::PointerMoved(point)],
                modifiers,
            );
            assert_eq!(
                cursor,
                if ctrl {
                    egui::CursorIcon::None
                } else {
                    egui::CursorIcon::ResizeNwSe
                }
            );
            frame(
                &mut object,
                vec![egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                }],
                modifiers,
            );
            let destination = point + egui::vec2(25.0, 0.0);
            let (rotation, _) = frame(
                &mut object,
                vec![egui::Event::PointerMoved(destination)],
                modifiers,
            );
            if ctrl {
                assert!(rotation > 0.0);
                assert_eq!(object.rect, original);
                let (rotation, _) = frame(&mut object, vec![], egui::Modifiers::NONE);
                assert_eq!(rotation, 0.0);
                assert_eq!(object.rect, original);
                let (rotation, _) = frame(
                    &mut object,
                    vec![egui::Event::PointerMoved(
                        destination + egui::vec2(5.0, 0.0),
                    )],
                    egui::Modifiers::NONE,
                );
                assert_eq!(rotation, 0.0);
                assert!((object.rect.width() - (original.width() - 5.0)).abs() < 0.001);
            } else {
                assert_eq!(rotation, 0.0);
                assert!(object.rect.width() < original.width());
            }
            frame(
                &mut object,
                vec![egui::Event::PointerButton {
                    pos: destination,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                }],
                modifiers,
            );
            let handle = object.screen_rect(canvas, 1.0).min;
            let (_, cursor) = frame(
                &mut object,
                vec![egui::Event::PointerMoved(handle)],
                egui::Modifiers::NONE,
            );
            assert_eq!(cursor, egui::CursorIcon::ResizeNwSe);
        }
    }

    #[test]
    fn rotation_drag_measures_turns_and_handles_angle_wraparound() {
        let center = Pos2::ZERO;
        assert!(
            (rotation_delta(center, egui::pos2(10.0, 0.0), egui::pos2(0.0, 10.0)) - 90.0).abs()
                < 0.001
        );
        assert!(
            (rotation_delta(center, egui::pos2(0.0, 10.0), egui::pos2(10.0, 0.0)) + 90.0).abs()
                < 0.001
        );
        let delta = rotation_delta(center, egui::pos2(-10.0, 0.1), egui::pos2(-10.0, -0.1));
        assert!(delta > 0.0 && delta < 2.0);
        assert_eq!(rotation_delta(center, center, egui::pos2(10.0, 0.0)), 0.0);
    }

    #[test]
    fn resize_handles_keep_opposite_corner_and_prevent_flipping() {
        let rect = Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(50.0, 60.0));
        let resized = resize_rect(rect, 0, 0, egui::vec2(100.0, 100.0));
        assert_eq!(resized.max, rect.max);
        assert_eq!(resized.size(), Vec2::splat(1.0));
        assert_eq!(
            resize_rect(rect, 2, 1, egui::vec2(10.0, 20.0)).size(),
            egui::vec2(50.0, 40.0)
        );
    }
}
