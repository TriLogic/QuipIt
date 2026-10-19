use ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont};
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use image::{GrayImage, RgbaImage};

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Area,
    Speech,
    Thought,
}
impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Area => "Text rectangle",
            Self::Speech => "Speech balloon",
            Self::Thought => "Thought bubble",
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct TextData {
    pub kind: Kind,
    pub content: String,
    #[serde(with = "font_serde")]
    pub font: FontArc,
    pub font_name: String,
    #[serde(skip)]
    font_error: Option<String>,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub color: Color32,
    pub border: Color32,
    pub border_width: f32,
    pub fill: Color32,
    pub shadow: bool,
    pub shadow_color: Color32,
    pub shadow_offset: Vec2,
    pub shadow_blur: f32,
    pub alignment: u8,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub cursor: usize,
    pub anchor: Option<usize>,
    #[serde(skip)]
    pub undo: Vec<(String, usize)>,
    #[serde(skip)]
    pub redo: Vec<(String, usize)>,
}

/// Typography reused for new text objects without copying their content or geometry.
#[derive(Clone)]
pub struct FontSettings {
    font: FontArc,
    font_name: String,
    size: f32,
    bold: bool,
    italic: bool,
    color: Color32,
    alignment: u8,
}
impl FontSettings {
    pub fn from_text(data: &TextData) -> Self {
        Self {
            font: data.font.clone(),
            font_name: data.font_name.clone(),
            size: data.size,
            bold: data.bold,
            italic: data.italic,
            color: data.color,
            alignment: data.alignment,
        }
    }
    pub fn apply(&self, data: &mut TextData) {
        data.font = self.font.clone();
        data.font_name = self.font_name.clone();
        data.size = self.size;
        data.bold = self.bold;
        data.italic = self.italic;
        data.color = self.color;
        data.alignment = self.alignment;
    }
}

