use crate::{
    EditorState, InsertedImage, NEW_IMAGE_MAX_SIDE, QuipIt, commands, initial_shape_side, insert,
    objects, shapes, text,
};
use eframe::egui::{self, Color32};
use image::RgbaImage;

impl QuipIt {
    pub(crate) fn refresh(&mut self, ctx: &egui::Context) {
        self.background_selected = false;
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("canvas_text_editor")));
        self.palette.picking = false;
        self.inserted = None;
        self.active_shape = None;
        self.shape_render_key = None;
        self.curve_edit = false;
        self.canvas_rect = None;
        self.active_text = None;
        self.object_before = None;
        self.text_render_size = None;
        self.active_name = None;
        self.selection = None;
        self.drag_start = None;
        self.crop_mode = false;
        self.show_resize = false;
        self.upload_canvas(ctx);
    }

    pub(crate) fn composed_canvas(&self) -> Option<RgbaImage> {
        let mut pixels = self.pixels.clone()?;
        let active_position = self
            .object_before
            .as_ref()
            .map_or(self.objects.len(), |(i, _)| *i);
        for index in 0..=self.objects.len() {
            if index == active_position
                && let Some(active) = &self.inserted
            {
                active.composite_into(&mut pixels);
            }
            if let Some(object) = self.objects.get(index)
                && object.visible
            {
                object.object.composite_into(&mut pixels);
            }
        }
        Some(pixels)
    }

    pub(crate) fn upload_canvas(&mut self, ctx: &egui::Context) {
        if let Some(mut pixels) = self.pixels.clone() {
            let end = self
                .object_before
                .as_ref()
                .map_or(self.objects.len(), |(i, _)| *i);
            for object in self.objects.iter().take(end) {
                if object.visible {
                    object.object.composite_into(&mut pixels);
                }
            }
            self.width = pixels.width();
            self.height = pixels.height();
            self.texture = Some(ctx.load_texture(
                "canvas",
                egui::ColorImage::from_rgba_unmultiplied(
                    [self.width as usize, self.height as usize],
                    pixels.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            ));
        }
        self.upload_pending = false;
    }

    pub(crate) fn do_new_image(&mut self, ctx: &egui::Context) {
        self.pixels = Some(RgbaImage::from_pixel(
            self.new_width.clamp(1, NEW_IMAGE_MAX_SIDE),
            self.new_height.clamp(1, NEW_IMAGE_MAX_SIDE),
            image::Rgba(
                self.new_background
                    .unwrap_or(Color32::WHITE)
                    .to_srgba_unmultiplied(),
            ),
        ));
        self.objects.clear();
        self.path = None;
        self.project_path = None;
        self.undo.clear();
        self.redo.clear();
        self.dirty = true;
        self.lock_ratio = true;
        self.show_new = false;
        self.refresh(ctx);
        self.status = "New image".into();
    }

    pub(crate) fn do_place_text(&mut self, kind: text::Kind, ctx: &egui::Context) {
        if self.pixels.is_none() {
            return;
        }
        self.do_commit_insert(ctx);
        let canvas = self.pixels.as_ref().unwrap();
        let side = initial_shape_side(canvas.width(), canvas.height()) as f32;
        let width = (side * 1.5).min(canvas.width() as f32).max(1.0) as u32;
        let height = (side * 0.95).min(canvas.height() as f32).max(1.0) as u32;
        let mut data = text::TextData::new(kind, (side / 8.0).clamp(12.0, 512.0));
        if let Some(settings) = &self.last_font_settings {
            settings.apply(&mut data);
        }
        if kind != text::Kind::Area {
            data.border_width = self.last_shape_border_width.unwrap_or(3.0);
        }
        let (pixels, layout) = text::render(&data, width, height);
        let mut object = InsertedImage::new(
            pixels,
            egui::vec2(canvas.width() as f32, canvas.height() as f32),
            ctx,
        );
        object.image_style = Some(insert::ImageStyle::default());
        self.inserted = Some(object);
        self.active_text = Some(data);
        self.object_before = None;
        self.text_render_size = Some([width, height]);
        self.text_dirty = false;
        self.text_overflow = layout.overflow;
        self.active_shape = None;
        self.curve_edit = false;
        self.shape_rotation = 0.0;
        self.selection = None;
        self.crop_mode = false;
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("canvas_text_editor")));
        self.status="Type on the canvas. Ctrl+Enter inserts a line break. Click away to accept; Escape cancels.".into();
    }

    pub(crate) fn do_edit_object(&mut self, index: usize, ctx: &egui::Context) {
        // Committing may insert the previously selected object before this index.
        let index = index
            + usize::from(
                self.object_before
                    .as_ref()
                    .is_some_and(|(i, _)| *i <= index),
            );
        self.do_commit_insert(ctx);
        let mut original = self.objects.remove(index);
        original.visible = true;
        self.active_text = original.data.clone();
        self.inserted = Some(original.object.clone());
        self.inserted.as_mut().unwrap().restore_texture(ctx);
        self.active_shape = original.shape;
        self.curve_edit = original.curve_edit;
        self.shape_rotation = original.rotation;
        self.object_before = Some((index, original));
        self.text_render_size = None;
        self.text_dirty = true;
        self.upload_canvas(ctx);
        self.update_shape();
        if self.active_text.is_some() {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("canvas_text_editor")));
        }
        self.status =
            "Editing object. Apply keeps changes; Escape restores the previous version.".into();
    }

    pub(crate) fn do_place_shape(&mut self, kind: shapes::ShapeKind, ctx: &egui::Context) {
        if self.pixels.is_none() {
            return;
        }
        self.do_commit_insert(ctx);
        let canvas = self.pixels.as_ref().unwrap();
        let side = initial_shape_side(canvas.width(), canvas.height());
        let style = shapes::Style {
            border_width: self
                .last_shape_border_width
                .unwrap_or(shapes::Style::default().border_width),
            ..Default::default()
        };
        let pixels = shapes::render(kind, style, side, side);
        self.inserted = Some(InsertedImage::new(
            pixels,
            egui::vec2(canvas.width() as f32, canvas.height() as f32),
            ctx,
        ));
        self.active_shape = Some((kind, style));
        self.shape_rotation = 0.0;
        self.curve_edit = matches!(kind, shapes::ShapeKind::Curve4 | shapes::ShapeKind::Curve5);
        self.shape_render_key = Some((style, [side, side]));
        self.crop_mode = false;
        self.selection = None;
        self.show_resize = false;
        self.status = format!(
            "{}: move or resize with handles; adjust its properties on the right. Enter applies; Escape cancels.",
            kind.name()
        );
        self.update_shape();
    }

    pub(crate) fn update_shape(&mut self) {
        if let Some(data) = &self.active_text {
            self.last_font_settings = Some(text::FontSettings::from_text(data));
        }
        if self.curve_edit
            && let (Some((kind, style)), Some(inserted)) =
                (&mut self.active_shape, &mut self.inserted)
        {
            let initializing = style.curve_points.is_none();
            shapes::enable_curve_edit(*kind, style, &mut inserted.rect);
            if initializing && let Some(canvas) = &self.pixels {
                let available = egui::vec2(canvas.width() as f32, canvas.height() as f32);
                let factor = (available.x * 0.85 / inserted.rect.width())
                    .min(available.y * 0.85 / inserted.rect.height())
                    .min(1.0);
                let size = inserted.rect.size() * factor;
                let center = egui::pos2(
                    inserted
                        .rect
                        .center()
                        .x
                        .clamp(size.x / 2.0, available.x - size.x / 2.0),
                    inserted
                        .rect
                        .center()
                        .y
                        .clamp(size.y / 2.0, available.y - size.y / 2.0),
                );
                inserted.rect = egui::Rect::from_center_size(center, size);
            }
        }
        if let (Some((kind, style)), Some(inserted)) = (self.active_shape, &mut self.inserted) {
            inserted.image_style.get_or_insert_default();
            self.last_shape_border_width = Some(style.border_width);
            let size = [
                inserted.rect.width().round().max(1.0) as u32,
                inserted.rect.height().round().max(1.0) as u32,
            ];
            if self.shape_render_key != Some((style, size)) {
                inserted.update_pixels(shapes::render(kind, style, size[0], size[1]));
                self.shape_render_key = Some((style, size));
            }
        }
        if let (Some(data), Some(object)) = (&self.active_text, &mut self.inserted) {
            if let Some(style) = &mut object.image_style {
                style.shadow_container = Some(text::Container::from_text(data));
            }
            let size = object.content_size();
            if self.text_dirty || self.text_render_size != Some(size) {
                let (pixels, layout) = text::render(data, size[0], size[1]);
                object.update_pixels(pixels);
                self.text_overflow = layout.overflow;
                self.text_render_size = Some(size);
                self.text_dirty = false;
            }
        }
        if let Some(inserted) = &mut self.inserted {
            inserted.prepare();
        }
    }

    pub(crate) fn do_commit_insert(&mut self, ctx: &egui::Context) {
        self.background_selected = false;
        self.update_shape();
        if self.inserted.is_none() {
            return;
        }
        self.checkpoint();
        let object = self.inserted.take().unwrap();
        let data = self.active_text.take();
        let shape = self.active_shape.take();
        let active_name = self.active_name.take();
        let (position, name, visible) = self.object_before.take().map_or_else(
            || {
                let kind = data
                    .as_ref()
                    .map(|d| d.kind.name())
                    .or_else(|| shape.map(|(kind, _)| kind.name()))
                    .unwrap_or("Image");
                (
                    self.objects.len(),
                    format!("{} {}", kind, self.objects.len() + 1),
                    true,
                )
            },
            |(index, original)| {
                (
                    index.min(self.objects.len()),
                    original.name,
                    original.visible,
                )
            },
        );
        self.objects.insert(
            position,
            objects::Object {
                name: active_name.unwrap_or(name),
                visible,
                data,
                shape,
                object,
                rotation: self.shape_rotation,
                curve_edit: self.curve_edit,
            },
        );
        self.dirty = true;
        self.refresh(ctx);
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("canvas_text_editor")));
        self.status =
            "Object accepted. Select it on the canvas or in Objects to edit again.".into();
    }

    pub(crate) fn do_select_background(&mut self, ctx: &egui::Context) {
        if self.pixels.is_none() {
            return;
        }
        self.do_commit_insert(ctx);
        self.background_selected = true;
        self.crop_mode = false;
        self.selection = None;
        self.status =
            "Canvas selected. Adjust its fill color and transparency in Properties.".into();
    }

    pub(crate) fn selected_object_name(&self) -> Option<String> {
        self.inserted.as_ref()?;
        if let Some(name) = &self.active_name {
            return Some(name.clone());
        }
        Some(self.object_before.as_ref().map_or_else(
            || {
                let kind = self
                    .active_text
                    .as_ref()
                    .map(|d| d.kind.name())
                    .or_else(|| self.active_shape.map(|(kind, _)| kind.name()))
                    .unwrap_or("Image");
                format!("{} {}", kind, self.objects.len() + 1)
            },
            |(_, object)| object.name.clone(),
        ))
    }

    pub(crate) fn do_object_list_action(
        &mut self,
        index: usize,
        action: &str,
        target: usize,
        ctx: &egui::Context,
    ) {
        // List indices include the active object at its original depth.
        self.do_commit_insert(ctx);
        if index >= self.objects.len() {
            return;
        }
        self.checkpoint();
        match action {
            "visibility" => self.objects[index].visible = !self.objects[index].visible,
            "delete" => {
                self.objects.remove(index);
            }
            "duplicate" => {
                let mut copy = self.objects[index].clone();
                copy.object.id = commands::ObjectId::new();
                copy.name = format!("{} copy", copy.name);
                copy.object.rect = copy.object.rect.translate(egui::vec2(12.0, 12.0));
                copy.object.restore_texture(ctx);
                self.objects.insert(index + 1, copy);
            }
            _ => {
                let target = match action {
                    "forward" => (index + 1).min(self.objects.len() - 1),
                    "backward" => index.saturating_sub(1),
                    "front" => self.objects.len() - 1,
                    "back" => 0,
                    "reorder" => target.min(self.objects.len() - 1),
                    _ => return,
                };
                let object = self.objects.remove(index);
                self.objects.insert(target, object);
            }
        }
        self.dirty = true;
        self.refresh(ctx);
    }

    pub(crate) fn do_delete_object(&mut self, ctx: &egui::Context) {
        if self.inserted.is_none() {
            return;
        }
        self.checkpoint();
        self.object_before = None;
        self.refresh(ctx);
        self.dirty = true;
    }

    pub(crate) fn snapshot(&self) -> Option<EditorState> {
        Some(EditorState {
            pixels: self.pixels.clone()?,
            inserted: self.inserted.clone(),
            active_shape: self.active_shape.filter(|_| self.inserted.is_some()),
            rotation: self.shape_rotation,
            curve_edit: self.curve_edit,
            objects: self.objects.clone(),
            active_text: self.active_text.clone(),
            object_before: self.object_before.clone(),
            active_name: self.active_name.clone(),
        })
    }

    pub(crate) fn checkpoint(&mut self) {
        if let Some(state) = self.snapshot() {
            if self.undo.len() >= 10 {
                self.undo.remove(0);
            }
            self.undo.push(state);
        }
        self.redo.clear();
    }

    pub(crate) fn do_discard_selection(&mut self) {
        if let Some((_, style)) = self.active_shape {
            self.last_shape_border_width = Some(style.border_width);
        }
        if self.inserted.is_some() {
            self.checkpoint();
        }
        if let Some((index, original)) = self.object_before.take() {
            self.objects.insert(index.min(self.objects.len()), original);
            self.upload_pending = true;
        }
        self.active_text = None;
        self.text_render_size = None;
        self.palette.picking = false;
        self.active_name = None;
        self.inserted = None;
        self.active_shape = None;
        self.shape_render_key = None;
        self.curve_edit = false;
    }

    pub(crate) fn do_crop_to_selection(&mut self, ctx: &egui::Context) {
        let Some([x, y, width, height]) = self.selection else {
            return;
        };
        let Some(pixels) = &self.pixels else { return };
        let cropped = image::imageops::crop_imm(pixels, x, y, width, height).to_image();
        self.checkpoint();
        for text in &mut self.objects {
            text.object.rect = text.object.rect.translate(-egui::vec2(x as f32, y as f32));
        }
        self.pixels = Some(cropped);
        self.dirty = true;
        self.refresh(ctx);
        self.status = "Cropped to selection".into();
    }

    pub(crate) fn do_resize_canvas(&mut self, ctx: &egui::Context) {
        let Some(pixels) = &self.pixels else {
            return;
        };
        let scale = egui::vec2(
            self.width as f32 / pixels.width() as f32,
            self.height as f32 / pixels.height() as f32,
        );
        let resized = image::imageops::resize(
            pixels,
            self.width,
            self.height,
            image::imageops::FilterType::Lanczos3,
        );
        self.checkpoint();
        let font_scale = (scale.x * scale.y).sqrt();
        for text in &mut self.objects {
            text.object.rect = egui::Rect::from_min_size(
                egui::Pos2::ZERO + text.object.rect.min.to_vec2() * scale,
                text.object.rect.size() * scale,
            );
            if let Some(data) = &mut text.data {
                data.size *= font_scale;
                data.border_width *= font_scale;
                data.shadow_offset *= font_scale;
                data.shadow_blur *= font_scale;
                if let Some(style) = &mut text.object.image_style {
                    style.shadow_container = Some(text::Container::from_text(data));
                }
                let size = text.object.content_size();
                let (pixels, _) = text::render(data, size[0], size[1]);
                text.object.update_pixels(pixels);
            } else if let Some((kind, style)) = &mut text.shape {
                style.border_width *= font_scale;
                let size = text.object.rect.size();
                text.object.update_pixels(shapes::render(
                    *kind,
                    *style,
                    size.x.round().max(1.0) as u32,
                    size.y.round().max(1.0) as u32,
                ));
            }
            text.object.prepare();
        }
        self.pixels = Some(resized);
        self.dirty = true;
        self.refresh(ctx);
        self.status = "Image resized".into();
    }

    pub(crate) fn do_history(&mut self, redo: bool, ctx: &egui::Context) {
        let current = self.snapshot();
        let previous = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(previous) = previous {
            if let Some(current) = current {
                if redo {
                    self.undo.push(current);
                } else {
                    self.redo.push(current);
                }
            }
            self.pixels = Some(previous.pixels);
            self.objects = previous.objects;
            for object in &mut self.objects {
                object.object.restore_texture(ctx);
            }
            self.refresh(ctx);
            self.inserted = previous.inserted;
            self.active_shape = previous.active_shape;
            self.shape_rotation = previous.rotation;
            self.curve_edit = previous.curve_edit;
            self.active_text = previous.active_text;
            self.object_before = previous.object_before;
            self.active_name = previous.active_name;
            self.text_dirty = true;
            self.text_render_size = None;
            if let Some(inserted) = &mut self.inserted {
                inserted.restore_texture(ctx);
            }
            self.update_shape();
            self.upload_canvas(ctx);
            self.dirty = true;
            if self.active_shape.is_some() {
                self.status =
                    "Shape restored. Continue editing; Enter applies, Escape cancels.".into();
            }
        }
    }
}
