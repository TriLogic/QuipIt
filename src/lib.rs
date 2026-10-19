use eframe::egui::{self, Color32, TextureHandle, Vec2};
use image::RgbaImage;

mod about;
mod ai;
mod ai_window;
mod colors;
pub mod commands;
mod crop;
mod geometry;
mod insert;
mod native_mouse;
mod objects;
mod project;
mod save;
pub mod shapes;
pub mod text;
mod toolbar;
use insert::InsertedImage;
use std::path::PathBuf;
use toolbar::{Icon, button as tool_button};

const NEW_IMAGE_MAX_SIDE: u32 = 5000;

fn initial_shape_side(width: u32, height: u32) -> u32 {
    // Match the familiar 240px shape on a 720px canvas at any resolution.
    ((width.min(height) as f64 / 3.0).round() as u32).max(1)
}

#[derive(Clone, Default)]
struct EditorState {
    pixels: RgbaImage,
    inserted: Option<InsertedImage>,
    active_shape: Option<(shapes::ShapeKind, shapes::Style)>,
    rotation: f32,
    curve_edit: bool,
    objects: Vec<objects::Object>,
    active_text: Option<text::TextData>,
    object_before: Option<(usize, objects::Object)>,
    active_name: Option<String>,
}

#[derive(Clone, Default)]
pub struct QuipIt {
    pixels: Option<RgbaImage>,
    texture: Option<TextureHandle>,
    path: Option<PathBuf>,
    project_path: Option<PathBuf>,
    window_title: String,
    program_icon: Option<TextureHandle>,
    show_about: bool,
    save_error: Option<String>,
    save_as_dialog: Option<save::SaveAsDialog>,
    undo: Vec<EditorState>,
    redo: Vec<EditorState>,
    width: u32,
    height: u32,
    selection: Option<[u32; 4]>,
    crop_mode: bool,
    drag_start: Option<egui::Pos2>,
    show_resize: bool,
    show_shapes: bool,
    selected_shape: Option<shapes::ShapeKind>,
    active_shape: Option<(shapes::ShapeKind, shapes::Style)>,
    shape_render_key: Option<(shapes::Style, [u32; 2])>,
    shape_rotation: f32,
    curve_edit: bool,
    last_shape_border_width: Option<f32>,
    shape_properties_rect: Option<egui::Rect>,
    palette: colors::Palette,
    lock_ratio: bool,
    dirty: bool,
    status: String,
    pending_open: bool,
    pending_open_path: Option<PathBuf>,
    dropped_image: Option<PathBuf>,
    pending_new: bool,
    show_new: bool,
    new_width: u32,
    new_height: u32,
    new_aspect_ratio: Option<f64>,
    new_background: Option<Color32>,
    background_selected: bool,
    background_before: Option<RgbaImage>,
    background_preview: Option<Color32>,
    inserted: Option<InsertedImage>,
    canvas_rect: Option<(egui::Rect, f32)>,
    confirm_close: bool,
    close_approved: bool,
    objects: Vec<objects::Object>,
    active_text: Option<text::TextData>,
    object_before: Option<(usize, objects::Object)>,
    active_name: Option<String>,
    text_dirty: bool,
    last_font_settings: Option<text::FontSettings>,
    text_render_size: Option<[u32; 2]>,
    text_overflow: bool,
    show_text_picker: bool,
    show_ai_commands: bool,
    ai_window: ai_window::AiWindow,
    ai_settings: ai::Settings,
    ai_chat: ai::Chat,
    upload_pending: bool,
}

impl QuipIt {
    fn document_title(&self) -> String {
        let name = self
            .project_path
            .as_ref()
            .or(self.path.as_ref())
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "untitled.qip".into());
        // Keep both the recognizable beginning and the extension of long names.
        let characters: Vec<char> = name.chars().collect();
        let name = if characters.len() > 48 {
            format!(
                "{}…{}",
                characters[..31].iter().collect::<String>(),
                characters[characters.len() - 16..]
                    .iter()
                    .collect::<String>()
            )
        } else {
            name.into_owned()
        };
        format!("QuipIt - {name}")
    }

    fn new_image(&mut self, ctx: &egui::Context) {
        self.run_command(
            ctx,
            commands::Command::NewDocument {
                width: self.new_width.clamp(1, NEW_IMAGE_MAX_SIDE),
                height: self.new_height.clamp(1, NEW_IMAGE_MAX_SIDE),
                background: self
                    .new_background
                    .unwrap_or(Color32::WHITE)
                    .to_srgba_unmultiplied(),
            },
        );
    }

    fn place_shape(&mut self, kind: shapes::ShapeKind, ctx: &egui::Context) {
        self.run_command(ctx, commands::Command::AddShape { kind });
    }

    fn place_text(&mut self, kind: text::Kind, ctx: &egui::Context) {
        self.run_command(ctx, commands::Command::AddText { kind });
    }

    fn edit_object(&mut self, index: usize, ctx: &egui::Context) {
        if let Some(object) = self.objects.get(index) {
            self.run_command(
                ctx,
                commands::Command::SelectObject {
                    id: object.object.id,
                },
            );
        }
    }

    fn commit_insert(&mut self, ctx: &egui::Context) {
        self.run_command(ctx, commands::Command::Apply);
    }

    fn discard_selection(&mut self, ctx: &egui::Context) {
        self.run_command(ctx, commands::Command::Cancel);
    }

    fn select_background(&mut self, ctx: &egui::Context) {
        self.run_command(ctx, commands::Command::SelectBackground);
    }

    fn delete_object(&mut self, ctx: &egui::Context) {
        if self.inserted.is_some() {
            self.run_command(
                ctx,
                commands::Command::Delete {
                    target: commands::Target::Selected,
                },
            );
        }
    }

    fn crop_to_selection(&mut self, ctx: &egui::Context) {
        if let Some(rect) = self.selection {
            self.run_command(ctx, commands::Command::Crop { rect });
        }
    }

    fn resize_canvas(&mut self, ctx: &egui::Context) {
        self.run_command(
            ctx,
            commands::Command::Resize {
                width: self.width,
                height: self.height,
            },
        );
    }

    fn history(&mut self, redo: bool, ctx: &egui::Context) {
        self.run_command(
            ctx,
            if redo {
                commands::Command::Redo
            } else {
                commands::Command::Undo
            },
        );
    }

    fn object_list_action(
        &mut self,
        index: usize,
        action: &str,
        target: usize,
        ctx: &egui::Context,
    ) {
        let objects = self.inspect().objects;
        let Some(object) = objects.get(index) else {
            return;
        };
        // List duplication keeps its committed-row behavior; the command API
        // selects copies so scripted follow-up edits can address them.
        if action == "duplicate" {
            self.do_object_list_action(index, action, target, ctx);
            return;
        }
        let selected = commands::Target::Object { id: object.id };
        let command = match action {
            "visibility" => commands::Command::SetVisibility {
                target: selected,
                visible: !object.visible,
            },
            "delete" => commands::Command::Delete { target: selected },
            "duplicate" => commands::Command::Duplicate { target: selected },
            _ => {
                let destination = match action {
                    "forward" => (index + 1).min(objects.len() - 1),
                    "backward" => index.saturating_sub(1),
                    "front" => objects.len() - 1,
                    "back" => 0,
                    "reorder" => target.min(objects.len() - 1),
                    _ => return,
                };
                commands::Command::Reorder {
                    target: selected,
                    index: destination,
                }
            }
        };
        self.run_command(ctx, command);
    }

    fn open(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("QuipIt project", &["qip", "quipit"])
            .add_filter(
                "Images",
                &[
                    "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "ico", "avif",
                    "pnm", "tga", "dds", "exr", "hdr", "qoi",
                ],
            )
            .pick_file()
        else {
            return;
        };
        self.run_command(ctx, commands::Command::OpenDocument { path });
    }

    fn request_open_path(&mut self, path: PathBuf, ctx: &egui::Context) {
        if self.dirty || self.inserted.is_some() {
            self.pending_open_path = Some(path);
            self.pending_open = true;
        } else {
            self.run_command(ctx, commands::Command::OpenDocument { path });
        }
    }

    fn receive_dropped_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        let project = path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("qip") || extension.eq_ignore_ascii_case("quipit")
        });
        if project || self.pixels.is_none() {
            self.request_open_path(path, ctx);
        } else {
            self.dropped_image = Some(path);
        }
    }

    fn handle_canvas_drop(&mut self, ctx: &egui::Context, rect: egui::Rect) {
        if self.show_about
            || self.pending_open
            || self.pending_new
            || self.confirm_close
            || self.dropped_image.is_some()
            || self.save_as_dialog.is_some()
            || self.show_new
            || self.show_resize
            || self.show_shapes
            || self.show_text_picker
            || self.palette.edit.is_some()
        {
            return;
        }
        let files = ctx.input(|input| {
            if input
                .pointer
                .latest_pos()
                .is_some_and(|point| !rect.contains(point))
            {
                Vec::new()
            } else {
                input.raw.dropped_files.clone()
            }
        });
        if files.len() > 1 {
            self.status = "Drop one file at a time to open or import it".into();
        } else if let Some(file) = files.into_iter().next() {
            if let Some(path) = file.path {
                self.receive_dropped_file(path, ctx);
            } else {
                self.status = "This dropped file has no local path".into();
            }
        }
    }

    fn dropped_image_dialog(&mut self, ctx: &egui::Context) {
        let Some(path) = self.dropped_image.clone() else {
            return;
        };
        let mut choice = None;
        egui::Modal::new(egui::Id::new("dropped_image_choice")).show(ctx, |ui| {
            ui.heading("Open or import image");
            ui.label(path.file_name().unwrap_or_default().to_string_lossy());
            ui.label("Open as a document or import into the current canvas?");
            ui.horizontal(|ui| {
                if ui.button("Open").clicked() {
                    choice = Some(true);
                }
                if ui.button("Import").clicked() {
                    choice = Some(false);
                }
                if ui.button("Cancel").clicked() {
                    self.dropped_image = None;
                }
            });
        });
        if let Some(open) = choice {
            self.resolve_dropped_image(open, ctx);
        }
    }

    fn resolve_dropped_image(&mut self, open: bool, ctx: &egui::Context) {
        if let Some(path) = self.dropped_image.take() {
            if open {
                self.request_open_path(path, ctx);
            } else {
                self.run_command(ctx, commands::Command::InsertImage { path });
            }
        }
    }

    fn continue_document_action(&mut self, ctx: &egui::Context) {
        // Finish the object only after approval so Cancel preserves the edit.
        self.commit_insert(ctx);
        if self.confirm_close {
            self.close_approved = true;
            self.dirty = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.pending_open {
            if let Some(path) = self.pending_open_path.take() {
                self.run_command(ctx, commands::Command::OpenDocument { path });
            } else {
                self.open(ctx);
            }
        }
        if self.pending_new {
            self.new_aspect_ratio = None;
            self.new_width = 1280;
            self.new_height = 720;
            self.show_new = true;
        }
        self.pending_new = false;
        self.confirm_close = false;
        self.pending_open = false;
        self.pending_open_path = None;
    }

    fn insert(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Images",
                &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"],
            )
            .pick_file()
        else {
            return;
        };
        self.run_command(ctx, commands::Command::InsertImage { path });
    }

    fn save(&mut self, save_as: bool, ctx: &egui::Context) {
        self.update_shape();
        if self.pixels.is_none() {
            return;
        }
        let current = self.project_path.as_ref().or(self.path.as_ref());
        let qoi = current.is_some_and(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("qoi"))
        });
        if save_as || current.is_none() || qoi {
            self.begin_save_as();
        } else if let Some(path) = current.cloned() {
            self.save_document_path(path, ctx);
        }
    }

    fn save_as_target(mut path: PathBuf, extension: Option<&str>) -> PathBuf {
        if let Some(extension) = extension
            && !path
                .extension()
                .is_some_and(|actual| actual.eq_ignore_ascii_case(extension))
        {
            path.set_extension(extension);
        }
        path
    }

    fn save_document_path(&mut self, path: PathBuf, ctx: &egui::Context) {
        let target = path.display().to_string();
        let project = path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("qip") || extension.eq_ignore_ascii_case("quipit")
        });
        let command = if project {
            commands::Command::SaveProject { path }
        } else {
            commands::Command::ExportImage { path }
        };
        match self.execute(ctx, command) {
            Ok(_) => {
                self.save_error = None;
                if project {
                    self.path = None;
                } else {
                    self.project_path = None;
                }
                self.dirty = false;
            }
            Err(error) => {
                let message = format!("Could not save {target}\n\n{error}");
                self.status = message.clone();
                self.save_error = Some(message);
                self.dirty = true;
            }
        }
    }

    fn save_error_dialog(&mut self, ctx: &egui::Context) {
        if let Some(message) = self.save_error.clone() {
            egui::Modal::new(egui::Id::new("save_error")).show(ctx, |ui| {
                ui.set_max_width(520.0);
                ui.heading("Save failed");
                ui.label(message);
                ui.label("Your changes are still open. You can retry Save or use Save As.");
                if ui.button("OK").clicked() {
                    self.save_error = None;
                }
            });
        }
    }

    fn begin_background_color(&mut self) {
        let Some(pixels) = &self.pixels else {
            return;
        };
        let Some(pixel) = pixels.pixels().next() else {
            return;
        };
        let color = Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]);
        self.background_before = Some(pixels.clone());
        self.background_preview = Some(color);
        self.palette
            .begin(colors::Target::CanvasBackground, color, 0.0);
    }

    fn objects_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut select = None;
        let mut row_action = None;
        let mut select_background = false;
        let active_name = self.selected_object_name();
        let active_index = active_name.as_ref().map(|_| {
            self.object_before
                .as_ref()
                .map_or(self.objects.len(), |(i, _)| *i)
        });
        egui::TopBottomPanel::top("objects")
            .frame(toolbar::panel_frame(ui.visuals().panel_fill))
            .show_separator_line(false)
            .resizable(true)
            .default_height(300.0)
            .min_height(150.0)
            .max_height((ui.available_height() - 180.0).max(150.0))
            .show_inside(ui, |ui| {
                if self.palette.edit.is_some()
                    || self.show_new
                    || self.pending_new
                    || self.pending_open
                    || self.confirm_close
                {
                    ui.disable();
                }
                toolbar::section_header(ui, "Objects");
                egui::ScrollArea::vertical()
                    .id_salt("object_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Include the object being edited at its original position.
                        let count = self.objects.len() + usize::from(active_index.is_some());
                        for index in (0..count).rev() {
                            let (name, visible) = if active_index == Some(index) {
                                (
                                    active_name.as_ref().unwrap().clone(),
                                    self.object_before.as_ref().is_none_or(|(_, o)| o.visible),
                                )
                            } else {
                                let object = &self.objects
                                    [index - usize::from(active_index.is_some_and(|i| i < index))];
                                (object.name.clone(), object.visible)
                            };
                            ui.push_id(index, |ui| {
                                ui.horizontal(|ui| {
                                    if objects::visibility_button(ui, visible).clicked() {
                                        row_action = Some((index, "visibility", index));
                                    }
                                    let response = ui.add(
                                        egui::Button::new(name)
                                            .truncate()
                                            .min_size(egui::vec2(ui.available_width(), 24.0))
                                            .selected(active_index == Some(index))
                                            .sense(egui::Sense::click_and_drag()),
                                    );
                                    if response.clicked() && active_index != Some(index) {
                                        select = Some(
                                            index
                                                - usize::from(
                                                    active_index.is_some_and(|i| i < index),
                                                ),
                                        );
                                    }
                                    if ui.is_enabled() {
                                        response.dnd_set_drag_payload(objects::ObjectDrag(index));
                                        if response
                                            .dnd_hover_payload::<objects::ObjectDrag>()
                                            .is_some()
                                        {
                                            let above = ui
                                                .input(|i| i.pointer.interact_pos())
                                                .is_some_and(|p| p.y < response.rect.center().y);
                                            let y = if above {
                                                response.rect.top()
                                            } else {
                                                response.rect.bottom()
                                            };
                                            ui.painter().hline(
                                                response.rect.x_range(),
                                                y,
                                                egui::Stroke::new(
                                                    2.0_f32,
                                                    ui.visuals().selection.bg_fill,
                                                ),
                                            );
                                            if let Some(source) = response
                                                .dnd_release_payload::<objects::ObjectDrag>()
                                            {
                                                let slot = index + usize::from(above);
                                                let target = slot - usize::from(source.0 < slot);
                                                row_action = Some((source.0, "reorder", target));
                                            }
                                        }
                                    }
                                    response.context_menu(|ui| {
                                        for (label, action) in [
                                            ("Duplicate", "duplicate"),
                                            ("Delete", "delete"),
                                            ("Move up", "forward"),
                                            ("Move down", "backward"),
                                            ("Move to top", "front"),
                                            ("Move to bottom", "back"),
                                        ] {
                                            if ui.button(label).clicked() {
                                                row_action = Some((index, action, index));
                                                ui.close();
                                            }
                                        }
                                    });
                                });
                            });
                        }
                        if self.pixels.is_some() {
                            ui.horizontal(|ui| {
                                ui.add_space(32.0);
                                if ui
                                    .add(
                                        egui::Button::new("Canvas")
                                            .selected(self.background_selected)
                                            .min_size(egui::vec2(ui.available_width(), 24.0)),
                                    )
                                    .clicked()
                                {
                                    select_background = true;
                                }
                            });
                        }
                    });
            });
        if select_background {
            self.select_background(ctx);
        } else if let Some((index, action, target)) = row_action {
            self.object_list_action(index, action, target, ctx);
        } else if let Some(index) = select {
            // Hidden objects become visible when selected for editing.
            self.edit_object(index, ctx);
        }
    }

    #[cfg(test)]
    fn arrange_object(&mut self, command: &str, ctx: &egui::Context) {
        if self.inserted.is_none() {
            return;
        }
        let index = self
            .object_before
            .as_ref()
            .map_or(self.objects.len(), |(i, _)| *i);
        self.commit_insert(ctx);
        self.checkpoint();
        let target = match command {
            "forward" => (index + 1).min(self.objects.len() - 1),
            "backward" => index.saturating_sub(1),
            "front" => self.objects.len() - 1,
            "back" => 0,
            "duplicate" => {
                let mut copy = self.objects[index].clone();
                copy.object.id = commands::ObjectId::new();
                copy.object.restore_texture(ctx);
                copy.name = format!("{} copy", copy.name);
                copy.object.rect = copy.object.rect.translate(egui::vec2(12.0, 12.0));
                self.objects.insert(index + 1, copy);
                index + 1
            }
            _ => return,
        };
        if command != "duplicate" {
            let object = self.objects.remove(index);
            self.objects.insert(target, object);
        }
        self.dirty = true;
        self.edit_object(target, ctx);
    }

    #[cfg(test)]
    fn replace(&mut self, pixels: RgbaImage, ctx: &egui::Context) {
        self.checkpoint();
        self.pixels = Some(pixels);
        self.dirty = true;
        self.refresh(ctx);
    }

    fn begin_crop(&mut self) {
        let Some(pixels) = &self.pixels else { return };
        self.crop_mode = true;
        self.show_resize = false;
        if self.selection.is_none() {
            let (w, h) = pixels.dimensions();
            let (x, y) = (w / 10, h / 10);
            self.selection = Some([x, y, w - 2 * x, h - 2 * y]);
        }
        self.status =
            "Move the rectangle or drag its eight handles. Hold Shift while resizing to keep proportions. Enter applies the crop; Escape cancels."
                .into();
    }

    fn preview_color(&mut self, edit: colors::Edit) {
        let color = colors::with_transparency(edit.color, edit.transparency);
        match edit.target {
            colors::Target::Border | colors::Target::Fill => {
                let mut shape = commands::ShapePatch::default();
                if edit.target == colors::Target::Border {
                    shape.border = Some(color.to_srgba_unmultiplied());
                } else {
                    shape.fill = Some(color.to_srgba_unmultiplied());
                }
                self.preview_object_patch(commands::ObjectPatch {
                    shape: Some(shape),
                    ..Default::default()
                });
            }
            colors::Target::ImageBorder | colors::Target::Image => {
                let mut image = commands::ImagePatch::default();
                if edit.target == colors::Target::ImageBorder {
                    image.border = Some(edit.color.to_srgba_unmultiplied());
                    image.border_transparency = Some(edit.transparency);
                } else {
                    image.transparency = Some(edit.transparency);
                }
                self.preview_object_patch(commands::ObjectPatch {
                    image: Some(image),
                    ..Default::default()
                });
            }
            colors::Target::ImageShadow | colors::Target::ShapeShadow => {
                self.preview_object_patch(commands::ObjectPatch {
                    shadow: Some(commands::ShadowPatch {
                        color: Some(color.to_srgba_unmultiplied()),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
            colors::Target::TextColor
            | colors::Target::TextBorder
            | colors::Target::TextFill
            | colors::Target::TextShadow => {
                let mut text = commands::TextPatch::default();
                match edit.target {
                    colors::Target::TextColor => text.color = Some(color.to_srgba_unmultiplied()),
                    colors::Target::TextBorder => text.border = Some(color.to_srgba_unmultiplied()),
                    colors::Target::TextFill => text.fill = Some(color.to_srgba_unmultiplied()),
                    colors::Target::TextShadow => {
                        text.shadow = Some(commands::ShadowPatch {
                            color: Some(color.to_srgba_unmultiplied()),
                            ..Default::default()
                        })
                    }
                    _ => unreachable!(),
                }
                self.preview_object_patch(commands::ObjectPatch {
                    text: Some(text),
                    ..Default::default()
                });
            }
            colors::Target::Background | colors::Target::CanvasBackground => {
                if let Some(original) = &self.background_before {
                    if self.background_preview != Some(color) {
                        let original_color = colors::with_transparency(
                            edit.original_color,
                            edit.original_transparency,
                        );
                        if color == original_color {
                            self.pixels = Some(original.clone());
                        } else {
                            self.fill_background(color.to_srgba_unmultiplied());
                        }
                        self.background_preview = Some(color);
                        self.upload_pending = true;
                    }
                } else {
                    self.new_background = Some(color);
                }
            }
        }
    }

    fn finish_color(&mut self, accept: bool) {
        if let Some(mut edit) = self.palette.edit.take() {
            if !accept {
                edit.color = edit.original_color;
                edit.transparency = edit.original_transparency;
            }
            self.preview_color(edit);
            if accept && edit.target != colors::Target::Image {
                self.palette
                    .remember(colors::with_transparency(edit.color, edit.transparency));
            }
        }
        self.background_preview = None;
        if let Some(original) = self.background_before.take() {
            let preview = self.pixels.replace(original);
            if accept && preview.as_ref() != self.pixels.as_ref() {
                self.checkpoint();
                self.pixels = preview;
                self.dirty = true;
            }
            self.upload_pending = true;
        }
        self.palette.picking = false;
        self.palette.custom_original = None;
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.show_about || self.save_as_dialog.is_some() || self.save_error.is_some() {
            return;
        }
        if self.active_text.is_some()
            && self.palette.edit.is_none()
            && ctx.memory(|m| m.has_focus(egui::Id::new("canvas_text_editor")))
        {
            if ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::CTRL, egui::Key::S)
                    || i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)
            }) {
                self.commit_insert(ctx);
                self.save(false, ctx);
            }
            return;
        }
        if self.palette.edit.is_some()
            || ctx.wants_keyboard_input()
            || self.pending_open
            || self.pending_new
            || self.confirm_close
            || self.show_new
            || self.show_resize
            || self.show_shapes
            || self.dropped_image.is_some()
            || self.save_as_dialog.is_some()
        {
            return;
        }
        let (undo, save) = ctx.input_mut(|input| {
            let undo = input.consume_key(egui::Modifiers::CTRL, egui::Key::Z)
                || input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);
            let save = input.consume_key(egui::Modifiers::CTRL, egui::Key::S)
                || input.consume_key(egui::Modifiers::COMMAND, egui::Key::S);
            (undo, save)
        });
        if undo {
            self.history(false, ctx);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
            self.delete_object(ctx);
        }
        if save {
            self.commit_insert(ctx);
            self.save(false, ctx);
        }
    }
}

