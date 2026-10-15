use eframe::egui::{self, Color32, Rect};

#[derive(Clone, Copy, PartialEq)]
pub enum Target {
    Border,
    Fill,
    ImageBorder,
    Image,
    Background,
    TextColor,
    TextBorder,
    TextFill,
    TextShadow,
}
#[derive(Clone, Copy)]
pub struct Edit {
    pub target: Target,
    pub original_color: Color32,
    pub original_transparency: f32,
    pub color: Color32,
    pub transparency: f32,
}
#[derive(Default)]
pub struct Palette {
    pub picking: bool,
    recent: [Option<Color32>; 2],
    pub edit: Option<Edit>,
    pub custom_original: Option<Color32>,
    custom_transparency: Option<f32>,
}
impl Palette {
    pub fn remember(&mut self, color: Color32) {
        if self.recent[0] != Some(color) {
            self.recent = [Some(color), self.recent[0]];
        }
    }
    pub fn begin(&mut self, target: Target, color: Color32, transparency: f32) {
        self.picking = false;
        self.custom_original = None;
        self.custom_transparency = None;
        self.edit = Some(Edit {
            target,
            original_color: color,
            original_transparency: transparency,
            color: opaque(color),
            transparency: 100.0 * (1.0 - color.a() as f32 / 255.0 * (1.0 - transparency / 100.0)),
        });
    }
    pub fn finish_custom(&mut self, accept: bool) {
        if let Some(original) = self.custom_original.take()
            && !accept
            && let Some(edit) = &mut self.edit
        {
            edit.color = original;
            if let Some(t) = self.custom_transparency {
                edit.transparency = t;
            }
        }
        self.custom_transparency = None;
    }
    pub fn dialog(&mut self, ctx: &egui::Context) -> Option<bool> {
        let custom_was_open = self.custom_original.is_some();
        let enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let edit = self.edit.as_mut()?;
        let before = (edit.color, edit.transparency);
        let mut open = true;
        let mut decision = if enter && !custom_was_open {
            Some(true)
        } else {
            None
        };
        let title = match edit.target {
            Target::Border => "Border",
            Target::Fill => "Fill",
            Target::ImageBorder => "Image border",
            Target::Image => "Image",
            Target::Background => "Background",
            Target::TextColor => "Text",
            Target::TextBorder => "Text border",
            Target::TextFill => "Text background",
            Target::TextShadow => "Text shadow",
        };
        let title = if edit.target == Target::Background {
            "Background color".to_owned()
        } else {
            format!("{title} color and transparency")
        };
        egui::Window::new(title)
            .id(egui::Id::new("color_dialog"))
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if custom_was_open {
                    ui.disable();
                }
                if edit.target != Target::Image {
                    if selector(ui, "Custom color…", edit.color).clicked() {
                        self.custom_original = Some(edit.color);
                        self.custom_transparency = Some(edit.transparency);
                        if edit.target == Target::Background {
                            edit.transparency = 0.0;
                        }
                        self.picking = false;
                    }
                    ui.label("Basic colors");
                    if let Some(color) = basic_palette(ui, edit.color) {
                        edit.color = color;
                        edit.transparency = 0.0;
                    }
                    if transparent_button(ui, edit.transparency == 100.0 || edit.color.a() == 0)
                        .clicked()
                    {
                        edit.transparency = 100.0;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Recent");
                        for (index, color) in self.recent.into_iter().enumerate() {
                            ui.push_id(index, |ui| {
                                if let Some(color) = color
                                    && swatch(ui, color, "Recent color").clicked()
                                {
                                    edit.color = opaque(color);
                                    edit.transparency = if edit.target == Target::Background {
                                        if color.a() == 0 { 100.0 } else { 0.0 }
                                    } else {
                                        100.0 * (1.0 - color.a() as f32 / 255.0)
                                    };
                                }
                            });
                        }
                    });
                    if ui
                        .selectable_label(self.picking, "Pick color from image")
                        .clicked()
                    {
                        self.picking = !self.picking;
                    }
                    if self.picking {
                        ui.label("Click the image to sample its underlying color.");
                    }
                }
                if edit.target != Target::Background {
                    ui.label("Transparency");
                    ui.add(egui::Slider::new(&mut edit.transparency, 0.0..=100.0).suffix("%"));
                }
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        decision = Some(false);
                    }
                    if ui.button("OK").clicked() {
                        decision = Some(true);
                    }
                });
            });
        if self.custom_original.is_some() {
            let mut result = if enter && custom_was_open {
                Some(true)
            } else {
                None
            };
            let modal = egui::Modal::new(egui::Id::new("custom_color_picker")).show(ctx, |ui| {
                ui.heading("Custom color");
                egui::color_picker::color_picker_color32(
                    ui,
                    &mut edit.color,
                    egui::color_picker::Alpha::Opaque,
                );
                ui.separator();
                let mut rgb = [edit.color.r(), edit.color.g(), edit.color.b()];
                egui::Grid::new("custom_rgb_channels")
                    .num_columns(3)
                    .show(ui, |ui| {
                        for (label, channel) in ["Red", "Green", "Blue"].into_iter().zip(&mut rgb) {
                            ui.label(label);
                            ui.add(egui::Slider::new(channel, 0..=255).show_value(false));
                            ui.add(egui::DragValue::new(channel).range(0..=255));
                            ui.end_row();
                        }
                    });
                edit.color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        result = Some(false);
                    }
                    if ui.button("OK").clicked() {
                        result = Some(true);
                    }
                });
            });
            if modal.should_close() {
                result = Some(false);
            }
            if let Some(accept) = result {
                self.finish_custom(accept);
            }
        }
        if !open {
            decision = Some(false);
        }
        if self
            .edit
            .is_some_and(|edit| (edit.color, edit.transparency) != before)
            || decision.is_some()
        {
            ctx.request_repaint();
        }
        decision
    }
}
pub fn opaque(color: Color32) -> Color32 {
    let [r, g, b, _] = color.to_srgba_unmultiplied();
    Color32::from_rgb(r, g, b)
}
pub fn with_transparency(color: Color32, transparency: f32) -> Color32 {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(
        r,
        g,
        b,
        (a as f32 * (1.0 - transparency / 100.0)).round() as u8,
    )
}
fn checker(ui: &egui::Ui, rect: Rect) {
    for row in 0..2 {
        for col in 0..3 {
            ui.painter().rect_filled(
                Rect::from_min_size(
                    rect.min
                        + egui::vec2(
                            col as f32 * rect.width() / 3.0,
                            row as f32 * rect.height() / 2.0,
                        ),
                    egui::vec2(rect.width() / 3.0, rect.height() / 2.0),
                ),
                0.0,
                if (row + col) % 2 == 0 {
                    Color32::WHITE
                } else {
                    Color32::from_gray(160)
                },
            );
        }
    }
}
pub fn selector(ui: &mut egui::Ui, label: &str, color: Color32) -> egui::Response {
    let response = ui.add(
        egui::Button::new("")
            .min_size(egui::vec2(200.0, 30.0))
            .stroke(egui::Stroke::new(1.0, ui.visuals().text_color())),
    );
    let rect = Rect::from_min_size(
        egui::pos2(response.rect.left() + 7.0, response.rect.center().y - 9.0),
        egui::vec2(28.0, 18.0),
    );
    if color.a() < 255 {
        checker(ui, rect);
    }
    ui.painter().rect_filled(rect, 0.0, color);
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, ui.visuals().text_color()),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.right() + 12.0, response.rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        ui.style().interact(&response).fg_stroke.color,
    );
    response.on_hover_text("Edit color and transparency")
}
pub fn image_selector(ui: &mut egui::Ui, texture: egui::TextureId) -> egui::Response {
    let response = selector(ui, "Image transparency", Color32::TRANSPARENT);
    let rect = Rect::from_min_size(
        egui::pos2(response.rect.left() + 7.0, response.rect.center().y - 9.0),
        egui::vec2(28.0, 18.0),
    );
    ui.painter().image(
        texture,
        rect,
        Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    response
}
pub fn transparent_button(ui: &mut egui::Ui, selected: bool) -> egui::Response {
    let response = ui.add(
        egui::Button::new("       Transparent")
            .min_size(egui::vec2(160.0, 28.0))
            .stroke(if selected {
                ui.visuals().selection.stroke
            } else {
                egui::Stroke::new(1.0, ui.visuals().text_color())
            })
            .selected(selected),
    );
    let rect = Rect::from_min_size(
        egui::pos2(response.rect.left() + 8.0, response.rect.center().y - 7.0),
        egui::Vec2::splat(14.0),
    );
    checker(ui, rect);
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, ui.visuals().text_color()),
        egui::StrokeKind::Inside,
    );
    response.on_hover_text("Transparent: no background color")
}
pub fn basic_palette(ui: &mut egui::Ui, current: Color32) -> Option<Color32> {
    let colors = [
        ("Black", 0, 0, 0),
        ("White", 255, 255, 255),
        ("Gray", 128, 128, 128),
        ("Silver", 192, 192, 192),
        ("Red", 255, 0, 0),
        ("Maroon", 128, 0, 0),
        ("Orange", 255, 165, 0),
        ("Brown", 139, 69, 19),
        ("Yellow", 255, 255, 0),
        ("Lime", 0, 255, 0),
        ("Green", 0, 128, 0),
        ("Olive", 128, 128, 0),
        ("Cyan", 0, 255, 255),
        ("Teal", 0, 128, 128),
        ("Blue", 0, 0, 255),
        ("Navy", 0, 0, 128),
        ("Purple", 128, 0, 128),
        ("Magenta", 255, 0, 255),
        ("Pink", 255, 192, 203),
        ("Gold", 255, 215, 0),
    ];
    let mut chosen = None;
    egui::Grid::new("basic_colors")
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for (i, (name, r, g, b)) in colors.into_iter().enumerate() {
                let color = Color32::from_rgb(r, g, b);
                let response = swatch(ui, color, name);
                if color == current {
                    ui.painter().rect_stroke(
                        response.rect,
                        2.0,
                        egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Inside,
                    );
                }
                if response.clicked() {
                    chosen = Some(color);
                }
                if (i + 1) % 5 == 0 {
                    ui.end_row();
                }
            }
        });
    chosen
}
fn swatch(ui: &mut egui::Ui, color: Color32, name: &str) -> egui::Response {
    let response = ui.add(egui::Button::new("").min_size(egui::Vec2::splat(24.0)));
    let rect = response.rect.shrink(3.0);
    if color.a() < 255 {
        checker(ui, rect);
    }
    ui.painter().rect_filled(rect, 0.0, color);
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, ui.visuals().text_color()),
        egui::StrokeKind::Inside,
    );
    response.on_hover_text(name)
}
pub fn sample(image: &image::RgbaImage, rect: Rect, point: egui::Pos2) -> Option<Color32> {
    if !rect.contains(point) || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let p = (point - rect.min) / rect.size();
    let x = ((p.x * image.width() as f32).floor() as u32).min(image.width() - 1);
    let y = ((p.y * image.height() as f32).floor() as u32).min(image.height() - 1);
    let [r, g, b, a] = image.get_pixel(x, y).0;
    Some(Color32::from_rgba_unmultiplied(r, g, b, a))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recent_colors_are_unique_and_new_color_edits_keep_original_values() {
        let mut p = Palette::default();
        p.remember(Color32::RED);
        p.remember(Color32::RED);
        p.remember(Color32::BLUE);
        assert_eq!(p.recent, [Some(Color32::BLUE), Some(Color32::RED)]);
        p.begin(Target::Border, Color32::RED, 25.0);
        p.edit.as_mut().unwrap().color = Color32::GREEN;
        assert_eq!(p.edit.unwrap().original_color, Color32::RED);
        assert_eq!(p.edit.unwrap().original_transparency, 25.0);
    }
    #[test]
    fn canceling_custom_picker_restores_only_its_color_draft() {
        let mut p = Palette::default();
        p.begin(Target::Fill, Color32::RED, 35.0);
        let transparency = p.edit.unwrap().transparency;
        p.custom_original = Some(Color32::RED);
        p.edit.as_mut().unwrap().color = Color32::BLUE;
        p.finish_custom(false);
        assert_eq!(p.edit.unwrap().color, Color32::RED);
        assert_eq!(p.edit.unwrap().transparency, transparency);
    }
    #[test]
    fn enter_accepts_the_innermost_color_dialog() {
        let mut p = Palette::default();
        p.begin(Target::Fill, Color32::RED, 0.0);
        p.custom_original = Some(Color32::RED);
        p.edit.as_mut().unwrap().color = Color32::BLUE;
        let input = || egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut decision = None;
        let _ = egui::Context::default().run(input(), |ctx| decision = p.dialog(ctx));
        assert_eq!(decision, None);
        assert!(p.custom_original.is_none());
        let _ = egui::Context::default().run(input(), |ctx| decision = p.dialog(ctx));
        assert_eq!(decision, Some(true));
    }
    #[test]
    fn eyedropper_maps_scaled_canvas_to_pixels_and_rejects_outside_clicks() {
        let image = image::RgbaImage::from_fn(4, 2, |x, y| {
            image::Rgba([x as u8 * 50, y as u8 * 100, 0, 255])
        });
        let rect = Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(200.0, 100.0));
        assert_eq!(
            sample(&image, rect, egui::pos2(195.0, 105.0)),
            Some(Color32::from_rgb(150, 100, 0))
        );
        assert_eq!(sample(&image, rect, egui::pos2(5.0, 5.0)), None);
    }
}
