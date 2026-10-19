use crate::QuipIt;
use eframe::egui;
use std::path::PathBuf;

const FORMATS: &[(&str, &str)] = &[
    ("QuipIt project (.qip)", "qip"),
    ("PNG image (.png)", "png"),
    ("JPEG image (.jpg)", "jpg"),
    ("WebP image (.webp)", "webp"),
    ("BMP image (.bmp)", "bmp"),
    ("GIF image (.gif)", "gif"),
    ("TIFF image (.tiff)", "tiff"),
    ("ICO image (.ico)", "ico"),
    ("AVIF image (.avif)", "avif"),
];

#[derive(Clone)]
pub(crate) struct SaveAsDialog {
    folder: String,
    name: String,
    format: usize,
    continue_action: bool,
}

impl QuipIt {
    pub(crate) fn begin_save_as(&mut self) {
        let current = self.project_path.as_ref().or(self.path.as_ref());
        let path = current
            .cloned()
            .unwrap_or_else(|| PathBuf::from("untitled.qip"));
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("qip");
        let extension = match extension.to_ascii_lowercase().as_str() {
            "jpeg" => "jpg".to_owned(),
            "tif" => "tiff".to_owned(),
            "quipit" => "qip".to_owned(),
            other => other.to_owned(),
        };
        let format = FORMATS
            .iter()
            .position(|(_, ext)| *ext == extension)
            .unwrap_or(1);
        let path = Self::save_as_target(path, Some(FORMATS[format].1));
        self.save_as_dialog = Some(SaveAsDialog {
            folder: path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| {
                    std::env::current_dir()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                }),
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            format,
            continue_action: self.pending_open || self.pending_new || self.confirm_close,
        });
    }

    pub(crate) fn show_save_as_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.save_as_dialog.take() else {
            return;
        };
        let mut save = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("save_as_dialog")).show(ctx, |ui| {
            ui.set_width(440.0);
            ui.heading("Save As");
            ui.label("File name");
            ui.text_edit_singleline(&mut dialog.name);
            ui.label("Folder");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut dialog.folder);
                if ui.button("Browse…").clicked()
                    && let Some(folder) = rfd::FileDialog::new()
                        .set_directory(&dialog.folder)
                        .pick_folder()
                {
                    dialog.folder = folder.to_string_lossy().into_owned();
                }
            });
            let previous_format = dialog.format;
            egui::ComboBox::from_label("Format")
                .selected_text(FORMATS[dialog.format].0)
                .show_ui(ui, |ui| {
                    for (index, (label, _)) in FORMATS.iter().enumerate() {
                        ui.selectable_value(&mut dialog.format, index, *label);
                    }
                });
            if previous_format != dialog.format {
                dialog.name = Self::save_as_target(
                    PathBuf::from(&dialog.name),
                    Some(FORMATS[dialog.format].1),
                )
                .to_string_lossy()
                .into_owned();
            }
            ui.horizontal(|ui| {
                save = ui
                    .add_enabled(!dialog.name.trim().is_empty(), egui::Button::new("Save"))
                    .clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        cancel |= ctx.input(|input| input.key_pressed(egui::Key::Escape));
        if cancel {
            return;
        }
        if save {
            let target = Self::save_as_target(
                PathBuf::from(&dialog.folder).join(&dialog.name),
                Some(FORMATS[dialog.format].1),
            );
            let replace = !target.exists()
                || rfd::MessageDialog::new()
                    .set_title("Replace existing file?")
                    .set_description(format!("{} already exists. Replace it?", target.display()))
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show()
                    == rfd::MessageDialogResult::Yes;
            if replace {
                self.save_document_path(target, ctx);
                if self.save_error.is_none() {
                    let status = self.status.clone();
                    self.commit_insert(ctx);
                    self.status = status;
                    self.dirty = false;
                    if dialog.continue_action {
                        self.continue_document_action(ctx);
                    }
                    return;
                }
            }
        }
        self.save_as_dialog = Some(dialog);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn frame(app: &mut QuipIt, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1150.0, 780.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.show_save_as_dialog(ctx),
        )
    }

    fn click(app: &mut QuipIt, ctx: &egui::Context, label: &str) {
        let _ = frame(app, ctx, vec![]);
        let output = frame(app, ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == label
                {
                    return Some(text.pos + text.galley.size() * 0.5);
                }
                None
            })
            .unwrap_or_else(|| panic!("Missing {label}"));
        for pressed in [true, false] {
            frame(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }

    #[test]
    fn save_as_completes_before_pending_open_and_cancel_preserves_the_document() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let next = directory.path().join("next.png");
        RgbaImage::new(12, 10).save(&next).unwrap();
        let original = RgbaImage::from_pixel(100, 80, Rgba([20, 40, 60, 255]));
        let mut app = QuipIt {
            pixels: Some(original.clone()),
            dirty: true,
            ..Default::default()
        };
        app.request_open_path(next.clone(), &ctx);
        app.save(false, &ctx);
        click(&mut app, &ctx, "Cancel");
        assert_eq!(app.pixels.as_ref(), Some(&original));
        assert!(app.pending_open && app.dirty);
        app.save(false, &ctx);
        let dialog = app.save_as_dialog.as_mut().unwrap();
        dialog.folder = directory.path().to_string_lossy().into_owned();
        dialog.name = "preserved.qip".into();
        click(&mut app, &ctx, "Save");
        let (saved, _) =
            crate::project::load(&directory.path().join("preserved.qip"), &ctx).unwrap();
        assert_eq!(saved, original);
        assert_eq!(app.path.as_ref(), Some(&next));
        assert!(!app.pending_open && !app.dirty);
    }

    #[test]
    fn dialog_dropdown_changes_extension_and_saves_flattened_webp() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("caption.png");
        let mut app = QuipIt {
            pixels: Some(RgbaImage::from_pixel(320, 240, Rgba([255, 255, 255, 255]))),
            path: Some(source),
            ..Default::default()
        };
        app.place_text(crate::text::Kind::Area, &ctx);
        app.active_text.as_mut().unwrap().content = "Save As dropdown".into();
        app.text_dirty = true;
        app.update_shape();
        let expected = app.composed_canvas().unwrap();
        app.begin_save_as();
        assert_eq!(app.save_as_dialog.as_ref().unwrap().name, "caption.png");
        click(&mut app, &ctx, "PNG image (.png)");
        click(&mut app, &ctx, "WebP image (.webp)");
        assert_eq!(app.save_as_dialog.as_ref().unwrap().name, "caption.webp");
        click(&mut app, &ctx, "Save");
        let target = directory.path().join("caption.webp");
        assert_eq!(image::open(&target).unwrap().to_rgba8(), expected);
        assert_eq!(app.path.as_ref(), Some(&target));
        assert!(app.save_as_dialog.is_none());
        assert!(!app.dirty);
    }

    #[test]
    fn qoi_save_is_rejected_and_save_dialog_offers_png_instead() {
        let ctx = egui::Context::default();
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("original.qoi");
        let mut app = QuipIt {
            pixels: Some(RgbaImage::new(5, 5)),
            path: Some(source.clone()),
            dirty: true,
            ..Default::default()
        };
        app.save_document_path(source.clone(), &ctx);
        assert!(app.save_error.as_ref().unwrap().contains("QOI"));
        assert!(!source.exists());
        app.save_error = None;
        app.save(false, &ctx);
        assert_eq!(app.save_as_dialog.as_ref().unwrap().name, "original.png");
        assert!(!FORMATS.iter().any(|(_, ext)| *ext == "qoi"));
        click(&mut app, &ctx, "Cancel");
        assert!(app.save_as_dialog.is_none() && app.dirty);
        assert_eq!(app.path.as_ref(), Some(&source));
    }
}