impl TextData {
    pub fn new(kind: Kind, size: f32) -> Self {
        Self {
            kind,
            content: String::new(),
            font: sans(),
            font_name: "Sans".into(),
            font_error: None,
            size,
            bold: false,
            italic: false,
            color: Color32::BLACK,
            border: Color32::BLACK,
            border_width: 3.0,
            fill: Color32::WHITE,
            shadow: false,
            shadow_color: Color32::BLACK,
            shadow_offset: egui::vec2(2.0, 2.0),
            shadow_blur: 2.0,
            alignment: 0,
            flip_horizontal: false,
            flip_vertical: false,
            cursor: 0,
            anchor: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
    pub(crate) fn selection(&self) -> std::ops::Range<usize> {
        let anchor = self.anchor.unwrap_or(self.cursor);
        anchor.min(self.cursor)..anchor.max(self.cursor)
    }
    fn checkpoint(&mut self) {
        if self.undo.len() >= 100 {
            self.undo.remove(0);
        }
        self.undo.push((self.content.clone(), self.cursor));
        self.redo.clear();
    }
    pub(crate) fn insert(&mut self, value: &str) {
        self.checkpoint();
        let range = self.selection();
        let start = range.start;
        self.content.replace_range(range, value);
        self.cursor = start + value.len();
        self.anchor = None;
    }
    fn previous(&self) -> usize {
        self.content[..self.cursor]
            .char_indices()
            .last()
            .map_or(0, |(i, _)| i)
    }
    fn next(&self) -> usize {
        self.cursor
            + self.content[self.cursor..]
                .chars()
                .next()
                .map_or(0, char::len_utf8)
    }
    pub(crate) fn word_boundary(&self, backward: bool) -> usize {
        let mut position = self.cursor;
        if backward {
            let mut chars = self.content[..position].char_indices().rev().peekable();
            while chars.peek().is_some_and(|(_, c)| c.is_whitespace()) {
                position = chars.next().unwrap().0;
            }
            while chars.peek().is_some_and(|(_, c)| !c.is_whitespace()) {
                position = chars.next().unwrap().0;
            }
        } else {
            let mut chars = self.content[position..].char_indices().peekable();
            while chars.peek().is_some_and(|(_, c)| !c.is_whitespace()) {
                let (i, c) = chars.next().unwrap();
                position = self.cursor + i + c.len_utf8();
            }
            while chars.peek().is_some_and(|(_, c)| c.is_whitespace()) {
                let (i, c) = chars.next().unwrap();
                position = self.cursor + i + c.len_utf8();
            }
        }
        position
    }
    pub(crate) fn erase(&mut self, backward: bool) {
        let mut range = self.selection();
        if range.is_empty() {
            range = if backward {
                self.previous()..self.cursor
            } else {
                self.cursor..self.next()
            };
        }
        if !range.is_empty() {
            self.checkpoint();
            self.cursor = range.start;
            self.content.replace_range(range, "");
            self.anchor = None;
        }
    }
    pub(crate) fn history(&mut self, redo: bool) {
        let previous = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(previous) = previous {
            let current = (self.content.clone(), self.cursor);
            if redo {
                self.undo.push(current);
            } else {
                self.redo.push(current);
            }
            (self.content, self.cursor) = previous;
            self.anchor = None;
        }
    }

    pub fn keyboard(&mut self, ctx: &egui::Context, layout: &Layout) -> (bool, bool) {
        let before = self.content.clone();
        let mut accept = false;
        let events = ctx.input_mut(|input| {
            let mut events = Vec::new();
            input.events.retain(|event| {
                let handled = matches!(
                    event,
                    egui::Event::Text(_)
                        | egui::Event::Paste(_)
                        | egui::Event::Copy
                        | egui::Event::Cut
                ) || matches!(
                    event,
                    egui::Event::Key {
                        pressed: true,
                        key: egui::Key::Enter
                            | egui::Key::A
                            | egui::Key::Z
                            | egui::Key::ArrowLeft
                            | egui::Key::ArrowRight
                            | egui::Key::ArrowUp
                            | egui::Key::ArrowDown
                            | egui::Key::Home
                            | egui::Key::End
                            | egui::Key::Backspace
                            | egui::Key::Delete,
                        ..
                    }
                );
                if handled {
                    events.push(event.clone());
                }
                !handled
            });
            events
        });
        for event in events {
            match event {
                egui::Event::Text(value) | egui::Event::Paste(value) => {
                    self.insert(&value.replace("\r\n", "\n").replace('\r', "\n"))
                }
                egui::Event::Copy => ctx.copy_text(self.content[self.selection()].to_owned()),
                egui::Event::Cut => {
                    ctx.copy_text(self.content[self.selection()].to_owned());
                    if !self.selection().is_empty() {
                        self.erase(false);
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let command = modifiers.command || modifiers.ctrl;
                    if key == egui::Key::Enter {
                        if command {
                            self.insert("\n");
                        } else {
                            accept = true;
                        }
                        continue;
                    }
                    if command && key == egui::Key::A {
                        self.anchor = Some(0);
                        self.cursor = self.content.len();
                        continue;
                    }
                    if command && key == egui::Key::Z {
                        self.history(modifiers.shift);
                        continue;
                    }
                    if key == egui::Key::Backspace {
                        if command && self.selection().is_empty() {
                            self.anchor = Some(self.word_boundary(true));
                        }
                        self.erase(true);
                        continue;
                    }
                    if key == egui::Key::Delete {
                        if command && self.selection().is_empty() {
                            self.anchor = Some(self.word_boundary(false));
                        }
                        self.erase(false);
                        continue;
                    }
                    let destination = match key {
                        egui::Key::ArrowLeft => {
                            Some(if !modifiers.shift && !self.selection().is_empty() {
                                self.selection().start
                            } else if command {
                                self.word_boundary(true)
                            } else {
                                self.previous()
                            })
                        }
                        egui::Key::ArrowRight => {
                            Some(if !modifiers.shift && !self.selection().is_empty() {
                                self.selection().end
                            } else if command {
                                self.word_boundary(false)
                            } else {
                                self.next()
                            })
                        }
                        egui::Key::Home if command => Some(0),
                        egui::Key::End if command => Some(self.content.len()),
                        egui::Key::Home => {
                            Some(self.content[..self.cursor].rfind('\n').map_or(0, |i| i + 1))
                        }
                        egui::Key::End => Some(
                            self.content[self.cursor..]
                                .find('\n')
                                .map_or(self.content.len(), |i| self.cursor + i),
                        ),
                        egui::Key::ArrowUp | egui::Key::ArrowDown => {
                            let current = layout
                                .carets
                                .iter()
                                .rev()
                                .find(|(i, _)| *i == self.cursor)
                                .map(|(_, p)| *p)
                                .unwrap_or(Pos2::ZERO);
                            Some(layout.hit(
                                current
                                    + egui::vec2(
                                        0.0,
                                        if key == egui::Key::ArrowUp {
                                            -layout.line_height
                                        } else {
                                            layout.line_height
                                        },
                                    ),
                            ))
                        }
                        _ => None,
                    };
                    if let Some(destination) = destination {
                        if modifiers.shift {
                            if self.anchor.is_none() {
                                self.anchor = Some(self.cursor);
                            }
                        } else {
                            self.anchor = None;
                        }
                        self.cursor = destination;
                    }
                }
                _ => {}
            }
        }
        (self.content != before, accept)
    }
}

pub fn sans() -> FontArc {
    FontArc::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).unwrap()
}
pub fn mono() -> FontArc {
    FontArc::try_from_slice(epaint_default_fonts::HACK_REGULAR).unwrap()
}

pub fn text_bounds(data: &TextData, width: u32, height: u32) -> Rect {
    let size = egui::vec2(width as f32, height as f32);
    let (min, max) = match data.kind {
        Kind::Area => (egui::vec2(0.0, 0.0), egui::vec2(1.0, 1.0)),
        Kind::Speech => (egui::vec2(0.0, 0.0), egui::vec2(1.0, 0.684)),
        Kind::Thought => (egui::vec2(0.0, 0.0), egui::vec2(1.0, 0.684)),
    };
    let mut rect = Rect::from_min_max(Pos2::ZERO + size * min, Pos2::ZERO + size * max);
    if data.flip_vertical {
        rect = Rect::from_min_max(
            egui::pos2(rect.left(), size.y - rect.bottom()),
            egui::pos2(rect.right(), size.y - rect.top()),
        );
    }
    let padding = (data.border_width / 2.0 + 6.0).min(rect.width().min(rect.height()) / 4.0);
    rect.shrink(padding)
}

// Use the narrowest pill cross-section touched by the entire line, so both
// ascenders and descenders stay inside the curved sides.
fn line_span(data: &TextData, bounds: Rect, row: usize, height: f32) -> (f32, f32) {
    if data.kind != Kind::Thought {
        return (bounds.left(), bounds.width());
    }
    let center = bounds.center().y;
    let radius = bounds.height() / 2.0;
    let top = bounds.top() + row as f32 * height;
    let distance = (top - center).abs().max((top + height - center).abs());
    let fraction = (distance / radius.max(1.0)).min(1.0);
    let inset = bounds.width() / 3.0 * (1.0 - (1.0 - fraction * fraction).sqrt());
    (
        bounds.left() + inset,
        (bounds.width() - 2.0 * inset).max(1.0),
    )
}

#[derive(Clone)]
pub struct Layout {
    glyphs: Vec<(Glyph, f32)>,
    pub carets: Vec<(usize, Pos2)>,
    pub line_height: f32,
    pub bounds: Rect,
    pub overflow: bool,
}
impl Layout {
    pub fn hit(&self, point: Pos2) -> usize {
        self.carets
            .iter()
            .min_by(|(_, a), (_, b)| {
                // Choose the nearest line before comparing horizontal distance.
                // Balloon lines can have very different left insets.
                (point.y - a.y)
                    .abs()
                    .total_cmp(&(point.y - b.y).abs())
                    .then_with(|| (point.x - a.x).abs().total_cmp(&(point.x - b.x).abs()))
            })
            .map_or(0, |(i, _)| *i)
    }
}

pub fn layout(data: &TextData, width: u32, height: u32) -> Layout {
    let bounds = text_bounds(data, width, height);
    let font = data.font.as_scaled(PxScale::from(data.size));
    let line_height = (font.ascent() - font.descent() + font.line_gap())
        .ceil()
        .max(1.0);
    let overhang = if data.italic { data.size * 0.22 } else { 0.0 }
        + if data.bold {
            (data.size * 0.035).max(1.0)
        } else {
            0.0
        };
    let row_space = |row| {
        let (left, width) = line_span(data, bounds, row, line_height);
        (left, (width - overhang).max(1.0))
    };
    let mut available = row_space(0).1;
    type Row = (Vec<(Glyph, f32)>, Vec<(usize, Pos2)>, f32);
    let mut rows: Vec<Row> = Vec::new();
    let mut glyphs = Vec::new();
    let mut carets = vec![(0, Pos2::ZERO)];
    let mut x = 0.0;
    let mut row = 0;
    let mut previous = None;
    for (index, ch) in data.content.char_indices() {
        if ch == '\n' {
            rows.push((std::mem::take(&mut glyphs), std::mem::take(&mut carets), x));
            row += 1;
            available = row_space(row).1;
            x = 0.0;
            previous = None;
            carets.push((index + 1, egui::pos2(0.0, row as f32 * line_height)));
            continue;
        }
        let id = font.glyph_id(ch);
        let advance = font.h_advance(id)
            + if data.bold {
                (data.size * 0.035).max(1.0)
            } else {
                0.0
            };
        let word_start = index == 0
            || data.content[..index]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace);
        let mut word_width = 0.0;
        if word_start && !ch.is_whitespace() {
            let mut prev = None;
            for letter in data.content[index..]
                .chars()
                .take_while(|c| !c.is_whitespace())
            {
                let glyph = font.glyph_id(letter);
                word_width += font.h_advance(glyph)
                    + prev.map_or(0.0, |p| font.kern(p, glyph))
                    + if data.bold {
                        (data.size * 0.035).max(1.0)
                    } else {
                        0.0
                    };
                prev = Some(glyph);
            }
        }
        let kern = previous.map_or(0.0, |p| font.kern(p, id));
        if x > 0.0
            && (x + kern + advance > available
                || (word_width <= available && x + word_width > available))
        {
            rows.push((std::mem::take(&mut glyphs), std::mem::take(&mut carets), x));
            row += 1;
            available = row_space(row).1;
            x = 0.0;
            previous = None;
            carets.push((index, egui::pos2(0.0, row as f32 * line_height)));
        }
        let kern = previous.map_or(0.0, |p| font.kern(p, id));
        x += kern;
        let baseline = row as f32 * line_height + font.ascent();
        glyphs.push((
            id.with_scale_and_position(data.size, ab_glyph::point(x, baseline)),
            baseline,
        ));
        x += advance;
        carets.push((
            index + ch.len_utf8(),
            egui::pos2(x, row as f32 * line_height),
        ));
        previous = Some(id);
    }
    rows.push((glyphs, carets, x));
    let mut glyphs = Vec::new();
    let mut carets = Vec::new();
    let mut overflow = false;
    for (row, (mut row_glyphs, mut row_carets, row_width)) in rows.into_iter().enumerate() {
        let (left, available) = row_space(row);
        if row_width > available {
            overflow = true;
        }
        let shift = match data.alignment {
            1 => (available - row_width).max(0.0) / 2.0,
            2 => (available - row_width).max(0.0),
            _ => 0.0,
        };
        for (glyph, baseline) in &mut row_glyphs {
            glyph.position.x += left + shift;
            glyph.position.y += bounds.top();
            *baseline += bounds.top();
        }
        for (_, point) in &mut row_carets {
            *point += egui::vec2(left + shift, bounds.top());
            if point.y + line_height > bounds.bottom() {
                overflow = true;
            }
        }
        glyphs.extend(row_glyphs);
        carets.extend(row_carets);
    }
    Layout {
        glyphs,
        carets,
        line_height,
        bounds,
        overflow,
    }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Container {
    kind: Kind,
    border: Color32,
    border_width: f32,
    fill: Color32,
    flip_horizontal: bool,
    flip_vertical: bool,
}
impl Container {
    pub fn from_text(data: &TextData) -> Self {
        Self {
            kind: data.kind,
            border: data.border,
            border_width: data.border_width,
            fill: data.fill,
            flip_horizontal: data.flip_horizontal,
            flip_vertical: data.flip_vertical,
        }
    }
    pub fn render(self, width: u32, height: u32) -> RgbaImage {
        let shape = match self.kind {
            Kind::Area => crate::shapes::ShapeKind::Square,
            Kind::Speech => crate::shapes::ShapeKind::WordBalloon,
            Kind::Thought => crate::shapes::ShapeKind::ThoughtBalloon,
        };
        let mut image = crate::shapes::render(
            shape,
            crate::shapes::Style {
                border_width: self.border_width,
                border: self.border,
                fill: self.fill,
                ..Default::default()
            },
            width,
            height,
        );
        if self.flip_horizontal {
            image::imageops::flip_horizontal_in_place(&mut image);
        }
        if self.flip_vertical {
            image::imageops::flip_vertical_in_place(&mut image);
        }
        image
    }
}

pub fn render(data: &TextData, width: u32, height: u32) -> (RgbaImage, Layout) {
    let layout = layout(data, width, height);
    let mut image = Container::from_text(data).render(width, height);
    let mut mask = GrayImage::new(width, height);
    let bold = if data.bold {
        (data.size * 0.035).round().clamp(1.0, 6.0) as i32
    } else {
        0
    };
    for (glyph, baseline) in &layout.glyphs {
        if let Some(outline) = data.font.outline_glyph(glyph.clone()) {
            let bounds = outline.px_bounds();
            outline.draw(|x, y, coverage| {
                let py = bounds.min.y.floor() as i32 + y as i32;
                let shear = if data.italic {
                    ((*baseline - py as f32) * 0.2).round() as i32
                } else {
                    0
                };
                for offset in 0..=bold {
                    let px = bounds.min.x.floor() as i32 + x as i32 + shear + offset;
                    if px >= 0
                        && py >= 0
                        && px < width as i32
                        && py < height as i32
                        && layout.bounds.contains(egui::pos2(px as f32, py as f32))
                    {
                        let value = mask.get_pixel_mut(px as u32, py as u32);
                        value[0] = value[0].max((coverage * 255.0).round() as u8);
                    }
                }
            });
        }
    }
    if data.shadow {
        let blurred = if data.shadow_blur > 0.0 {
            image::imageops::blur(&mask, data.shadow_blur)
        } else {
            mask.clone()
        };
        for (x, y, p) in blurred.enumerate_pixels() {
            let px = x as i32 + data.shadow_offset.x.round() as i32;
            let py = y as i32 + data.shadow_offset.y.round() as i32;
            if px >= 0
                && py >= 0
                && px < width as i32
                && py < height as i32
                && layout.bounds.contains(egui::pos2(px as f32, py as f32))
            {
                blend(
                    image.get_pixel_mut(px as u32, py as u32),
                    data.shadow_color,
                    p[0],
                );
            }
        }
    }
    for (x, y, p) in mask.enumerate_pixels() {
        if p[0] > 0 {
            blend(image.get_pixel_mut(x, y), data.color, p[0]);
        }
    }
    (image, layout)
}
fn blend(dst: &mut image::Rgba<u8>, color: Color32, coverage: u8) {
    let src = color.to_srgba_unmultiplied();
    let sa = (src[3] as u32 * coverage as u32 + 127) / 255;
    let da = dst[3] as u32;
    let alpha = sa * 255 + da * (255 - sa);
    if alpha == 0 {
        return;
    }
    for i in 0..3 {
        dst[i] = ((src[i] as u32 * sa * 255 + dst[i] as u32 * da * (255 - sa) + alpha / 2) / alpha)
            as u8;
    }
    dst[3] = ((alpha + 127) / 255) as u8;
}

pub fn properties(
    ui: &mut egui::Ui,
    data: &mut TextData,
    palette: &mut crate::colors::Palette,
    overflow: bool,
) -> bool {
    let mut changed = false;
    ui.label(data.kind.name());
    egui::ComboBox::from_id_salt("text_font")
        .selected_text(&data.font_name)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(data.font_name == "Sans", "Sans")
                .clicked()
            {
                data.font = sans();
                data.font_name = "Sans".into();
                changed = true;
            }
            if ui
                .selectable_label(data.font_name == "Monospace", "Monospace")
                .clicked()
            {
                data.font = mono();
                data.font_name = "Monospace".into();
                changed = true;
            }
        });
    if ui.button("Load font…").clicked()
        && let Some(path) = rfd::FileDialog::new()
            .add_filter("Fonts", &["ttf", "otf"])
            .pick_file()
    {
        match std::fs::read(&path)
            .ok()
            .and_then(|bytes| FontArc::try_from_vec(bytes).ok())
        {
            Some(font) => {
                data.font = font;
                data.font_error = None;
                data.font_name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into();
                changed = true;
            }
            None => {
                data.font_error = Some("Could not load this font.".into());
            }
        }
    }
    if let Some(error) = &data.font_error {
        ui.colored_label(Color32::RED, error);
    }

