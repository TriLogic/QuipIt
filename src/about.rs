use crate::{QuipIt, toolbar};
use eframe::egui::{self, TextureHandle, Vec2};

impl QuipIt {
    fn application_icon(&mut self, ctx: &egui::Context) -> TextureHandle {
        self.program_icon
            .get_or_insert_with(|| {
                let pixels = image::load_from_memory(include_bytes!("../assets/quipit-256.png"))
                    .expect("Bundled application icon must be valid")
                    .into_rgba8();
                ctx.load_texture(
                    "quipit_application_icon",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [pixels.width() as usize, pixels.height() as usize],
                        pixels.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                )
            })
            .clone()
    }

    pub(crate) fn about_button(&mut self, ui: &mut egui::Ui) {
        if toolbar::button(ui, toolbar::Icon::About, "About QuipIt", true, false).clicked() {
            self.show_about = true;
        }
    }

    pub(crate) fn about_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        let icon = self.application_icon(ctx);
        let response = egui::Modal::new(egui::Id::new("about_quipit")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.vertical_centered(|ui| {
                ui.image((icon.id(), Vec2::splat(80.0)));
                ui.heading("QuipIt");
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
            });
            ui.add_space(12.0);
            ui.label(
                "An image editor for captions, speech balloons, shapes, and quick visual edits.",
            );
            ui.add_space(6.0);
            ui.label("Save editable QuipIt projects or flattened images in your chosen format.");
            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                if ui.button("OK").clicked() {
                    self.show_about = false;
                }
            });
        });
        if response.should_close() {
            self.show_about = false;
        }
    }
}