impl QuipIt {
    fn ui(&mut self, ctx: &egui::Context) {
        if !self.ai_window.detached || !self.show_ai_commands {
            self.ai_settings.dialog(ctx);
        }
        if self.palette.edit.is_some()
            && (ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
                || ctx.input(|i| i.viewport().close_requested()))
        {
            if self.palette.custom_original.is_some() {
                self.palette.finish_custom(false);
            } else {
                self.finish_color(false);
            }
        }
        if let Some(inserted) = &self.inserted
            && self.palette.edit.is_none()
            && !self.show_new
            && !self.pending_new
            && !self.pending_open
            && !self.confirm_close
            && self.dropped_image.is_none()
            && self.save_as_dialog.is_none()
            && !self.show_about
        {
            let clicked_outside = ctx
                .input(|i| {
                    i.pointer
                        .primary_pressed()
                        .then(|| i.pointer.interact_pos())
                        .flatten()
                })
                .is_some_and(|point| {
                    self.canvas_rect.is_some_and(|(rect, scale)| {
                        let shape_commit_area = rect.contains(point);
                        let foreground = ctx
                            .layer_id_at(point)
                            .is_some_and(|layer| layer.order >= egui::Order::Middle);
                        !inserted.contains_pointer(rect, scale, point)
                            && ((self.active_shape.is_none()
                                && inserted.image_style.is_none()
                                && shape_commit_area)
                                || (shape_commit_area
                                    && !foreground
                                    && !self
                                        .shape_properties_rect
                                        .is_some_and(|panel| panel.contains(point))))
                    })
                });
            let accept_with_enter =
                self.active_text.is_none() && !self.palette.picking && !ctx.wants_keyboard_input();
            if (clicked_outside && !self.palette.picking && !self.curve_edit)
                || ctx.input(|i| {
                    i.viewport().close_requested()
                        || (accept_with_enter && i.key_pressed(egui::Key::Enter))
                })
            {
                self.commit_insert(ctx);
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty && !self.close_approved {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        self.shortcuts(ctx);
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            if self.palette.edit.is_some() {
                ui.disable();
            }
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New…").clicked() {
                        ui.close();
                        if self.dirty || self.inserted.is_some() {
                            self.pending_new = true;
                        } else {
                            self.new_aspect_ratio = None;
                            self.new_width = 1280;
                            self.new_height = 720;
                            self.show_new = true;
                        }
                    }
                    if ui.button("Open…").clicked() {
                        ui.close();
                        if self.dirty || self.inserted.is_some() {
                            self.pending_open = true;
                        } else {
                            self.open(ctx);
                        }
                    }
                    ui.add_enabled_ui(self.pixels.is_some(), |ui| {
                        if ui.button("Insert…").clicked() {
                            ui.close();
                            self.insert(ctx);
                        }
                        ui.separator();
                        if ui
                            .add(egui::Button::new("Save").shortcut_text("Ctrl+S"))
                            .clicked()
                        {
                            ui.close();
                            self.commit_insert(ctx);
                            self.save(false, ctx);
                        }
                        if ui.button("Save As…").clicked() {
                            ui.close();
                            self.begin_save_as();
                        }
                    });
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ui.close();
                        if self.dirty || self.inserted.is_some() {
                            self.confirm_close = true;
                        } else {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(
                            !self.undo.is_empty(),
                            egui::Button::new("Undo").shortcut_text("Ctrl+Z"),
                        )
                        .clicked()
                    {
                        ui.close();
                        self.history(false, ctx);
                    }
                    if ui
                        .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                        .clicked()
                    {
                        ui.close();
                        self.history(true, ctx);
                    }
                });
                ui.menu_button("Image", |ui| {
                    ui.add_enabled_ui(self.pixels.is_some(), |ui| {
                        if ui.button("Crop").clicked() {
                            ui.close();
                            self.commit_insert(ctx);
                            self.begin_crop();
                        }
                        if ui.button("Resize…").clicked() {
                            ui.close();
                            self.commit_insert(ctx);
                            let pixels = self.pixels.as_ref().unwrap();
                            self.width = pixels.width();
                            self.height = pixels.height();
                            self.crop_mode = false;
                            self.selection = None;
                            self.show_resize = true;
                        }
                    });
                });
                if self.dirty || self.inserted.is_some() {
                    ui.separator();
                    ui.label("Unsaved changes");
                }
            });
        });
        egui::TopBottomPanel::top("graphic_toolbar").show(ctx, |ui| {
            if self.palette.edit.is_some() {
                ui.disable();
            }
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                let tools_width =
                    (ui.available_width() - toolbar::BUTTON_SIZE - ui.spacing().item_spacing.x)
                        .max(0.0);
                egui::ScrollArea::horizontal()
                    .id_salt("toolbar_scroll")
                    .max_width(tools_width)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if tool_button(ui, Icon::New, "New image", true, false).clicked() {
                                if self.dirty || self.inserted.is_some() {
                                    self.pending_new = true;
                                } else {
                                    self.new_aspect_ratio = None;
                                    self.new_width = 1280;
                                    self.new_height = 720;
                                    self.show_new = true;
                                }
                            }
                            if tool_button(ui, Icon::Open, "Open image", true, false).clicked() {
                                if self.dirty || self.inserted.is_some() {
                                    self.pending_open = true;
                                } else {
                                    self.open(ctx);
                                }
                            }
                            let loaded = self.pixels.is_some();
                            if tool_button(ui, Icon::Insert, "Insert image", loaded, false)
                                .clicked()
                            {
                                self.insert(ctx);
                            }
                            if tool_button(ui, Icon::Save, "Save", loaded, false).clicked() {
                                self.commit_insert(ctx);
                                self.save(false, ctx);
                            }
                            ui.separator();
                            if tool_button(ui, Icon::Undo, "Undo", !self.undo.is_empty(), false)
                                .clicked()
                            {
                                self.history(false, ctx);
                            }
                            if tool_button(ui, Icon::Redo, "Redo", !self.redo.is_empty(), false)
                                .clicked()
                            {
                                self.history(true, ctx);
                            }
                            ui.separator();
                            if tool_button(
                                ui,
                                Icon::Crop,
                                "Crop: drag a rectangle on the image",
                                loaded,
                                self.crop_mode,
                            )
                            .clicked()
                            {
                                self.commit_insert(ctx);
                                self.begin_crop();
                            }
                            if tool_button(
                                ui,
                                Icon::Resize,
                                "Resize image",
                                loaded,
                                self.show_resize,
                            )
                            .clicked()
                            {
                                self.commit_insert(ctx);
                                let pixels = self.pixels.as_ref().unwrap();
                                self.width = pixels.width();
                                self.height = pixels.height();
                                self.crop_mode = false;
                                self.selection = None;
                                self.show_resize = true;
                            }
                            ui.separator();
                            if tool_button(
                                ui,
                                Icon::Shapes,
                                "2D Shapes: lines, curves and shapes",
                                loaded,
                                self.show_shapes,
                            )
                            .clicked()
                            {
                                self.show_shapes = !self.show_shapes;
                                self.crop_mode = false;
                                self.selection = None;
                                self.show_resize = false;
                            }
                            if tool_button(
                                ui,
                                Icon::Text,
                                "Add text rectangle, speech balloon or thought bubble",
                                loaded,
                                self.show_text_picker,
                            )
                            .clicked()
                            {
                                self.show_text_picker = !self.show_text_picker;
                            }
                            if self.selection.is_some() && ui.button("Apply crop").clicked() {
                                self.crop_to_selection(ctx);
                            }
                            ui.separator();
                            if tool_button(
                                ui,
                                Icon::Ai,
                                "Show or hide AI commands",
                                true,
                                self.show_ai_commands,
                            )
                            .clicked()
                            {
                                self.show_ai_commands = !self.show_ai_commands;
                            }
                        });
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.about_button(ui);
                });
            });
            ui.add_space(3.0);
        });
        egui::TopBottomPanel::bottom("status_footer")
            .frame(egui::Frame::new().fill(ctx.style().visuals.panel_fill))
            .show_separator_line(false)
            .exact_height(20.0)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(&self.status)
                            .truncate()
                            .halign(egui::Align::Min),
                    );
                });
            });
        {
            let mut apply = false;
            let mut cancel = false;
            let mut horizontal = false;
            let mut vertical = false;
            let panel = egui::SidePanel::right("shape_properties")
                .frame(egui::Frame::NONE)
                .show_separator_line(false)
                .default_width(260.0)
                .min_width(230.0)
                .resizable(true)
                .show(ctx, |ui| {
                    self.objects_panel(ctx, ui);
                    toolbar::panel_frame(ui.visuals().panel_fill)
                        .show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            let previous_rotation = self.shape_rotation;
                            let selected_name = self
                                .selected_object_name()
                                .or_else(|| self.background_selected.then(|| "Canvas".into()));
                            toolbar::section_header(ui, "Properties");
                            if let Some(name) = &selected_name {
                                ui.add(
                                    egui::Label::new(egui::RichText::new(name).strong()).truncate(),
                                );
                            }
                            if selected_name.is_none() {
                                return previous_rotation;
                            }
                            if self.palette.edit.is_some()
                                || self.show_new
                                || self.pending_new
                                || self.pending_open
                                || self.confirm_close
                            {
                                ui.disable();
                            }
                            let mut draft = self.edit_draft();
                            egui::ScrollArea::vertical()
                                .id_salt("object_properties")
                                .show(ui, |ui| {
                                    if self.background_selected {
                                        let pixel = self.pixels.as_ref().unwrap().get_pixel(0, 0).0;
                                        let color = Color32::from_rgba_unmultiplied(
                                            pixel[0], pixel[1], pixel[2], pixel[3],
                                        );
                                        if colors::selector(ui, "Fill color", color).clicked() {
                                            self.begin_background_color();
                                        }
                                        return;
                                    }
                                    let Some(edit) = &mut draft else {
                                        return;
                                    };
                                    if let Some((_, object)) = &self.object_before {
                                        ui.label("Name");
                                        let name =
                                            edit.name.get_or_insert_with(|| object.name.clone());
                                        ui.text_edit_singleline(name);
                                        ui.separator();
                                    }
                                    if let Some(data) = &mut edit.text {
                                        edit.text_changed |= text::properties(
                                            ui,
                                            data,
                                            &mut self.palette,
                                            self.text_overflow,
                                        );
                                        if let Some(style) = &mut edit.image {
                                            ui.separator();
                                            ui.checkbox(&mut style.shadow, "Border shadow");
                                            if style.shadow {
                                                if colors::selector(
                                                    ui,
                                                    "Shadow color",
                                                    style.shadow_color,
                                                )
                                                .clicked()
                                                {
                                                    self.palette.begin(
                                                        colors::Target::ImageShadow,
                                                        style.shadow_color,
                                                        0.0,
                                                    );
                                                }
                                                crate::toolbar::numeric(
                                                    ui,
                                                    "X",
                                                    &mut style.shadow_offset.x,
                                                    -64.0..=64.0,
                                                    " px",
                                                    false,
                                                );
                                                crate::toolbar::numeric(
                                                    ui,
                                                    "Y",
                                                    &mut style.shadow_offset.y,
                                                    -64.0..=64.0,
                                                    " px",
                                                    false,
                                                );
                                                crate::toolbar::numeric(
                                                    ui,
                                                    "Blur",
                                                    &mut style.shadow_blur,
                                                    0.0..=32.0,
                                                    " px",
                                                    false,
                                                );
                                            }
                                        }
                                        ui.separator();

                                        crate::toolbar::numeric(
                                            ui,
                                            "Rotation",
                                            &mut edit.rotation,
                                            -180.0..=180.0,
                                            "°",
                                            false,
                                        );
                                        ui.horizontal(|ui| {
                                            if tool_button(
                                                ui,
                                                Icon::RotateLeft,
                                                "Rotate left 90°",
                                                true,
                                                false,
                                            )
                                            .clicked()
                                            {
                                                edit.rotation -= 90.0;
                                            }
                                            if tool_button(
                                                ui,
                                                Icon::RotateRight,
                                                "Rotate right 90°",
                                                true,
                                                false,
                                            )
                                            .clicked()
                                            {
                                                edit.rotation += 90.0;
                                            }
                                            vertical = tool_button(
                                                ui,
                                                Icon::FlipVertical,
                                                "Flip vertically",
                                                true,
                                                false,
                                            )
                                            .clicked();
                                            horizontal = tool_button(
                                                ui,
                                                Icon::FlipHorizontal,
                                                "Flip horizontally",
                                                true,
                                                false,
                                            )
                                            .clicked();
                                        });
                                        ui.separator();
                                        ui.horizontal(|ui| {
                                            cancel = ui.button("Cancel").clicked();
                                            apply = ui.button("Apply").clicked();
                                        });
                                        ui.label(
                                    "Ctrl+Enter: line break. Click away: accept. Escape: cancel.",
                                );
                                        return;
                                    }
                                    let Some((kind, style)) = edit.shape.as_mut() else {
                                        if let Some(inserted) = &self.inserted {
                                            let thumbnail = inserted.texture_id();
                                            if let Some(style) = &mut edit.image {
                                                toolbar::border_size(ui, &mut style.border_width);
                                                if colors::selector(
                                                    ui,
                                                    "Border",
                                                    colors::with_transparency(
                                                        style.border,
                                                        style.border_transparency,
                                                    ),
                                                )
                                                .clicked()
                                                {
                                                    self.palette.begin(
                                                        colors::Target::ImageBorder,
                                                        style.border,
                                                        style.border_transparency,
                                                    );
                                                }
                                                if colors::image_selector(ui, thumbnail).clicked() {
                                                    self.palette.begin(
                                                        colors::Target::Image,
                                                        Color32::WHITE,
                                                        style.image_transparency,
                                                    );
                                                }
                                                toolbar::numeric(
                                                    ui,
                                                    "Transparency",
                                                    &mut style.image_transparency,
                                                    0.0..=100.0,
                                                    "%",
                                                    false,
                                                );
                                                ui.checkbox(&mut style.shadow, "Drop shadow");
                                                if style.shadow {
                                                    if colors::selector(
                                                        ui,
                                                        "Shadow",
                                                        style.shadow_color,
                                                    )
                                                    .clicked()
                                                    {
                                                        self.palette.begin(
                                                            colors::Target::ImageShadow,
                                                            style.shadow_color,
                                                            0.0,
                                                        );
                                                    }
                                                    crate::toolbar::numeric(
                                                        ui,
                                                        "X",
                                                        &mut style.shadow_offset.x,
                                                        -64.0..=64.0,
                                                        " px",
                                                        false,
                                                    );
                                                    crate::toolbar::numeric(
                                                        ui,
                                                        "Y",
                                                        &mut style.shadow_offset.y,
                                                        -64.0..=64.0,
                                                        " px",
                                                        false,
                                                    );
                                                    crate::toolbar::numeric(
                                                        ui,
                                                        "Blur",
                                                        &mut style.shadow_blur,
                                                        0.0..=32.0,
                                                        " px",
                                                        false,
                                                    );
                                                }

                                                crate::toolbar::numeric(
                                                    ui,
                                                    "Rotation",
                                                    &mut edit.rotation,
                                                    -180.0..=180.0,
                                                    "°",
                                                    false,
                                                );
                                                ui.horizontal(|ui| {
                                                    if tool_button(
                                                        ui,
                                                        Icon::RotateLeft,
                                                        "Rotate left 90°",
                                                        true,
                                                        false,
                                                    )
                                                    .clicked()
                                                    {
                                                        edit.rotation -= 90.0;
                                                    }
                                                    if tool_button(
                                                        ui,
                                                        Icon::RotateRight,
                                                        "Rotate right 90°",
                                                        true,
                                                        false,
                                                    )
                                                    .clicked()
                                                    {
                                                        edit.rotation += 90.0;
                                                    }
                                                    vertical = tool_button(
                                                        ui,
                                                        Icon::FlipVertical,
                                                        "Flip vertically",
                                                        true,
                                                        false,
                                                    )
                                                    .clicked();
                                                    horizontal = tool_button(
                                                        ui,
                                                        Icon::FlipHorizontal,
                                                        "Flip horizontally",
                                                        true,
                                                        false,
                                                    )
                                                    .clicked();
                                                });
                                                ui.separator();
                                                ui.horizontal(|ui| {
                                                    cancel = ui.button("Cancel").clicked();
                                                    apply = ui.button("Apply").clicked();
                                                });
                                                ui.label("Shift keeps proportions while resizing.");
                                                return;
                                            }
                                        }
                                        return;
                                    };
                                    ui.label(kind.name());
                                    if matches!(
                                        kind,
                                        shapes::ShapeKind::Curve4 | shapes::ShapeKind::Curve5
                                    ) {
                                        ui.horizontal(|ui| {
                                            ui.selectable_value(
                                                &mut edit.curve_editing,
                                                true,
                                                "Edit curve",
                                            );
                                            ui.selectable_value(
                                                &mut edit.curve_editing,
                                                false,
                                                "Move / resize",
                                            );
                                        });
                                        if edit.curve_editing {
                                            ui.label(
                                        "Drag the endpoints and control points to shape the bends.",
                                    );
                                        }
                                    }

                                    ui.separator();
                                    toolbar::border_size(ui, &mut style.border_width);
                                    let closed = !matches!(
                                        kind,
                                        shapes::ShapeKind::Line
                                            | shapes::ShapeKind::Curve3
                                            | shapes::ShapeKind::Curve4
                                            | shapes::ShapeKind::Curve5
                                    );
                                    if colors::selector(ui, "Border", style.border).clicked() {
                                        self.palette.begin(
                                            colors::Target::Border,
                                            style.border,
                                            0.0,
                                        );
                                    }
                                    ui.add_enabled_ui(closed, |ui| {
                                        if colors::selector(ui, "Fill", style.fill).clicked() {
                                            self.palette.begin(
                                                colors::Target::Fill,
                                                style.fill,
                                                0.0,
                                            );
                                        }
                                    });
                                    if let Some(inserted) = &mut self.inserted {
                                        let shadow = inserted.image_style.get_or_insert_default();
                                        ui.checkbox(&mut shadow.shadow, "Drop shadow");
                                        if shadow.shadow {
                                            if colors::selector(ui, "Shadow", shadow.shadow_color)
                                                .clicked()
                                            {
                                                self.palette.begin(
                                                    colors::Target::ShapeShadow,
                                                    shadow.shadow_color,
                                                    0.0,
                                                );
                                            }
                                            crate::toolbar::numeric(
                                                ui,
                                                "X",
                                                &mut shadow.shadow_offset.x,
                                                -64.0..=64.0,
                                                " px",
                                                false,
                                            );
                                            crate::toolbar::numeric(
                                                ui,
                                                "Y",
                                                &mut shadow.shadow_offset.y,
                                                -64.0..=64.0,
                                                " px",
                                                false,
                                            );
                                            crate::toolbar::numeric(
                                                ui,
                                                "Blur",
                                                &mut shadow.shadow_blur,
                                                0.0..=32.0,
                                                " px",
                                                false,
                                            );
                                        }
                                    }
                                    ui.separator();

                                    crate::toolbar::numeric(
                                        ui,
                                        "Rotation",
                                        &mut edit.rotation,
                                        -180.0..=180.0,
                                        "°",
                                        false,
                                    );
                                    ui.horizontal(|ui| {
                                        if tool_button(
                                            ui,
                                            Icon::RotateLeft,
                                            "Rotate left 90°",
                                            true,
                                            false,
                                        )
                                        .clicked()
                                        {
                                            edit.rotation -= 90.0;
                                        }
                                        if tool_button(
                                            ui,
                                            Icon::RotateRight,
                                            "Rotate right 90°",
                                            true,
                                            false,
                                        )
                                        .clicked()
                                        {
                                            edit.rotation += 90.0;
                                        }
                                        vertical = tool_button(
                                            ui,
                                            Icon::FlipVertical,
                                            "Flip vertically",
                                            true,
                                            false,
                                        )
                                        .clicked();
                                        horizontal = tool_button(
                                            ui,
                                            Icon::FlipHorizontal,
                                            "Flip horizontally",
                                            true,
                                            false,
                                        )
                                        .clicked();
                                    });
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        cancel = ui.button("Cancel").clicked();
                                        apply = ui.button("Apply").clicked();
                                    });
                                    ui.label("Shift keeps proportions while resizing.");
                                });
                            self.apply_ui_draft(draft);
                            previous_rotation
                        })
                        .inner
                });
            let rotation_delta = self.shape_rotation - panel.inner;
            if rotation_delta != 0.0 || horizontal || vertical {
                self.shape_rotation = panel.inner;
                self.run_command(
                    ctx,
                    commands::Command::Transform {
                        target: commands::Target::Selected,
                        degrees: rotation_delta,
                        horizontal,
                        vertical,
                    },
                );
            }
            self.shape_properties_rect = Some(panel.response.rect);
            if cancel {
                self.discard_selection(ctx);
            }
            if apply {
                self.commit_insert(ctx);
            }
            self.update_shape();
        }
        if self.crop_mode
            && self.selection.is_some()
            && ctx.input(|i| i.key_pressed(egui::Key::Enter))
        {
            self.crop_to_selection(ctx);
        }
        if !self.show_about
            && self.save_as_dialog.is_none()
            && self.save_error.is_none()
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            if self.dropped_image.is_some() {
                self.dropped_image = None;
            } else if self.show_new {
                self.show_new = false;
            } else if self.pending_new || self.pending_open || self.confirm_close {
                self.pending_new = false;
                self.pending_open = false;
                self.pending_open_path = None;
                self.confirm_close = false;
            } else {
                self.crop_mode = false;
                self.selection = None;
                self.drag_start = None;
                self.show_resize = false;
                self.discard_selection(ctx);
                self.show_shapes = false;
                self.show_text_picker = false;
                self.status.clear();
            }
        }
        if self.show_new {
            self.show_resize = false;
            self.crop_mode = false;
            self.selection = None;
            let mut open = true;
            egui::Window::new("New image")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if self.palette.edit.is_some() {
                        ui.disable();
                    }
                    ui.label("Create a new canvas (up to 5000 × 5000 pixels)");
                    let mut constrained = self.new_aspect_ratio.is_some();
                    if ui.checkbox(&mut constrained, "Keep proportions").changed() {
                        self.new_aspect_ratio = constrained
                            .then(|| self.new_width.max(1) as f64 / self.new_height.max(1) as f64);
                    }
                    let ratio = self.new_aspect_ratio;
                    let max_width = ratio.map_or(NEW_IMAGE_MAX_SIDE, |r| {
                        (NEW_IMAGE_MAX_SIDE as f64 * r)
                            .min(NEW_IMAGE_MAX_SIDE as f64)
                            .floor()
                            .max(1.0) as u32
                    });
                    let max_height = ratio.map_or(NEW_IMAGE_MAX_SIDE, |r| {
                        (NEW_IMAGE_MAX_SIDE as f64 / r)
                            .min(NEW_IMAGE_MAX_SIDE as f64)
                            .floor()
                            .max(1.0) as u32
                    });
                    ui.horizontal(|ui| {
                        if crate::toolbar::numeric(
                            ui,
                            "Width",
                            &mut self.new_width,
                            1..=max_width,
                            " px",
                            true,
                        ) && let Some(ratio) = ratio
                        {
                            self.new_height = (self.new_width as f64 / ratio)
                                .round()
                                .clamp(1.0, NEW_IMAGE_MAX_SIDE as f64)
                                as u32;
                        }
                    });
                    ui.horizontal(|ui| {
                        if crate::toolbar::numeric(
                            ui,
                            "Height",
                            &mut self.new_height,
                            1..=max_height,
                            " px",
                            true,
                        ) && let Some(ratio) = ratio
                        {
                            self.new_width = (self.new_height as f64 * ratio)
                                .round()
                                .clamp(1.0, NEW_IMAGE_MAX_SIDE as f64)
                                as u32;
                        }
                    });
                    ui.separator();
                    let current = self.new_background.unwrap_or(Color32::WHITE);
                    if colors::selector(ui, "Background", current).clicked() {
                        self.palette.begin(colors::Target::Background, current, 0.0);
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_new = false;
                        }
                        if ui.button("Create").clicked() {
                            self.new_image(ctx);
                        }
                    });
                });
            self.show_new &= open;
        }
        if self.show_resize {
            let mut open = true;
            egui::Window::new("Resize image")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if let Some(pixels) = &self.pixels {
                        let (w, h) = pixels.dimensions();
                        ui.checkbox(&mut self.lock_ratio, "Keep proportions");
                        ui.horizontal(|ui| {
                            if crate::toolbar::numeric(
                                ui,
                                "Width",
                                &mut self.width,
                                1..=16384,
                                " px",
                                true,
                            ) && self.lock_ratio
                            {
                                self.height = ((self.width as f64 * h as f64 / w as f64).round()
                                    as u32)
                                    .clamp(1, 16384);
                            }
                        });
                        ui.horizontal(|ui| {
                            if crate::toolbar::numeric(
                                ui,
                                "Height",
                                &mut self.height,
                                1..=16384,
                                " px",
                                true,
                            ) && self.lock_ratio
                            {
                                self.width = ((self.height as f64 * w as f64 / h as f64).round()
                                    as u32)
                                    .clamp(1, 16384);
                            }
                        });
                        if ui.button("Apply resize").clicked() {
                            self.resize_canvas(ctx);
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_resize = false;
                        }
                    }
                });
            self.show_resize &= open;
        }
        let mut accept_text = false;
        let mut edit_object_index = None;
        let mut select_background = false;
        self.footer(ctx);
        let canvas_panel = egui::CentralPanel::default()
            .frame(toolbar::panel_frame(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                if self.show_new
                    || self.pending_new
                    || self.pending_open
                    || self.confirm_close
                    || self.dropped_image.is_some()
                    || self.save_as_dialog.is_some()
                    || self.show_about
                {
                    ui.disable();
                }
                if let Some(texture) = &self.texture {
                    let available = ui.available_size();
                    let size = texture.size_vec2();
                    let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
                    let image_rect =
                        egui::Rect::from_center_size(ui.max_rect().center(), size * scale);
                    // A checkerboard makes transparent canvas pixels visible.
                    let checker = ui.painter().with_clip_rect(image_rect);
                    let tile = 16.0;
                    for row in 0..(image_rect.height() / tile).ceil() as u32 {
                        for column in 0..(image_rect.width() / tile).ceil() as u32 {
                            let color = if (row + column) % 2 == 0 {
                                Color32::from_gray(205)
                            } else {
                                Color32::from_gray(245)
                            };
                            checker.rect_filled(
                                egui::Rect::from_min_size(
                                    image_rect.min
                                        + egui::vec2(column as f32 * tile, row as f32 * tile),
                                    Vec2::splat(tile),
                                ),
                                0.0,
                                color,
                            );
                        }
                    }
                    ui.painter().image(
                        texture.id(),
                        image_rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    self.canvas_rect = Some((image_rect, scale));
                    if ui.is_enabled()
                        && !self.crop_mode
                        && self.inserted.is_none()
                        && self.palette.edit.is_none()
                        && let Some(pointer) = ctx.input(|i| {
                            i.pointer
                                .primary_pressed()
                                .then(|| i.pointer.interact_pos())
                                .flatten()
                        })
                        && ctx
                            .layer_id_at(pointer)
                            .is_none_or(|layer| layer.order < egui::Order::Middle)
                    {
                        edit_object_index = self.objects.iter().rposition(|text| {
                            text.visible
                                && image_rect.contains(pointer)
                                && text.object.hit_test(image_rect, scale, pointer)
                        });
                        select_background =
                            edit_object_index.is_none() && image_rect.contains(pointer);
                    }

                    let sampling = self.palette.picking;
                    if sampling {
                        ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::Crosshair);
                        let point = ctx.input(|input| {
                            input
                                .pointer
                                .primary_pressed()
                                .then(|| input.pointer.interact_pos())
                                .flatten()
                        });
                        if let Some(color) = point
                            .filter(|point| {
                                ctx.layer_id_at(*point)
                                    .is_none_or(|layer| layer.order < egui::Order::Middle)
                            })
                            .and_then(|point| {
                                colors::sample(&self.composed_canvas().unwrap(), image_rect, point)
                            })
                        {
                            if let Some(edit) = &mut self.palette.edit {
                                edit.color = colors::opaque(color);
                                edit.transparency = if edit.target == colors::Target::Background {
                                    if color.a() == 0 { 100.0 } else { 0.0 }
                                } else {
                                    100.0 * (1.0 - color.a() as f32 / 255.0)
                                };
                            }
                            self.palette.picking = false;
                        }
                    }
                    let response = ui.interact(
                        image_rect,
                        ui.id().with("image_selection"),
                        egui::Sense::drag(),
                    );
                    if self.crop_mode && (self.selection.is_none() || self.drag_start.is_some()) {
                        response
                            .clone()
                            .on_hover_cursor(egui::CursorIcon::Crosshair);
                        if response.drag_started() {
                            self.drag_start = ctx.input(|i| i.pointer.press_origin());
                            self.selection = None;
                        }
                        if response.dragged() || response.drag_stopped() {
                            if let (Some(start), Some(end)) =
                                (self.drag_start, response.interact_pointer_pos())
                            {
                                self.selection = selection_from_drag(image_rect, size, start, end);
                            }
                            if response.drag_stopped() {
                                self.drag_start = None;
                            }
                        }
                    }
                    if let Some([x, y, w, h]) = self.selection {
                        let selected = egui::Rect::from_min_size(
                            image_rect.min + egui::vec2(x as f32, y as f32) * scale,
                            egui::vec2(w as f32, h as f32) * scale,
                        );
                        let painter = ui.painter();
                        let shade = Color32::from_black_alpha(140);
                        for rect in [
                            egui::Rect::from_min_max(
                                image_rect.min,
                                egui::pos2(image_rect.right(), selected.top()),
                            ),
                            egui::Rect::from_min_max(
                                egui::pos2(image_rect.left(), selected.bottom()),
                                image_rect.max,
                            ),
                            egui::Rect::from_min_max(
                                egui::pos2(image_rect.left(), selected.top()),
                                selected.left_bottom(),
                            ),
                            egui::Rect::from_min_max(
                                selected.right_top(),
                                egui::pos2(image_rect.right(), selected.bottom()),
                            ),
                        ] {
                            painter.rect_filled(rect, 0.0, shade);
                        }
                        painter.rect_stroke(
                            selected,
                            0.0,
                            egui::Stroke::new(2.0_f32, Color32::WHITE),
                            egui::StrokeKind::Inside,
                        );
                        painter.rect_stroke(
                            selected.expand(1.0),
                            0.0,
                            egui::Stroke::new(1.0_f32, Color32::BLACK),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if self.crop_mode
                        && self.drag_start.is_none()
                        && let Some(selection) = &mut self.selection
                    {
                        crop::selection_ui(
                            ui,
                            image_rect,
                            scale,
                            [size.x as u32, size.y as u32],
                            selection,
                        );
                    }
                    // Read the selection after its handles have processed this frame's drag.
                    // Keep the readout independent of selection size and image brightness.
                    if self.crop_mode
                        && let Some([x, y, width, height]) = self.selection
                    {
                        let painter = ui.painter();
                        let text = format!("X: {x}  Y: {y}   Width: {width}  Height: {height} px");
                        let galley = painter.layout_no_wrap(
                            text,
                            egui::FontId::proportional(14.0),
                            Color32::WHITE,
                        );
                        let origin = ui.max_rect().min + egui::vec2(12.0, 12.0);
                        let background = egui::Rect::from_min_size(
                            origin,
                            galley.size() + egui::vec2(16.0, 10.0),
                        );
                        painter.rect_filled(background, 4.0, Color32::from_black_alpha(220));
                        painter.galley(origin + egui::vec2(8.0, 5.0), galley, Color32::WHITE);
                    }
                    let previous_edit = self.edit_draft();
                    let mut handle_rotation = 0.0;
                    if let Some(inserted) = &mut self.inserted {
                        inserted.preview(ui, image_rect, scale);
                        if let Some((index, _)) = &self.object_before {
                            for object in self.objects.iter().skip(*index) {
                                if object.visible {
                                    object.object.preview(ui, image_rect, scale);
                                }
                            }
                        }
                        if sampling || self.palette.edit.is_some() {
                            // Picking colors leaves only the object preview visible.
                        } else if self.curve_edit
                            && let Some((kind, style)) = &mut self.active_shape
                        {
                            shapes::curve_editor(
                                ui,
                                *kind,
                                style,
                                &mut inserted.rect,
                                image_rect,
                                scale,
                            );
                        } else {
                            let delta = inserted.ui(
                                ui,
                                image_rect,
                                scale,
                                self.active_shape.is_some() || inserted.image_style.is_some(),
                                self.active_text.as_ref().map(|data| {
                                    text::text_bounds(
                                        data,
                                        inserted.pixels.width(),
                                        inserted.pixels.height(),
                                    )
                                }),
                            );
                            handle_rotation = delta;
                        }
                        if self.palette.edit.is_none()
                            && !self.show_new
                            && !self.pending_new
                            && !self.pending_open
                            && !self.confirm_close
                            && !self.show_about
                            && !sampling
                            && let Some(data) = &mut self.active_text
                        {
                            let (changed, accept) =
                                text::canvas_editor(ui, data, inserted, image_rect, scale);
                            self.text_dirty |= changed;
                            accept_text |= accept;
                            if changed {
                                ctx.request_repaint();
                            }
                        }
                    }
                    self.accept_ui_edit(previous_edit);
                    if handle_rotation != 0.0 {
                        self.run_command(
                            ctx,
                            commands::Command::Transform {
                                target: commands::Target::Selected,
                                degrees: handle_rotation,
                                horizontal: false,
                                vertical: false,
                            },
                        );
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("QuipIt\nOpen or drop a file to begin");
                    });
                }
            });
        self.handle_canvas_drop(ctx, canvas_panel.response.rect);
        self.dropped_image_dialog(ctx);
        if accept_text {
            self.commit_insert(ctx);
        }
        if let Some(index) = edit_object_index {
            self.edit_object(index, ctx);
        } else if select_background {
            self.select_background(ctx);
        }
        if self.upload_pending {
            self.upload_canvas(ctx);
        }
        self.update_shape();
        if self.palette.edit.is_some() {
            let decision = self.palette.dialog(ctx);
            if let Some(edit) = self.palette.edit {
                self.preview_color(edit);
            }
            if let Some(accept) = decision {
                self.finish_color(accept);
            }
            if self.upload_pending {
                self.upload_canvas(ctx);
            }
            self.update_shape();
        }
        if self.show_shapes
            && let Some(kind) = shapes::picker(ctx, &mut self.show_shapes, &mut self.selected_shape)
        {
            match kind {
                shapes::ShapeKind::WordBalloon => self.place_text(text::Kind::Speech, ctx),
                shapes::ShapeKind::ThoughtBalloon => self.place_text(text::Kind::Thought, ctx),
                _ => self.place_shape(kind, ctx),
            }
        }
        if self.show_text_picker {
            let mut open = true;
            let mut chosen = None;
            egui::Window::new("Add text")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    for kind in [text::Kind::Area, text::Kind::Speech, text::Kind::Thought] {
                        if ui.button(kind.name()).clicked() {
                            chosen = Some(kind);
                        }
                    }
                });
            if let Some(kind) = chosen {
                self.place_text(kind, ctx);
                self.show_text_picker = false;
            } else {
                self.show_text_picker &= open;
            }
        }
        if self.pending_open || self.pending_new || self.confirm_close {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("Save your changes before continuing?");
                    ui.horizontal(|ui| {
                        let save = ui.button("Save").clicked();
                        let discard = ui.button("Discard changes").clicked();
                        if save {
                            self.dirty = true;
                            self.save(false, ctx);
                        }
                        if discard || (save && !self.dirty) {
                            self.continue_document_action(ctx);
                            if save {
                                self.dirty = false;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.pending_new = false;
                            self.pending_open = false;
                            self.pending_open_path = None;
                            self.confirm_close = false;
                        }
                    });
                });
        }
        self.about_dialog(ctx);
    }
}