    changed |= crate::toolbar::numeric(ui, "Size", &mut data.size, 4.0..=512.0, " px", true);
    ui.horizontal(|ui| {
        changed |= ui.checkbox(&mut data.bold, "Bold").changed();
        changed |= ui.checkbox(&mut data.italic, "Italic").changed();
    });
    egui::ComboBox::from_id_salt("text_alignment")
        .selected_text(match data.alignment {
            1 => "Center",
            2 => "Right",
            _ => "Left",
        })
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(&mut data.alignment, 0, "Left")
                .changed();
            changed |= ui
                .selectable_value(&mut data.alignment, 1, "Center")
                .changed();
            changed |= ui
                .selectable_value(&mut data.alignment, 2, "Right")
                .changed();
        });
    for (label, color, target) in [
        ("Text", data.color, crate::colors::Target::TextColor),
        ("Border", data.border, crate::colors::Target::TextBorder),
        ("Background", data.fill, crate::colors::Target::TextFill),
    ] {
        if crate::colors::selector(ui, label, color).clicked() {
            palette.begin(target, color, 0.0);
        }
    }
    changed |= crate::toolbar::border_size(ui, &mut data.border_width);
    changed |= ui.checkbox(&mut data.shadow, "Text shadow").changed();
    if data.shadow {
        if crate::colors::selector(ui, "Shadow", data.shadow_color).clicked() {
            palette.begin(crate::colors::Target::TextShadow, data.shadow_color, 0.0);
        }
        changed |= crate::toolbar::numeric(
            ui,
            "X",
            &mut data.shadow_offset.x,
            -32.0..=32.0,
            " px",
            false,
        );
        changed |= crate::toolbar::numeric(
            ui,
            "Y",
            &mut data.shadow_offset.y,
            -32.0..=32.0,
            " px",
            false,
        );
        changed |=
            crate::toolbar::numeric(ui, "Blur", &mut data.shadow_blur, 0.0..=16.0, " px", false);
    }
    if overflow {
        ui.colored_label(
            Color32::YELLOW,
            "Text overflows. Enlarge the box or reduce the font size.",
        );
    }
    changed
}

pub fn canvas_editor(
    ui: &mut egui::Ui,
    data: &mut TextData,
    object: &crate::insert::InsertedImage,
    canvas: Rect,
    scale: f32,
) -> (bool, bool) {
    let id = egui::Id::new("canvas_text_editor");
    let size = [object.pixels.width(), object.pixels.height()];
    let layout = layout(data, size[0], size[1]);
    let focus = ui.interact(
        object.screen_rect(canvas, scale),
        id,
        egui::Sense::focusable_noninteractive(),
    );
    if let Some(pointer) = ui.input(|i| {
        i.pointer
            .primary_pressed()
            .then(|| i.pointer.interact_pos())
            .flatten()
    }) {
        let local = object.screen_to_source(pointer, canvas, scale);
        let unobscured = ui
            .ctx()
            .layer_id_at(pointer)
            .is_none_or(|layer| layer.order < egui::Order::Middle);
        ui.data_mut(|state| state.remove::<usize>(id.with("selection_start")));
        if object.text_edit_hit_test(
            pointer,
            canvas,
            scale,
            layout.bounds,
            ui.input(|i| i.modifiers.alt),
        ) && unobscured
        {
            if ui.input(|i| i.modifiers.shift) {
                data.anchor.get_or_insert(data.cursor);
            } else {
                data.anchor = None;
            }
            let cursor = layout.hit(local - egui::vec2(0.0, layout.line_height / 2.0));
            data.cursor = cursor;
            ui.data_mut(|state| state.insert_temp(id.with("selection_start"), cursor));
            focus.request_focus();
        } else {
            focus.surrender_focus();
        }
    }
    // Arrow keys belong to the text editor, rather than egui's directional
    // focus navigation between toolbar buttons and other widgets.
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            id,
            egui::EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                ..Default::default()
            },
        );
    });
    if !focus.has_focus() {
        return (false, false);
    }
    if ui.input(|i| i.pointer.primary_down() && i.pointer.is_decidedly_dragging())
        && let Some(pointer) = ui.input(|i| i.pointer.interact_pos())
    {
        let local = object.screen_to_source(pointer, canvas, scale);
        if let Some(start) = ui.data(|state| state.get_temp::<usize>(id.with("selection_start"))) {
            data.anchor.get_or_insert(start);
            data.cursor = layout.hit(local - egui::vec2(0.0, layout.line_height / 2.0));
        }
    }
    let result = data.keyboard(ui.ctx(), &layout);
    let layout = layout_after_edit(data, size, &layout, result.0);
    if data.content.is_empty() {
        let origin = object.source_to_screen(layout.bounds.min, canvas, scale);
        ui.painter().text(
            origin,
            egui::Align2::LEFT_TOP,
            "Type here",
            egui::FontId::proportional((data.size * scale).clamp(10.0, 24.0)),
            Color32::GRAY,
        );
    }
    if let Some((_, caret)) = layout.carets.iter().rev().find(|(i, _)| *i == data.cursor) {
        let bottom = egui::pos2(
            caret.x,
            (caret.y + layout.line_height).min(layout.bounds.bottom()),
        );
        if layout.bounds.contains(*caret) {
            ui.painter().line_segment(
                [
                    object.source_to_screen(*caret, canvas, scale),
                    object.source_to_screen(bottom, canvas, scale),
                ],
                egui::Stroke::new(1.5_f32, Color32::LIGHT_BLUE),
            );
        }
    }
    let selection = data.selection();
    if !selection.is_empty() {
        for pair in layout.carets.windows(2) {
            if pair[0].0 >= selection.start
                && pair[0].0 < selection.end
                && (pair[0].1.y - pair[1].1.y).abs() < 0.1
            {
                let a = pair[0].1;
                let b = pair[1].1;
                if layout.bounds.contains(a) {
                    let corners = [
                        a,
                        b,
                        b + egui::vec2(0.0, layout.line_height),
                        a + egui::vec2(0.0, layout.line_height),
                    ];
                    ui.painter().add(egui::Shape::convex_polygon(
                        corners
                            .into_iter()
                            .map(|p| object.source_to_screen(p, canvas, scale))
                            .collect(),
                        Color32::from_rgba_unmultiplied(80, 140, 255, 60),
                        egui::Stroke::NONE,
                    ));
                }
            }
        }
    }
    result
}
fn layout_after_edit<'a>(
    data: &TextData,
    size: [u32; 2],
    previous: &'a Layout,
    changed: bool,
) -> std::borrow::Cow<'a, Layout> {
    if changed {
        std::borrow::Cow::Owned(layout(data, size[0], size[1]))
    } else {
        std::borrow::Cow::Borrowed(previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(key: egui::Key, ctrl: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: if ctrl {
                egui::Modifiers::CTRL
            } else {
                egui::Modifiers::NONE
            },
        }
    }
    fn input(data: &mut TextData, events: Vec<egui::Event>) -> (bool, bool) {
        let ctx = egui::Context::default();
        let mut result = (false, false);
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                let layout = layout(data, 300, 200);
                result = data.keyboard(ctx, &layout);
            },
        );
        result
    }
    #[test]
    fn zero_text_shadow_blur_matches_original_glyph_coverage() {
        let mut data = TextData::new(Kind::Area, 24.0);
        data.content = "Sharp".into();
        data.fill = Color32::TRANSPARENT;
        data.border_width = 0.0;
        data.color = Color32::RED;
        let expected = render(&data, 150, 80).0;
        data.color = Color32::TRANSPARENT;
        data.shadow = true;
        data.shadow_color = Color32::RED;
        data.shadow_offset = Vec2::ZERO;
        data.shadow_blur = 0.0;
        assert_eq!(render(&data, 150, 80).0, expected);
    }

    #[test]
    fn flipping_keeps_letters_upright_and_moves_the_balloon_text_area() {
        for horizontal in [false, true] {
            for vertical in [false, true] {
                let mut data = TextData::new(Kind::Speech, 24.0);
                data.content = "Readable".into();
                data.flip_horizontal = horizontal;
                data.flip_vertical = vertical;
                let (combined, layout) = render(&data, 320, 240);
                let mut letters = data.clone();
                letters.border = Color32::TRANSPARENT;
                letters.fill = Color32::TRANSPARENT;
                let letters = render(&letters, 320, 240).0;
                for (x, y, p) in letters.enumerate_pixels() {
                    if p[3] == 255 {
                        assert_eq!(combined.get_pixel(x, y), p);
                    }
                }
                assert_eq!(layout.bounds, text_bounds(&data, 320, 240));
                data.flip_horizontal = false;
                assert_eq!(render(&data, 320, 240).1.carets, layout.carets);
            }
        }
    }

    #[test]
    fn selections_collapse_and_empty_cut_preserves_text() {
        let mut data = TextData::new(Kind::Area, 20.0);
        data.insert("one two");
        input(&mut data, vec![egui::Event::Cut]);
        assert_eq!(data.content, "one two");
        data.anchor = Some(0);
        input(&mut data, vec![key(egui::Key::ArrowLeft, false)]);
        assert_eq!(data.cursor, 0);
        data.anchor = Some(data.content.len());
        input(&mut data, vec![key(egui::Key::ArrowRight, false)]);
        assert_eq!(data.cursor, data.content.len());
        input(&mut data, vec![key(egui::Key::Backspace, true)]);
        assert_eq!(data.content, "one ");
    }

    #[test]
    fn italic_does_not_change_balloon_geometry() {
        for kind in [Kind::Area, Kind::Speech, Kind::Thought] {
            let mut data = TextData::new(kind, 24.0);
            let normal = render(&data, 320, 240).0;
            data.italic = true;
            assert_eq!(normal, render(&data, 320, 240).0);
        }
    }

    #[test]
    fn thought_text_rows_follow_the_pill_contour() {
        let data = TextData::new(Kind::Thought, 20.0);
        let bounds = text_bounds(&data, 400, 300);
        let (top_left, top_width) = line_span(&data, bounds, 0, 20.0);
        let middle_row = (bounds.height() / 40.0) as usize;
        let (middle_left, middle_width) = line_span(&data, bounds, middle_row, 20.0);
        assert!(top_left > middle_left);
        assert!(top_width < middle_width);
        let speech = TextData::new(Kind::Speech, 20.0);
        let speech_bounds = text_bounds(&speech, 400, 300);
        assert!(speech_bounds.left() < 12.0);
        assert!(speech_bounds.top() < 12.0);
    }

    #[test]
    fn wraps_words_and_long_words_and_respects_explicit_line_breaks() {
        let mut data = TextData::new(Kind::Area, 24.0);
        data.content =
            "A long sentence with multiple words and Supercalifragilisticexpialidocious".into();
        let wide = layout(&data, 600, 400);
        let narrow = layout(&data, 140, 400);
        let last_y = |layout: &Layout| layout.carets.last().unwrap().1.y;
        assert!(last_y(&narrow) > last_y(&wide));
        assert!(
            narrow
                .carets
                .iter()
                .all(|(_, p)| p.x <= narrow.bounds.right() + 1.0)
        );
        data.content = "First\nSecond".into();
        let forced = layout(&data, 600, 200);
        assert!(
            (forced.carets.last().unwrap().1.y - forced.bounds.top() - forced.line_height).abs()
                < 0.01
        );
    }
    #[test]
    fn ctrl_enter_inserts_newline_and_plain_enter_accepts() {
        let mut data = TextData::new(Kind::Area, 20.0);
        input(&mut data, vec![egui::Event::Text("First".into())]);
        assert_eq!(
            input(&mut data, vec![key(egui::Key::Enter, true)]),
            (true, false)
        );
        input(&mut data, vec![egui::Event::Text("Second".into())]);
        assert_eq!(data.content, "First\nSecond");
        assert_eq!(
            input(&mut data, vec![key(egui::Key::Enter, false)]),
            (false, true)
        );
    }
    #[test]
    fn unicode_deletion_selection_paste_and_local_undo_are_safe() {
        let mut data = TextData::new(Kind::Area, 20.0);
        input(&mut data, vec![egui::Event::Text("hé🙂".into())]);
        input(&mut data, vec![key(egui::Key::Backspace, false)]);
        assert_eq!(data.content, "hé");
        input(
            &mut data,
            vec![
                key(egui::Key::A, true),
                egui::Event::Paste("Replacement".into()),
            ],
        );
        assert_eq!(data.content, "Replacement");
        input(&mut data, vec![key(egui::Key::Z, true)]);
        assert_eq!(data.content, "hé");
    }
    #[test]
    fn glyphs_and_shadow_are_clipped_to_the_text_area_and_overflow_is_reported() {
        let mut data = TextData::new(Kind::Area, 24.0);
        data.fill = Color32::TRANSPARENT;
        data.border_width = 0.0;
        data.content = "Many words repeated ".repeat(20);
        data.shadow = true;
        data.shadow_color = Color32::RED;
        let (pixels, layout) = render(&data, 150, 80);
        assert!(layout.overflow);
        assert!(
            pixels
                .enumerate_pixels()
                .all(
                    |(x, y, p)| layout.bounds.contains(egui::pos2(x as f32, y as f32)) || p[3] == 0
                )
        );
        assert!(pixels.pixels().any(|p| p[3] > 0));
    }
    #[test]
    fn font_bold_italic_and_shadow_affect_the_rendered_pixels() {
        let mut data = TextData::new(Kind::Area, 28.0);
        data.content = "QuipIt text".into();
        let plain = render(&data, 300, 120).0;
        data.bold = true;
        assert_ne!(render(&data, 300, 120).0, plain);
        data.bold = false;
        data.italic = true;
        assert_ne!(render(&data, 300, 120).0, plain);
        data.italic = false;
        data.font = mono();
        assert_ne!(render(&data, 300, 120).0, plain);
        data.shadow = true;
        data.shadow_color = Color32::RED;
        assert!(
            render(&data, 300, 120)
                .0
                .pixels()
                .any(|p| p[0] > p[1] && p[3] > 0)
        );
    }
}

mod font_serde {
    use ab_glyph::{Font, FontArc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(font: &FontArc, serializer: S) -> Result<S::Ok, S::Error> {
        font.font_data().serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<FontArc, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        FontArc::try_from_vec(bytes).map_err(serde::de::Error::custom)
    }
}