// Convert either drag direction into an image-bounded pixel rectangle.
fn selection_from_drag(
    rect: egui::Rect,
    size: Vec2,
    start: egui::Pos2,
    end: egui::Pos2,
) -> Option<[u32; 4]> {
    let to_pixel = |point: egui::Pos2| {
        let fraction = (point - rect.min) / rect.size();
        egui::pos2(
            fraction.x.clamp(0.0, 1.0) * size.x,
            fraction.y.clamp(0.0, 1.0) * size.y,
        )
    };
    let start = to_pixel(start);
    let end = to_pixel(end);
    if (start.x - end.x).abs() < f32::EPSILON || (start.y - end.y).abs() < f32::EPSILON {
        return None;
    }
    let x = start.x.min(end.x).floor() as u32;
    let y = start.y.min(end.y).floor() as u32;
    let right = start.x.max(end.x).ceil() as u32;
    let bottom = start.y.max(end.y).ceil() as u32;
    Some([x, y, right - x, bottom - y])
}

impl eframe::App for QuipIt {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_ai(ctx);
        self.ui(ctx);
        self.show_save_as_dialog(ctx);
        self.save_error_dialog(ctx);
        self.detached_ai_window(ctx);
        let title = self.document_title();
        if self.window_title != title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_as_webp_honors_selected_format_with_missing_or_previous_extension() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.png");
        RgbaImage::from_pixel(320, 240, image::Rgba([255, 255, 255, 255]))
            .save(&source)
            .unwrap();
        let source_bytes = std::fs::read(&source).unwrap();
        let mut app = QuipIt::default();
        app.request_open_path(source.clone(), &ctx);
        app.place_text(text::Kind::Area, &ctx);
        app.active_text.as_mut().unwrap().content = "Save As WebP".into();
        app.text_dirty = true;
        app.commit_insert(&ctx);
        let expected = app.composed_canvas().unwrap();
        for name in [
            "new-image",
            "old-extension.png",
            "correct.webp",
            "uppercase.WEBP",
        ] {
            let target = QuipIt::save_as_target(directory.path().join(name), Some("webp"));
            app.save_document_path(target.clone(), &ctx);
            assert!(app.save_error.is_none(), "{}", app.status);
            assert!(!app.dirty);
            assert_eq!(
                image::guess_format(&std::fs::read(&target).unwrap()).unwrap(),
                image::ImageFormat::WebP
            );
            assert_eq!(image::open(&target).unwrap().to_rgba8(), expected);
            assert_eq!(app.path.as_ref(), Some(&target));
            assert!(app.project_path.is_none());
            app.edit_object(0, &ctx);
            app.active_text.as_mut().unwrap().content = "Updated WebP".into();
            app.text_dirty = true;
            app.commit_insert(&ctx);
            let updated = app.composed_canvas().unwrap();
            app.save(false, &ctx);
            assert_eq!(image::open(&target).unwrap().to_rgba8(), updated);
            app.edit_object(0, &ctx);
            app.active_text.as_mut().unwrap().content = "Save As WebP".into();
            app.text_dirty = true;
            app.commit_insert(&ctx);
        }
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
    }

    #[test]
    fn toolbar_save_overwrites_the_original_webp_with_flattened_edits() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.webp");
        let original = RgbaImage::from_pixel(320, 240, image::Rgba([255, 255, 255, 255]));
        original.save(&path).unwrap();
        let mut app = QuipIt::default();
        app.request_open_path(path.clone(), &ctx);
        app.place_text(text::Kind::Area, &ctx);
        app.active_text.as_mut().unwrap().content = "Flatten this into the original".into();
        app.text_dirty = true;
        app.update_shape();
        let expected = app.composed_canvas().unwrap();
        assert_ne!(expected, original);
        let frame = |events| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1150.0, 780.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(frame(vec![]), |ctx| app.ui(ctx));
        let output = ctx.run(frame(vec![]), |ctx| app.ui(ctx));
        // The Save icon's outer square is 16 x 16 pixels.
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Rect(rect) = &shape.shape
                    && rect.rect.size() == egui::vec2(16.0, 16.0)
                    && rect.rect.center().y < 100.0
                {
                    return Some(rect.rect.center());
                }
                None
            })
            .expect("Save toolbar icon");
        for pressed in [true, false] {
            let _ = ctx.run(
                frame(vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]),
                |ctx| app.ui(ctx),
            );
        }
        assert_eq!(
            image::open(&path).unwrap().to_rgba8(),
            expected,
            "{}",
            app.status
        );
        assert!(!app.dirty, "{}", app.status);
        assert_eq!(app.path.as_ref(), Some(&path));
        assert!(!path.with_extension("qip").exists());
    }

    #[test]
    fn save_prompt_overwrites_opened_images_with_pending_text_before_opening_drop() {
        for extension in ["png", "jpg", "webp"] {
            let ctx = egui::Context::default();
            let directory = tempfile::tempdir().unwrap();
            let original = directory.path().join(format!("original.{extension}"));
            image::RgbImage::from_pixel(320, 240, image::Rgb([255, 255, 255]))
                .save(&original)
                .unwrap();
            let original_bytes = std::fs::read(&original).unwrap();
            let next = directory.path().join("next.png");
            RgbaImage::new(12, 10).save(&next).unwrap();
            let mut app = QuipIt::default();
            app.request_open_path(original.clone(), &ctx);
            app.place_text(text::Kind::Area, &ctx);
            app.active_text.as_mut().unwrap().content = "Save this text".into();
            app.text_dirty = true;
            app.update_shape();
            let expected = app.composed_canvas().unwrap();
            assert_ne!(expected, app.pixels.clone().unwrap());
            app.request_open_path(next.clone(), &ctx);
            let frame = |events| egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1150.0, 780.0),
                )),
                events,
                ..Default::default()
            };
            let _ = ctx.run(frame(vec![]), |ctx| app.ui(ctx));
            let output = ctx.run(frame(vec![]), |ctx| app.ui(ctx));
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.text() == "Save"
                    {
                        return Some(text.pos + text.galley.size() * 0.5);
                    }
                    None
                })
                .expect("Save button in unsaved changes prompt");
            for pressed in [true, false] {
                let _ = ctx.run(
                    frame(vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]),
                    |ctx| app.ui(ctx),
                );
            }
            assert_eq!(
                app.path.as_ref(),
                Some(&next),
                "{extension}: {}",
                app.status
            );
            assert_ne!(
                std::fs::read(&original).unwrap(),
                original_bytes,
                "{extension}"
            );
            let saved = image::open(&original).unwrap().to_rgba8();
            if extension != "jpg" {
                assert_eq!(saved, expected, "{extension}");
            }
            assert!(saved.pixels().any(|pixel| pixel[0] < 200));
        }
    }

    #[test]
    fn dropped_images_open_import_and_preserve_changes_before_replacement() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("dropped.png");
        RgbaImage::from_pixel(12, 10, image::Rgba([30, 60, 90, 255]))
            .save(&image)
            .unwrap();
        let mut app = QuipIt::default();
        let _ = ctx.run(
            egui::RawInput {
                dropped_files: vec![egui::DroppedFile {
                    path: Some(image.clone()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            |ctx| app.handle_canvas_drop(ctx, egui::Rect::EVERYTHING),
        );
        assert_eq!(app.path.as_ref(), Some(&image));
        assert_eq!(app.pixels.as_ref().unwrap().dimensions(), (12, 10));
        app.receive_dropped_file(image.clone(), &ctx);
        assert_eq!(app.dropped_image.as_ref(), Some(&image));
        app.resolve_dropped_image(false, &ctx);
        assert!(app.inserted.is_some());
        assert_eq!(app.path.as_ref(), Some(&image));
        assert!(!app.pending_open);
        let original = app.pixels.clone();
        app.receive_dropped_file(image.clone(), &ctx);
        app.resolve_dropped_image(true, &ctx);
        assert!(app.pending_open);
        assert!(app.inserted.is_some());
        assert_eq!(app.pixels, original);
        assert_eq!(app.pending_open_path.as_ref(), Some(&image));
        // Save the existing document, then continue directly to the dropped path.
        app.save(false, &ctx);
        assert!(!app.dirty);
        app.continue_document_action(&ctx);
        assert!(app.pending_open_path.is_none() && !app.pending_open);
        assert!(app.inserted.is_none());
        assert_eq!(app.path.as_ref(), Some(&image));
    }

    #[test]
    fn dropping_projects_uses_unsaved_prompt_and_escape_clears_the_target() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path().join("dropped.QuIpIt");
        project::save(&project, &RgbaImage::new(6, 5), &[]).unwrap();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(20, 20)),
            dirty: true,
            ..Default::default()
        };
        app.refresh(&ctx);
        app.place_text(text::Kind::Area, &ctx);
        app.active_text.as_mut().unwrap().content = "Keep my edits".into();
        app.text_dirty = true;
        app.receive_dropped_file(project.clone(), &ctx);
        assert!(app.pending_open && app.dropped_image.is_none());
        text_ui_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!app.pending_open && app.pending_open_path.is_none());
        assert_eq!(app.active_text.as_ref().unwrap().content, "Keep my edits");
        assert_eq!(app.pixels.as_ref().unwrap().dimensions(), (20, 20));
        app.receive_dropped_file(project.clone(), &ctx);
        // Discard follows the same continuation as the existing Open workflow.
        app.continue_document_action(&ctx);
        assert_eq!(app.project_path.as_ref(), Some(&project));
        assert_eq!(app.pixels.as_ref().unwrap().dimensions(), (6, 5));
        assert!(!app.dirty && app.objects.is_empty());
    }

    #[test]
    fn document_title_uses_current_filename_and_abbreviates_long_unicode_names() {
        let mut app = QuipIt::default();
        assert_eq!(app.document_title(), "QuipIt - untitled.qip");
        app.path = Some(PathBuf::from("/images/photo.jpg"));
        assert_eq!(app.document_title(), "QuipIt - photo.jpg");
        app.project_path = Some(PathBuf::from("/projects/caption.qip"));
        assert_eq!(app.document_title(), "QuipIt - caption.qip");
        app.project_path = Some(PathBuf::from(format!("{}.qip", "猫".repeat(60))));
        let title = app.document_title();
        assert!(title.contains('…'));
        assert!(title.ends_with(".qip"));
        assert_eq!(title.strip_prefix("QuipIt - ").unwrap().chars().count(), 48);
    }

    #[test]
    fn edits_restore_pixels_and_invalidate_redo_after_branching() {
        let ctx = egui::Context::default();
        let original = RgbaImage::from_pixel(4, 3, image::Rgba([10, 20, 30, 255]));
        let mut app = QuipIt {
            pixels: Some(original.clone()),
            ..Default::default()
        };
        app.replace(RgbaImage::new(2, 2), &ctx);
        app.history(false, &ctx);
        assert_eq!(app.pixels.as_ref(), Some(&original));
        app.history(true, &ctx);
        assert_eq!(app.pixels.as_ref().unwrap().dimensions(), (2, 2));
        app.history(false, &ctx);
        app.replace(RgbaImage::new(1, 1), &ctx);
        assert!(app.redo.is_empty());
        assert!(app.dirty);
        assert!(app.selection.is_none());
    }

    #[test]
    fn selection_maps_scaled_canvas_and_reverse_drags() {
        let rect = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 100.0));
        let size = egui::vec2(800.0, 400.0);
        assert_eq!(
            selection_from_drag(
                rect,
                size,
                egui::pos2(250.0, 125.0),
                egui::pos2(150.0, 75.0)
            ),
            Some([200, 100, 400, 200])
        );
        assert_eq!(
            selection_from_drag(
                rect,
                size,
                egui::pos2(150.0, 75.0),
                egui::pos2(500.0, 300.0)
            ),
            Some([200, 100, 600, 300])
        );
        assert_eq!(selection_from_drag(rect, size, rect.min, rect.min), None);
    }

    #[test]
    fn crop_preserves_selected_pixels_and_can_be_undone() {
        let ctx = egui::Context::default();
        let original = RgbaImage::from_fn(5, 4, |x, y| image::Rgba([x as u8, y as u8, 0, 255]));
        let mut app = QuipIt {
            pixels: Some(original.clone()),
            selection: Some([1, 2, 3, 2]),
            crop_mode: true,
            ..Default::default()
        };
        app.crop_to_selection(&ctx);
        let cropped = app.pixels.as_ref().unwrap();
        assert_eq!(cropped.dimensions(), (3, 2));
        assert_eq!(cropped.get_pixel(0, 0), original.get_pixel(1, 2));
        assert_eq!(cropped.get_pixel(2, 1), original.get_pixel(3, 3));
        assert!(app.selection.is_none());
        assert!(!app.crop_mode);
        app.history(false, &ctx);
        assert_eq!(app.pixels.as_ref(), Some(&original));
    }

    #[test]
    fn inserted_image_commits_with_alpha_clipping_and_undo() {
        let ctx = egui::Context::default();
        let original = RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 255, 255]));
        let mut inserted = InsertedImage::new(
            RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 128])),
            Vec2::splat(4.0),
            &ctx,
        );
        inserted.rect = egui::Rect::from_min_size(egui::pos2(-1.0, 1.0), Vec2::splat(2.0));
        let mut app = QuipIt {
            pixels: Some(original.clone()),
            inserted: Some(inserted),
            ..Default::default()
        };
        app.commit_insert(&ctx);
        assert_eq!(app.pixels.as_ref(), Some(&original));
        assert_eq!(app.objects.len(), 1);
        let result = app.composed_canvas().unwrap();
        assert_eq!(result.dimensions(), (4, 4));
        assert_eq!(result.get_pixel(0, 1).0, [128, 0, 127, 255]);
        assert_eq!(result.get_pixel(1, 1), original.get_pixel(1, 1));
        assert!(app.inserted.is_none());
        assert!(app.dirty);
        assert_eq!(app.undo.len(), 1);
        app.history(false, &ctx);
        assert_eq!(app.pixels.as_ref(), Some(&original));
        app.history(true, &ctx);
        assert_eq!(
            app.composed_canvas().unwrap().get_pixel(0, 1).0,
            [128, 0, 127, 255]
        );
    }

    #[test]
    fn new_image_is_a_white_unsaved_document_with_no_previous_history() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            new_width: 8,
            new_height: 6,
            path: Some(PathBuf::from("previous.png")),
            undo: vec![EditorState {
                pixels: RgbaImage::new(1, 1),
                ..Default::default()
            }],
            ..Default::default()
        };
        app.new_image(&ctx);
        let pixels = app.pixels.as_ref().unwrap();
        assert_eq!(pixels.dimensions(), (8, 6));
        assert!(pixels.pixels().all(|p| p.0 == [255; 4]));
        assert!(app.path.is_none());
        assert!(app.undo.is_empty());
        assert!(app.dirty);
    }

    #[test]
    fn new_image_uses_selected_color_or_transparent_background() {
        let ctx = egui::Context::default();
        for color in [
            Color32::RED,
            Color32::from_rgb(0, 128, 0),
            Color32::TRANSPARENT,
        ] {
            let mut app = QuipIt {
                new_width: 4,
                new_height: 3,
                new_background: Some(color),
                ..Default::default()
            };
            app.new_image(&ctx);
            assert!(
                app.pixels
                    .as_ref()
                    .unwrap()
                    .pixels()
                    .all(|pixel| pixel.0 == color.to_srgba_unmultiplied())
            );
            assert!(app.dirty);
            assert!(app.path.is_none());
        }
    }

    #[test]
    fn starting_crop_creates_selection_without_changing_pixels() {
        let original = RgbaImage::new(100, 80);
        let mut app = QuipIt {
            pixels: Some(original.clone()),
            ..Default::default()
        };
        app.begin_crop();
        assert_eq!(app.selection, Some([10, 8, 80, 64]));
        assert!(app.crop_mode);
        assert_eq!(app.pixels.as_ref(), Some(&original));
        assert!(!app.dirty);
        assert!(app.undo.is_empty());
    }

    fn press_ctrl(app: &mut QuipIt, ctx: &egui::Context, key: egui::Key) {
        let input = egui::RawInput {
            modifiers: egui::Modifiers::CTRL,
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::CTRL,
            }],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| app.shortcuts(ctx));
    }

    #[test]
    fn ctrl_z_restores_previous_image() {
        let ctx = egui::Context::default();
        let original = RgbaImage::new(4, 3);
        let mut app = QuipIt {
            pixels: Some(original.clone()),
            ..Default::default()
        };
        app.replace(RgbaImage::new(2, 2), &ctx);
        press_ctrl(&mut app, &ctx, egui::Key::Z);
        assert_eq!(app.pixels.as_ref(), Some(&original));
        assert_eq!(app.redo.len(), 1);
    }

    #[test]
    fn ctrl_s_writes_to_existing_filename() {
        let ctx = egui::Context::default();
        let path = std::env::temp_dir().join(format!(
            "quipit-shortcut-{}-{}.qip",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pixels = RgbaImage::from_pixel(2, 3, image::Rgba([20, 40, 60, 255]));
        let mut app = QuipIt {
            pixels: Some(pixels.clone()),
            project_path: Some(path.clone()),
            dirty: true,
            ..Default::default()
        };
        press_ctrl(&mut app, &ctx, egui::Key::S);
        let (saved, _) = project::load(&path, &ctx).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(saved, pixels);
        assert!(!app.dirty);
    }

    #[test]
    fn shape_properties_update_preview_and_accept_as_one_undoable_edit() {
        let ctx = egui::Context::default();
        let original = RgbaImage::from_pixel(100, 80, image::Rgba([255; 4]));
        let mut app = QuipIt {
            pixels: Some(original.clone()),
            ..Default::default()
        };
        app.place_shape(shapes::ShapeKind::Square, &ctx);
        assert_eq!(app.pixels.as_ref(), Some(&original));
        assert!(app.undo.is_empty());
        let style = &mut app.active_shape.as_mut().unwrap().1;
        style.fill = Color32::RED;
        style.border_width = 0.0;
        style.transparency = 50.0;
        app.inserted.as_mut().unwrap().rect =
            egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(40.0, 20.0));
        app.update_shape();
        assert_eq!(app.inserted.as_ref().unwrap().pixels.dimensions(), (40, 20));
        app.commit_insert(&ctx);
        assert_eq!(
            app.composed_canvas().unwrap().get_pixel(30, 20).0,
            [255, 127, 127, 255]
        );
        assert!(app.active_shape.is_none());
        assert_eq!(app.undo.len(), 1);
        app.history(false, &ctx);
        assert_eq!(app.pixels.as_ref(), Some(&original));
    }

    #[test]
    fn color_dialog_previews_and_cancels_without_discarding_shape() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(80, 80)),
            ..Default::default()
        };
        app.place_shape(shapes::ShapeKind::Square, &ctx);
        let original = app.active_shape.unwrap().1;
        app.palette.begin(colors::Target::Fill, original.fill, 0.0);
        let edit = app.palette.edit.as_mut().unwrap();
        edit.color = Color32::RED;
        edit.transparency = 50.0;
        app.preview_color(app.palette.edit.unwrap());
        assert_eq!(
            app.active_shape.unwrap().1.fill,
            Color32::from_rgba_unmultiplied(255, 0, 0, 128)
        );
        app.finish_color(false);
        assert!(app.active_shape.unwrap().1 == original);
        assert!(app.inserted.is_some());
        assert!(app.undo.is_empty());
        assert!(app.palette.edit.is_none());
    }

    #[test]
    fn color_dialog_accepts_image_opacity_and_restores_background_on_cancel() {
        let ctx = egui::Context::default();
        let mut inserted = InsertedImage::new(RgbaImage::new(10, 10), Vec2::splat(50.0), &ctx);
        inserted.image_style = Some(insert::ImageStyle::default());
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(50, 50)),
            inserted: Some(inserted),
            ..Default::default()
        };
        app.palette
            .begin(colors::Target::Image, Color32::WHITE, 0.0);
        app.palette.edit.as_mut().unwrap().transparency = 60.0;
        app.finish_color(true);
        assert_eq!(
            app.inserted
                .as_ref()
                .unwrap()
                .image_style
                .unwrap()
                .image_transparency,
            60.0
        );
        app.new_background = Some(Color32::BLUE);
        app.palette
            .begin(colors::Target::Background, Color32::BLUE, 0.0);
        app.palette.edit.as_mut().unwrap().color = Color32::RED;
        app.preview_color(app.palette.edit.unwrap());
        assert_eq!(app.new_background, Some(Color32::RED));
        app.finish_color(false);
        assert_eq!(app.new_background, Some(Color32::BLUE));
    }

    #[test]
    fn undo_deleted_shape_restores_editable_geometry_properties_and_redo_deletes_it() {
        let ctx = egui::Context::default();
        let original = RgbaImage::new(100, 100);
        let mut app = QuipIt {
            pixels: Some(original.clone()),
            ..Default::default()
        };
        app.place_shape(shapes::ShapeKind::Arrow, &ctx);
        app.active_shape.as_mut().unwrap().1.fill = Color32::RED;
        app.shape_rotation = 35.0;
        {
            let (kind, style) = app.active_shape.as_mut().unwrap();
            shapes::transform_shape(
                *kind,
                style,
                &mut app.inserted.as_mut().unwrap().rect,
                35.0,
                true,
                false,
            );
        }
        let expected_style = app.active_shape.unwrap();
        let expected_rect = app.inserted.as_ref().unwrap().rect;
        app.discard_selection(&ctx);
        assert!(app.inserted.is_none());
        press_ctrl(&mut app, &ctx, egui::Key::Z);
        assert!(app.active_shape == Some(expected_style));
        assert_eq!(app.inserted.as_ref().unwrap().rect, expected_rect);
        assert_eq!(app.shape_rotation, 35.0);
        assert_eq!(app.pixels.as_ref(), Some(&original));
        app.active_shape.as_mut().unwrap().1.border_width = 7.0;
        app.update_shape();
        app.history(true, &ctx);
        assert!(app.inserted.is_none());
        assert!(app.active_shape.is_none());
        app.history(false, &ctx);
        assert_eq!(app.active_shape.unwrap().1.border_width, 7.0);
    }

    #[test]
    fn new_shapes_reuse_last_border_width_including_zero() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(80, 80)),
            ..Default::default()
        };
        app.place_shape(shapes::ShapeKind::Square, &ctx);
        assert_eq!(app.active_shape.unwrap().1.border_width, 3.0);
        app.active_shape.as_mut().unwrap().1.border_width = 9.0;
        app.commit_insert(&ctx);
        app.place_shape(shapes::ShapeKind::Triangle, &ctx);
        assert_eq!(app.active_shape.unwrap().1.border_width, 9.0);
        app.active_shape.as_mut().unwrap().1.border_width = 0.0;
        app.discard_selection(&ctx);
        app.place_shape(shapes::ShapeKind::Circle, &ctx);
        assert_eq!(app.active_shape.unwrap().1.border_width, 0.0);
    }

    #[test]
    fn create_clamps_each_dimension_to_five_thousand() {
        let ctx = egui::Context::default();
        let _ = ctx.run(
            egui::RawInput {
                max_texture_side: Some(8192),
                ..Default::default()
            },
            |_| {},
        );
        for (width, height, expected) in [(6000, 1, (5000, 1)), (1, 6000, (1, 5000))] {
            let mut app = QuipIt {
                new_width: width,
                new_height: height,
                ..Default::default()
            };
            app.new_image(&ctx);
            assert_eq!(app.pixels.as_ref().unwrap().dimensions(), expected);
        }
    }

    #[test]
    fn initial_shape_size_scales_with_canvas_shorter_dimension() {
        assert_eq!(initial_shape_side(1280, 720), 240);
        assert_eq!(initial_shape_side(5000, 5000), 1667);
        assert_eq!(initial_shape_side(2000, 4000), 667);
        assert_eq!(initial_shape_side(1, 1), 1);
    }

    fn text_ui_frame(app: &mut QuipIt, ctx: &egui::Context, events: Vec<egui::Event>) {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1150.0, 780.0),
                )),
                max_texture_side: Some(8192),
                events,
                ..Default::default()
            },
            |ctx| app.ui(ctx),
        );
    }
    #[test]
    fn arrow_keys_move_the_text_caret_without_navigating_to_other_widgets() {
        for kind in [text::Kind::Area, text::Kind::Speech, text::Kind::Thought] {
            let ctx = egui::Context::default();
            let mut app = QuipIt {
                pixels: Some(RgbaImage::from_pixel(800, 600, image::Rgba([255; 4]))),
                ..Default::default()
            };
            app.refresh(&ctx);
            app.place_text(kind, &ctx);
            text_ui_frame(&mut app, &ctx, vec![egui::Event::Text("abc\ndef".into())]);
            text_ui_frame(&mut app, &ctx, vec![]);
            for (key, expected) in [
                (egui::Key::ArrowLeft, 6),
                (
                    egui::Key::ArrowUp,
                    if kind != text::Kind::Thought { 2 } else { 0 },
                ),
                (
                    egui::Key::ArrowRight,
                    if kind != text::Kind::Thought { 3 } else { 1 },
                ),
                (egui::Key::ArrowDown, 7),
            ] {
                text_ui_frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
                assert_eq!(
                    app.active_text.as_ref().unwrap().cursor,
                    expected,
                    "key {key:?}, focus {:?}, carets {:?}",
                    ctx.memory(|m| m.focused()),
                    text::layout(
                        app.active_text.as_ref().unwrap(),
                        app.inserted.as_ref().unwrap().pixels.width(),
                        app.inserted.as_ref().unwrap().pixels.height()
                    )
                    .carets
                );
                assert!(ctx.memory(|memory| memory.has_focus(egui::Id::new("canvas_text_editor"))));
            }
        }
    }

    #[test]
    fn new_text_objects_reuse_typography_but_start_with_fresh_content_and_box_style() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(800, 600)),
            ..Default::default()
        };
        app.place_text(text::Kind::Speech, &ctx);
        let data = app.active_text.as_mut().unwrap();
        data.font = text::mono();
        data.font_name = "Custom font".into();
        data.size = 37.0;
        data.bold = true;
        data.italic = true;
        data.color = Color32::RED;
        data.alignment = 2;
        data.content = "Do not copy".into();
        data.fill = Color32::GREEN;
        data.border = Color32::BLUE;
        data.border_width = 0.0;
        for kind in [text::Kind::Area, text::Kind::Thought, text::Kind::Speech] {
            app.place_text(kind, &ctx);
            let data = app.active_text.as_ref().unwrap();
            assert_eq!(data.font_name, "Custom font");
            assert_eq!(data.size, 37.0);
            assert!(data.bold && data.italic);
            assert_eq!(data.color, Color32::RED);
            assert_eq!(data.alignment, 2);
            assert!(data.content.is_empty());
            assert_eq!(data.fill, Color32::WHITE);
            assert_eq!(data.border, Color32::BLACK);
            if kind == text::Kind::Area {
                assert_eq!(data.border_width, 3.0);
            }
        }
        app.new_width = 400;
        app.new_height = 300;
        app.new_image(&ctx);
        app.place_text(text::Kind::Area, &ctx);
        assert_eq!(app.active_text.as_ref().unwrap().size, 37.0);
    }

    #[test]
    fn shape_shadows_follow_visible_pixels_and_survive_project_round_trip() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("shape-shadow.qip");
        for kind in shapes::ShapeKind::LINES
            .into_iter()
            .chain(shapes::ShapeKind::SHAPES)
        {
            let mut app = QuipIt {
                pixels: Some(RgbaImage::new(240, 200)),
                ..Default::default()
            };
            app.place_shape(kind, &ctx);
            app.active_shape.as_mut().unwrap().1.fill = Color32::TRANSPARENT;
            app.inserted.as_mut().unwrap().rect =
                egui::Rect::from_min_size(egui::pos2(60.0, 50.0), egui::vec2(80.0, 70.0));
            app.update_shape();
            let object = app.inserted.as_ref().unwrap();
            let rect = object.rect;
            let source = object.pixels.clone();
            let mut expected = RgbaImage::new(240, 200);
            let shadow_pixels = RgbaImage::from_fn(source.width(), source.height(), |x, y| {
                image::Rgba([255, 0, 0, source.get_pixel(x, y)[3]])
            });
            image::imageops::overlay(&mut expected, &shadow_pixels, 80, 65);
            object.composite_into(&mut expected);
            let shadow = app.inserted.as_mut().unwrap().image_style.as_mut().unwrap();
            shadow.shadow = true;
            shadow.shadow_color = Color32::RED;
            shadow.shadow_offset = egui::vec2(20.0, 15.0);
            shadow.shadow_blur = 0.0;
            app.update_shape();
            let rendered = app.composed_canvas().unwrap();
            for (actual, expected) in rendered.pixels().zip(expected.pixels()) {
                assert!(
                    actual[3].abs_diff(expected[3]) <= 1
                        && (actual[3] == 0
                            || expected[3] == 0
                            || (0..3)
                                .all(|channel| actual[channel].abs_diff(expected[channel]) <= 1)),
                    "{} silhouette: {actual:?} versus {expected:?}",
                    kind.name()
                );
            }
            assert!(
                rendered.enumerate_pixels().any(|(x, y, pixel)| {
                    !rect.contains(egui::pos2(x as f32, y as f32)) && pixel[3] > 0
                }),
                "{} shadow extends outside selector",
                kind.name()
            );
            assert_eq!(app.inserted.as_ref().unwrap().rect, rect);
            assert_eq!(app.inserted.as_ref().unwrap().pixels, source);
            app.commit_insert(&ctx);
            project::save(&file, app.pixels.as_ref().unwrap(), &app.objects).unwrap();
            let (pixels, objects) = project::load(&file, &ctx).unwrap();
            let mut restored = QuipIt {
                pixels: Some(pixels),
                objects,
                ..Default::default()
            };
            assert_eq!(restored.composed_canvas().unwrap(), rendered);
            restored.edit_object(0, &ctx);
            let shadow = restored.inserted.as_ref().unwrap().image_style.unwrap();
            assert!(shadow.shadow);
            assert_eq!(shadow.shadow_color, Color32::RED);
            assert_eq!(shadow.shadow_offset, egui::vec2(20.0, 15.0));
            assert_eq!(shadow.shadow_blur, 0.0);
            restored
                .inserted
                .as_mut()
                .unwrap()
                .image_style
                .as_mut()
                .unwrap()
                .shadow = false;
            restored.update_shape();
            assert_ne!(restored.composed_canvas().unwrap(), rendered);
        }
    }

    #[test]
    fn older_shapes_gain_shadow_controls_and_shadow_color_edits_can_be_canceled() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(240, 200)),
            ..Default::default()
        };
        app.place_shape(shapes::ShapeKind::Square, &ctx);
        app.commit_insert(&ctx);
        app.objects[0].object.image_style = None;
        app.edit_object(0, &ctx);
        let original = app.inserted.as_ref().unwrap().image_style.unwrap();
        assert!(!original.shadow);
        app.palette
            .begin(colors::Target::ShapeShadow, original.shadow_color, 0.0);
        let edit = app.palette.edit.as_mut().unwrap();
        edit.color = Color32::RED;
        edit.transparency = 50.0;
        app.preview_color(app.palette.edit.unwrap());
        assert_eq!(
            app.inserted
                .as_ref()
                .unwrap()
                .image_style
                .unwrap()
                .shadow_color,
            Color32::from_rgba_unmultiplied(255, 0, 0, 128)
        );
        app.finish_color(false);
        assert!(app.inserted.as_ref().unwrap().image_style.unwrap() == original);
    }

    #[test]
    fn border_shadows_follow_all_text_containers_and_survive_project_round_trip() {
        let ctx = egui::Context::default();
        for kind in [text::Kind::Area, text::Kind::Speech, text::Kind::Thought] {
            let mut app = QuipIt {
                pixels: Some(RgbaImage::new(240, 200)),
                ..Default::default()
            };
            app.place_text(kind, &ctx);
            app.active_text.as_mut().unwrap().fill = Color32::TRANSPARENT;
            app.active_text.as_mut().unwrap().border_width = 3.0;
            let rect = app.inserted.as_ref().unwrap().rect;
            let style = app.inserted.as_mut().unwrap().image_style.as_mut().unwrap();
            style.shadow = true;
            style.shadow_color = Color32::from_black_alpha(180);
            style.shadow_offset = egui::vec2(20.0, 15.0);
            style.shadow_blur = 2.0;
            app.update_shape();
            let empty = app.composed_canvas().unwrap();
            assert!(
                empty
                    .enumerate_pixels()
                    .any(|(x, y, p)| !rect.contains(egui::pos2(x as f32, y as f32)) && p[3] > 0)
            );
            app.active_text.as_mut().unwrap().content = "Container shadow".into();
            app.text_dirty = true;
            app.update_shape();
            let with_text = app.composed_canvas().unwrap();
            for (x, y, pixel) in empty.enumerate_pixels() {
                if !rect.contains(egui::pos2(x as f32, y as f32)) {
                    assert_eq!(pixel, with_text.get_pixel(x, y));
                }
            }
            assert_eq!(app.inserted.as_ref().unwrap().rect, rect);
            app.commit_insert(&ctx);
            let directory = tempfile::tempdir().unwrap();
            let file = directory.path().join("shadow.qip");
            project::save(&file, app.pixels.as_ref().unwrap(), &app.objects).unwrap();
            let (pixels, objects) = project::load(&file, &ctx).unwrap();
            let restored = QuipIt {
                pixels: Some(pixels),
                objects,
                ..Default::default()
            };
            assert_eq!(restored.composed_canvas().unwrap(), with_text);
            assert!(restored.objects[0].object.image_style.unwrap().shadow);
        }
    }

    #[test]
    fn text_objects_move_from_edges_or_with_alt_and_keep_interior_text_selection() {
        for kind in [text::Kind::Area, text::Kind::Speech, text::Kind::Thought] {
            for mode in 0..3 {
                let ctx = egui::Context::default();
                let mut app = QuipIt {
                    pixels: Some(RgbaImage::new(800, 600)),
                    ..Default::default()
                };
                app.refresh(&ctx);
                app.place_text(kind, &ctx);
                app.active_text.as_mut().unwrap().content = "Move or select this text".into();
                app.text_dirty = true;
                text_ui_frame(&mut app, &ctx, vec![]);
                text_ui_frame(&mut app, &ctx, vec![]);
                let original = app.inserted.as_ref().unwrap().rect;
                let (canvas, scale) = app.canvas_rect.unwrap();
                let screen = app.inserted.as_ref().unwrap().screen_rect(canvas, scale);
                let start = if mode == 0 {
                    egui::pos2(screen.left() + 8.0, screen.center().y)
                } else {
                    screen.min + egui::vec2(screen.width() * 0.4, screen.height() * 0.25)
                };
                let modifiers = if mode == 1 {
                    egui::Modifiers::ALT
                } else {
                    egui::Modifiers::NONE
                };
                for (offset, pressed) in [
                    (Vec2::ZERO, Some(true)),
                    (egui::vec2(-25.0, 20.0), None),
                    (egui::vec2(-40.0, 30.0), None),
                    (egui::vec2(-40.0, 30.0), Some(false)),
                ] {
                    let position = start + offset;
                    let mut events = vec![egui::Event::PointerMoved(position)];
                    if let Some(pressed) = pressed {
                        events.push(egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers,
                        });
                    }
                    let _ = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1150.0, 780.0),
                            )),
                            modifiers,
                            events,
                            ..Default::default()
                        },
                        |ctx| app.ui(ctx),
                    );
                }
                let moved = app.inserted.as_ref().unwrap().rect;
                if mode == 2 {
                    assert_eq!(moved, original);
                    assert!(app.active_text.as_ref().unwrap().anchor.is_some());
                } else {
                    assert!(
                        (moved.min - original.min - egui::vec2(-40.0, 30.0) / scale).length() < 0.1
                    );
                    assert!(app.active_text.as_ref().unwrap().anchor.is_none());
                }
            }
        }
    }

    #[test]
    fn canvas_text_typing_click_away_reediting_and_escape_restore_work_together() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::from_pixel(800, 600, image::Rgba([255; 4]))),
            ..Default::default()
        };
        app.refresh(&ctx);
        app.place_text(text::Kind::Area, &ctx);
        text_ui_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("Hello world".into())],
        );
        assert_eq!(app.active_text.as_ref().unwrap().content, "Hello world");
        let (canvas, _) = app.canvas_rect.unwrap();
        let outside = canvas.min + egui::vec2(5.0, 5.0);
        text_ui_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(outside),
                egui::Event::PointerButton {
                    pos: outside,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(app.active_text.is_none());
        assert_eq!(app.objects.len(), 1);
        assert_eq!(app.objects[0].data.as_ref().unwrap().content, "Hello world");
        app.edit_object(0, &ctx);
        text_ui_frame(&mut app, &ctx, vec![egui::Event::Text(" changed".into())]);
        assert_eq!(
            app.active_text.as_ref().unwrap().content,
            "Hello world changed"
        );
        text_ui_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.active_text.is_none());
        assert_eq!(app.objects[0].data.as_ref().unwrap().content, "Hello world");
    }

    #[test]
    fn accepted_text_is_editable_rotatable_exported_and_preserved_by_document_undo() {
        let ctx = egui::Context::default();
        let base = RgbaImage::new(300, 240);
        let mut app = QuipIt {
            pixels: Some(base.clone()),
            ..Default::default()
        };
        app.place_text(text::Kind::Speech, &ctx);
        app.active_text.as_mut().unwrap().content = "Editable balloon".into();
        app.text_dirty = true;
        app.inserted
            .as_mut()
            .unwrap()
            .transform_image(30.0, false, false);
        app.update_shape();
        app.commit_insert(&ctx);
        assert_eq!(app.objects.len(), 1);
        assert_eq!(app.pixels.as_ref(), Some(&base));
        let exported = app.composed_canvas().unwrap();
        assert_ne!(exported, base);
        app.edit_object(0, &ctx);
        app.active_text.as_mut().unwrap().content = "Changed".into();
        app.text_dirty = true;
        app.commit_insert(&ctx);
        assert_eq!(app.objects[0].data.as_ref().unwrap().content, "Changed");
        app.history(false, &ctx);
        assert!(app.active_text.is_some());
        app.discard_selection(&ctx);
        assert_eq!(
            app.objects[0].data.as_ref().unwrap().content,
            "Editable balloon"
        );
        assert_eq!(app.composed_canvas().unwrap(), exported);
    }

    #[test]
    fn text_layer_coordinates_follow_crop_and_undo() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(300, 240)),
            ..Default::default()
        };
        app.place_text(text::Kind::Area, &ctx);
        app.active_text.as_mut().unwrap().content = "Crop".into();
        app.text_dirty = true;
        app.commit_insert(&ctx);
        let original = app.objects[0].object.rect;
        app.selection = Some([10, 20, 200, 150]);
        app.crop_to_selection(&ctx);
        assert_eq!(
            app.objects[0].object.rect,
            original.translate(-egui::vec2(10.0, 20.0))
        );
        app.history(false, &ctx);
        assert_eq!(app.objects[0].object.rect, original);
    }

    #[test]
    fn text_ctrl_enter_wraps_in_place_and_escape_removes_new_object() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(500, 400)),
            ..Default::default()
        };
        app.refresh(&ctx);
        app.place_text(text::Kind::Thought, &ctx);
        text_ui_frame(&mut app, &ctx, vec![egui::Event::Text("First".into())]);
        text_ui_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::CTRL,
            }],
        );
        assert_eq!(app.active_text.as_ref().unwrap().content, "First\n");
        text_ui_frame(&mut app, &ctx, vec![egui::Event::Text("Second".into())]);
        assert_eq!(app.active_text.as_ref().unwrap().content, "First\nSecond");
        text_ui_frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.active_text.is_none() && app.inserted.is_none() && app.objects.is_empty());
    }

    #[test]
    fn png_save_contains_text_and_keeps_session_objects_editable() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(320, 240)),
            ..Default::default()
        };
        app.place_text(text::Kind::Area, &ctx);
        app.active_text.as_mut().unwrap().content = "Saved text".into();
        app.text_dirty = true;
        app.commit_insert(&ctx);
        let expected = app.composed_canvas().unwrap();
        assert!(expected.pixels().any(|p| p[3] > 0));
        let path = std::env::temp_dir().join(format!(
            "quipit-text-export-{}-{}.png",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        app.path = Some(path.clone());
        app.save(false, &ctx);
        let saved = image::open(&path).unwrap().to_rgba8();
        std::fs::remove_file(path).unwrap();
        assert_eq!(saved, expected);
        assert_eq!(app.objects[0].data.as_ref().unwrap().content, "Saved text");
        assert!(!app.dirty);
        app.edit_object(0, &ctx);
        assert_eq!(app.active_text.as_ref().unwrap().content, "Saved text");
    }

    #[test]
    fn save_as_switches_the_default_target_and_failed_saves_keep_it() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::from_pixel(8, 8, image::Rgba([20, 40, 60, 255]))),
            path: Some(directory.path().join("original.png")),
            dirty: true,
            ..Default::default()
        };
        app.save(false, &ctx);
        assert!(!app.dirty);
        let project = directory.path().join("original.qip");
        app.save_document_path(project.clone(), &ctx);
        assert_eq!(app.project_path.as_ref(), Some(&project));
        assert!(app.path.is_none());
        let jpeg = directory.path().join("original.jpg");
        app.save_document_path(jpeg.clone(), &ctx);
        assert!(app.project_path.is_none());
        assert_eq!(app.path.as_ref(), Some(&jpeg));
        assert_eq!(image::open(&jpeg).unwrap().width(), 8);
        app.dirty = true;
        app.save_document_path(directory.path().join("missing/file.png"), &ctx);
        assert!(app.dirty);
        assert!(app.save_error.as_ref().unwrap().contains("file.png"));
        assert_eq!(app.path.as_ref(), Some(&jpeg));
        app.save(false, &ctx);
        assert!(!app.dirty);
        assert!(app.save_error.is_none());
    }

    fn mixed_object_document(ctx: &egui::Context) -> QuipIt {
        let mut app = QuipIt {
            pixels: Some(RgbaImage::from_pixel(120, 100, image::Rgba([255; 4]))),
            ..Default::default()
        };
        app.refresh(ctx);
        app.place_shape(shapes::ShapeKind::Square, ctx);
        app.active_shape.as_mut().unwrap().1.fill = Color32::RED;
        app.active_shape.as_mut().unwrap().1.border_width = 0.0;
        app.inserted.as_mut().unwrap().rect =
            egui::Rect::from_min_size(egui::pos2(10.0, 10.0), Vec2::splat(60.0));
        app.commit_insert(ctx);
        let mut image = InsertedImage::new(
            RgbaImage::from_pixel(40, 40, image::Rgba([0, 0, 255, 255])),
            egui::vec2(120.0, 100.0),
            ctx,
        );
        image.image_style = Some(insert::ImageStyle::default());
        image.rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::splat(40.0));
        app.inserted = Some(image);
        app.commit_insert(ctx);
        app.place_text(text::Kind::Speech, ctx);
        app.active_text.as_mut().unwrap().content = "Editable".into();
        app.active_text.as_mut().unwrap().font = text::mono();
        app.active_text.as_mut().unwrap().font_name = "Embedded custom font".into();
        app.active_text.as_mut().unwrap().shadow = true;
        app.text_dirty = true;
        app.commit_insert(ctx);
        app.objects[0].name = "Red square".into();
        app.objects[2].visible = false;
        app
    }

    #[test]
    fn canvas_selects_frontmost_visible_object() {
        let ctx = egui::Context::default();
        let mut app = mixed_object_document(&ctx);
        text_ui_frame(&mut app, &ctx, vec![]);
        text_ui_frame(&mut app, &ctx, vec![]);
        let (canvas, scale) = app.canvas_rect.unwrap();
        let pointer = canvas.min + egui::vec2(30.0, 30.0) * scale;
        text_ui_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(pointer),
                egui::Event::PointerButton {
                    pos: pointer,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert_eq!(app.object_before.as_ref().unwrap().0, 1);
        assert!(app.active_shape.is_none() && app.active_text.is_none());
        app.commit_insert(&ctx);
        app.objects[1].visible = false;
        app.upload_canvas(&ctx);
        text_ui_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerButton {
                pos: pointer,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        text_ui_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerButton {
                pos: pointer,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(app.active_shape.unwrap().0, shapes::ShapeKind::Square);
    }

    #[test]
    fn mixed_objects_follow_crop_resize_and_undo() {
        let ctx = egui::Context::default();
        let mut app = mixed_object_document(&ctx);
        let original = app.composed_canvas().unwrap();
        let original_rect = app.objects[1].object.rect;
        app.selection = Some([5, 5, 100, 80]);
        app.crop_to_selection(&ctx);
        assert_eq!(
            app.objects[1].object.rect,
            original_rect.translate(egui::vec2(-5.0, -5.0))
        );
        app.width = 200;
        app.height = 160;
        app.resize_canvas(&ctx);
        assert_eq!(app.objects[1].object.pixels.dimensions(), (40, 40));
        assert_eq!(app.objects[1].object.rect.width(), 80.0);
        assert!(app.objects[0].shape.is_some());
        assert!(app.objects[2].data.is_some());
        app.history(false, &ctx);
        app.history(false, &ctx);
        assert_eq!(app.composed_canvas().unwrap(), original);
    }

    #[test]
    fn opening_background_color_does_not_flatten_an_imported_image() {
        let mut pixels = RgbaImage::from_pixel(10, 10, image::Rgba([255; 4]));
        pixels.put_pixel(5, 5, image::Rgba([0, 0, 255, 255]));
        let mut app = QuipIt {
            pixels: Some(pixels.clone()),
            ..Default::default()
        };
        app.begin_background_color();
        app.preview_color(app.palette.edit.unwrap());
        app.finish_color(true);
        assert_eq!(app.pixels.as_ref(), Some(&pixels));
        assert!(!app.dirty);
        assert!(app.undo.is_empty());
    }

    #[test]
    fn background_selection_has_no_movable_object_and_color_changes_are_undoable() {
        let ctx = egui::Context::default();
        let mut app = mixed_object_document(&ctx);
        app.edit_object(0, &ctx);
        app.select_background(&ctx);
        assert!(app.background_selected);
        assert!(app.inserted.is_none() && app.active_text.is_none() && app.active_shape.is_none());
        let objects = app
            .objects
            .iter()
            .map(|o| o.object.rect)
            .collect::<Vec<_>>();
        let original = app.pixels.clone().unwrap();
        app.dirty = false;
        app.begin_background_color();
        app.palette.edit.as_mut().unwrap().color = Color32::GREEN;
        app.palette.edit.as_mut().unwrap().transparency = 50.0;
        app.preview_color(app.palette.edit.unwrap());
        assert_ne!(app.pixels.as_ref().unwrap(), &original);
        app.finish_color(false);
        assert_eq!(app.pixels.as_ref().unwrap(), &original);
        assert!(!app.dirty);
        app.begin_background_color();
        app.palette.edit.as_mut().unwrap().color = Color32::BLUE;
        app.palette.edit.as_mut().unwrap().transparency = 50.0;
        app.finish_color(true);
        assert!(app.dirty);
        assert_eq!(
            app.pixels.as_ref().unwrap().get_pixel(0, 0).0,
            [0, 0, 255, 128]
        );
        assert_eq!(
            app.objects
                .iter()
                .map(|o| o.object.rect)
                .collect::<Vec<_>>(),
            objects
        );
        app.history(false, &ctx);
        assert_eq!(app.pixels.as_ref().unwrap(), &original);
        app.history(true, &ctx);
        assert_eq!(
            app.pixels.as_ref().unwrap().get_pixel(0, 0).0,
            [0, 0, 255, 128]
        );
        app.select_background(&ctx);
        text_ui_frame(&mut app, &ctx, vec![]);
        assert!(app.inserted.is_none());
        app.place_text(text::Kind::Area, &ctx);
        assert!(!app.background_selected);
    }

    #[test]
    fn list_actions_reorder_toggle_duplicate_and_delete_without_revealing_hidden_objects() {
        let ctx = egui::Context::default();
        let mut app = mixed_object_document(&ctx);
        let hidden_name = app.objects[2].name.clone();
        app.edit_object(0, &ctx);
        // Drag the hidden top object below the selected bottom object.
        app.object_list_action(2, "reorder", 0, &ctx);
        assert_eq!(app.objects[0].name, hidden_name);
        assert!(!app.objects[0].visible);
        assert_eq!(app.objects[1].name, "Red square");
        app.history(false, &ctx);
        assert_eq!(app.objects[0].name, "Red square");
        assert_eq!(app.objects[2].name, hidden_name);
        app.object_list_action(2, "duplicate", 2, &ctx);
        assert_eq!(app.objects[3].name, format!("{hidden_name} copy"));
        assert!(!app.objects[3].visible);
        app.object_list_action(3, "visibility", 3, &ctx);
        assert!(app.objects[3].visible);
        app.object_list_action(3, "back", 0, &ctx);
        assert_eq!(app.objects[0].name, format!("{hidden_name} copy"));
        app.object_list_action(0, "forward", 0, &ctx);
        assert_eq!(app.objects[1].name, format!("{hidden_name} copy"));
        app.object_list_action(1, "backward", 0, &ctx);
        app.object_list_action(0, "front", 0, &ctx);
        assert_eq!(app.objects[3].name, format!("{hidden_name} copy"));
        app.object_list_action(3, "delete", 3, &ctx);
        assert_eq!(app.objects.len(), 3);
        app.history(false, &ctx);
        assert_eq!(app.objects.len(), 4);
        assert!(app.objects[3].visible);
    }

    #[test]
    fn mixed_objects_keep_depth_while_editing_and_support_reordering() {
        let ctx = egui::Context::default();
        let mut app = mixed_object_document(&ctx);
        let expected = app.composed_canvas().unwrap();
        assert_eq!(expected.get_pixel(30, 30).0, [0, 0, 255, 255]);
        app.edit_object(0, &ctx);
        assert!(app.active_shape.is_some());
        assert_eq!(app.composed_canvas().unwrap(), expected);
        app.arrange_object("front", &ctx);
        assert_eq!(
            app.composed_canvas().unwrap().get_pixel(30, 30).0,
            [255, 0, 0, 255]
        );
        app.history(false, &ctx);
        assert_eq!(app.composed_canvas().unwrap(), expected);
        app.edit_object(1, &ctx);
        assert!(app.inserted.as_ref().unwrap().image_style.is_some());
        app.inserted
            .as_mut()
            .unwrap()
            .image_style
            .as_mut()
            .unwrap()
            .image_transparency = 50.0;
        app.discard_selection(&ctx);
        app.upload_canvas(&ctx);
        assert_eq!(app.composed_canvas().unwrap(), expected);
    }

    #[test]
    fn object_duplication_deletion_and_undo_preserve_sources() {
        let ctx = egui::Context::default();
        let mut app = mixed_object_document(&ctx);
        app.edit_object(0, &ctx);
        app.arrange_object("duplicate", &ctx);
        assert_eq!(app.objects.len(), 3); // One of four objects is being edited.
        assert_eq!(
            app.object_before.as_ref().unwrap().1.name,
            "Red square copy"
        );
        app.active_shape.as_mut().unwrap().1.fill = Color32::GREEN;
        app.commit_insert(&ctx);
        assert_eq!(app.objects[0].shape.unwrap().1.fill, Color32::RED);
        assert_eq!(app.objects[1].shape.unwrap().1.fill, Color32::GREEN);
        app.edit_object(1, &ctx);
        app.delete_object(&ctx);
        assert_eq!(app.objects.len(), 3);
        assert!(app.inserted.is_none());
        app.history(false, &ctx);
        assert_eq!(app.active_shape.unwrap().1.fill, Color32::GREEN);
        assert!(app.inserted.is_some());
    }

    #[test]
    fn project_round_trip_preserves_mixed_objects_and_active_edits() {
        let ctx = egui::Context::default();
        let mut app = mixed_object_document(&ctx);
        app.edit_object(1, &ctx);
        app.inserted
            .as_mut()
            .unwrap()
            .transform_image(30.0, true, false);
        let style = app.inserted.as_mut().unwrap().image_style.as_mut().unwrap();
        style.shadow = true;
        style.shadow_color = Color32::from_rgba_unmultiplied(12, 34, 56, 150);
        style.shadow_offset = egui::vec2(-5.0, 7.0);
        style.shadow_blur = 3.0;
        app.shape_rotation = 30.0;
        app.update_shape();
        let expected = app.composed_canvas().unwrap();
        let path = std::env::temp_dir().join(format!("quipit-project-{}.qip", std::process::id()));
        app.project_path = Some(path.clone());
        app.save(false, &ctx);
        assert!(!app.dirty, "{}", app.status);
        let (pixels, objects) = project::load(&path, &ctx).unwrap();
        std::fs::remove_file(path).unwrap();
        let mut restored = QuipIt {
            pixels: Some(pixels),
            objects,
            ..Default::default()
        };
        let shadow = restored.objects[1].object.image_style.unwrap();
        assert!(shadow.shadow);
        assert_eq!(shadow.shadow_offset, egui::vec2(-5.0, 7.0));
        assert_eq!(shadow.shadow_blur, 3.0);
        assert_eq!(restored.objects.len(), 3);
        assert_eq!(restored.objects[0].name, "Red square");
        assert_eq!(
            restored.objects[0].shape.unwrap().0,
            shapes::ShapeKind::Square
        );
        assert!(!restored.objects[2].visible);
        assert_eq!(
            restored.objects[2].data.as_ref().unwrap().font_name,
            "Embedded custom font"
        );
        assert!(restored.objects[2].data.as_ref().unwrap().shadow);
        assert_eq!(
            restored.objects[2].data.as_ref().unwrap().content,
            "Editable"
        );
        assert_eq!(restored.composed_canvas().unwrap(), expected);
        restored.edit_object(2, &ctx);
        assert_eq!(restored.active_text.as_ref().unwrap().content, "Editable");
        restored.active_text.as_mut().unwrap().content = "Reopened".into();
        restored.text_dirty = true;
        restored.commit_insert(&ctx);
        assert_eq!(
            restored.objects[2].data.as_ref().unwrap().content,
            "Reopened"
        );
    }

    #[test]
    fn approved_document_actions_finish_text_edits() {
        for closing in [false, true] {
            for kind in [text::Kind::Area, text::Kind::Speech, text::Kind::Thought] {
                let ctx = egui::Context::default();
                let mut app = QuipIt {
                    pixels: Some(RgbaImage::new(500, 400)),
                    ..Default::default()
                };
                app.refresh(&ctx);
                app.place_text(kind, &ctx);
                app.active_text.as_mut().unwrap().content = "Pending edit".into();
                app.text_dirty = true;
                app.pending_new = !closing;
                app.confirm_close = closing;
                app.continue_document_action(&ctx);
                assert!(app.active_text.is_none());
                assert!(app.inserted.is_none());
                assert!(!app.pending_new && !app.confirm_close);
                assert_eq!(
                    app.objects[0].data.as_ref().unwrap().content,
                    "Pending edit"
                );
                assert_eq!(app.show_new, !closing);
                assert_eq!(app.close_approved, closing);
                if closing {
                    assert!(!app.dirty);
                }
                text_ui_frame(&mut app, &ctx, vec![]);
                assert!(app.active_text.is_none());
                assert!(!app.confirm_close);
            }
        }
    }

    #[test]
    fn canceling_document_replacement_preserves_active_edits() {
        for tool in 0..3 {
            let ctx = egui::Context::default();
            let mut app = QuipIt {
                pixels: Some(RgbaImage::new(500, 400)),
                ..Default::default()
            };
            app.refresh(&ctx);
            match tool {
                0 => {
                    app.place_text(text::Kind::Speech, &ctx);
                    app.active_text.as_mut().unwrap().content = "Keep my edits".into();
                    app.text_dirty = true;
                }
                1 => app.place_shape(shapes::ShapeKind::Triangle, &ctx),
                _ => {
                    let mut object =
                        InsertedImage::new(RgbaImage::new(80, 60), egui::vec2(500.0, 400.0), &ctx);
                    object.image_style = Some(insert::ImageStyle::default());
                    app.inserted = Some(object);
                }
            }
            app.update_shape();
            let rect = app.inserted.as_ref().unwrap().rect;
            let pixels = app.pixels.clone();
            let undo_count = app.undo.len();
            for dialog in 0..3 {
                app.pending_new = dialog == 0;
                app.pending_open = dialog == 1;
                app.show_new = dialog == 2;
                text_ui_frame(&mut app, &ctx, vec![]);
                text_ui_frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
                assert!(!app.pending_new && !app.pending_open && !app.show_new);
                assert_eq!(app.inserted.as_ref().unwrap().rect, rect);
                assert_eq!(app.pixels, pixels);
                assert_eq!(app.undo.len(), undo_count);
                if tool == 0 {
                    assert_eq!(app.active_text.as_ref().unwrap().content, "Keep my edits");
                }
            }
            app.new_width = 100;
            app.new_height = 100;
            app.new_image(&ctx);
            assert!(
                app.inserted.is_none() && app.active_text.is_none() && app.active_shape.is_none()
            );
        }
    }

    #[test]
    fn history_is_bounded() {
        let ctx = egui::Context::default();
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(1, 1)),
            ..Default::default()
        };
        for _ in 0..15 {
            app.replace(RgbaImage::new(1, 1), &ctx);
        }
        assert_eq!(app.undo.len(), 10);
    }
}
