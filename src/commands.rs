pub use crate::insert::ImageStyle;
use crate::{QuipIt, insert, objects, project, shapes, text};
mod operations;
use ab_glyph::FontArc;
use eframe::egui::{self, Color32, Rect, Vec2};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(u64);

impl ObjectId {
    pub(crate) fn new() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
    pub(crate) fn is_valid(self) -> bool {
        self.0 > 0 && self.0 < u64::MAX
    }
    pub(crate) fn reserve(self) {
        NEXT_ID.fetch_max(self.0 + 1, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Selected,
    Object { id: ObjectId },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Inspect,
    DescribeDocument,
    CountObjects {
        #[serde(default)]
        kind: Option<ObjectKind>,
        #[serde(default)]
        visible: Option<bool>,
    },
    #[serde(alias = "purge", alias = "clear_history", alias = "clear_undo_history")]
    PurgeHistory,
    NewDocument {
        width: u32,
        height: u32,
        background: [u8; 4],
    },
    OpenDocument {
        path: PathBuf,
    },
    SaveProject {
        path: PathBuf,
    },
    ExportImage {
        path: PathBuf,
    },
    InsertImage {
        path: PathBuf,
    },
    AddShape {
        kind: shapes::ShapeKind,
    },
    AddText {
        kind: text::Kind,
    },
    SelectObject {
        id: ObjectId,
    },
    SelectBackground,
    UpdateObject {
        target: Target,
        patch: ObjectPatch,
    },
    EditText {
        target: Target,
        edit: TextEdit,
    },
    Transform {
        target: Target,
        degrees: f32,
        horizontal: bool,
        vertical: bool,
    },
    MoveCurvePoint {
        target: Target,
        index: usize,
        position: egui::Pos2,
    },
    SetCurveEditing {
        enabled: bool,
    },
    SetBackground {
        color: [u8; 4],
    },
    SetVisibility {
        target: Target,
        visible: bool,
    },
    Rename {
        target: Target,
        name: String,
    },
    Delete {
        target: Target,
    },
    Duplicate {
        target: Target,
    },
    Reorder {
        target: Target,
        index: usize,
    },
    Crop {
        rect: [u32; 4],
    },
    Resize {
        width: u32,
        height: u32,
    },
    Apply,
    Cancel,
    Undo,
    Redo,
    Batch {
        commands: Vec<Command>,
    },
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Shape,
    Text,
    Image,
}

#[derive(Clone, Default, Serialize)]
pub struct ObjectCounts {
    pub total: usize,
    pub visible: usize,
    pub hidden: usize,
    pub shapes: usize,
    pub text: usize,
    pub images: usize,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackgroundInfo {
    Uniform { color: [u8; 4] },
    Mixed,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ObjectPatch {
    pub bounds: Option<Rect>,
    pub shape: Option<ShapePatch>,
    pub text: Option<TextPatch>,
    pub image: Option<ImagePatch>,
    pub shadow: Option<ShadowPatch>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShapePatch {
    pub border_width: Option<f32>,
    pub border: Option<[u8; 4]>,
    pub fill: Option<[u8; 4]>,
    pub transparency: Option<f32>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShadowPatch {
    pub enabled: Option<bool>,
    pub color: Option<[u8; 4]>,
    pub offset: Option<Vec2>,
    pub blur: Option<f32>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImagePatch {
    pub border_width: Option<f32>,
    pub border: Option<[u8; 4]>,
    pub border_transparency: Option<f32>,
    pub transparency: Option<f32>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FontSource {
    Sans,
    Monospace,
    File { path: PathBuf },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum TextEdit {
    Select { start: usize, end: usize },
    Insert { content: String },
    Backspace { word: bool },
    Delete { word: bool },
    Undo,
    Redo,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TextPatch {
    pub content: Option<String>,
    pub font: Option<FontSource>,
    pub size: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub color: Option<[u8; 4]>,
    pub border: Option<[u8; 4]>,
    pub border_width: Option<f32>,
    pub fill: Option<[u8; 4]>,
    pub alignment: Option<u8>,
    pub shadow: Option<ShadowPatch>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NoDocument,
    NoSelection,
    NotFound,
    InvalidArgument,
    WrongObjectType,
    Io,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandError {
    pub code: ErrorCode,
    pub message: String,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl std::error::Error for CommandError {}

type Result<T> = std::result::Result<T, CommandError>;
fn error(code: ErrorCode, message: impl Into<String>) -> CommandError {
    CommandError {
        code,
        message: message.into(),
    }
}
fn invalid(message: impl Into<String>) -> CommandError {
    error(ErrorCode::InvalidArgument, message)
}
fn io_error(message: impl ToString) -> CommandError {
    error(ErrorCode::Io, message.to_string())
}
fn color(value: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(value[0], value[1], value[2], value[3])
}

#[derive(Clone, Serialize)]
pub struct DocumentInfo {
    pub dimensions: Option<[u32; 2]>,
    pub background: Option<BackgroundInfo>,
    pub object_counts: ObjectCounts,
    pub undo_steps: usize,
    pub redo_steps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_count: Option<usize>,
    pub objects: Vec<ObjectInfo>,
    pub selected: Option<ObjectId>,
    pub background_selected: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub dirty: bool,
}

#[derive(Clone, Serialize)]
pub struct ObjectInfo {
    pub id: ObjectId,
    pub name: String,
    pub visible: bool,
    pub editing: bool,
    pub bounds: Rect,
    pub rotation: f32,
    pub curve_editing: bool,
    pub shape: Option<(shapes::ShapeKind, shapes::Style)>,
    pub text: Option<TextInfo>,
    pub image: Option<ImageStyle>,
}

#[derive(Clone, Serialize)]
pub struct TextInfo {
    pub selection: [usize; 2],
    pub selected_text: String,
    pub kind: text::Kind,
    pub content: String,
    pub font_name: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub color: [u8; 4],
    pub border: [u8; 4],
    pub border_width: f32,
    pub fill: [u8; 4],
    pub shadow: bool,
    pub shadow_color: [u8; 4],
    pub shadow_offset: Vec2,
    pub shadow_blur: f32,
    pub alignment: u8,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
}

impl From<&text::TextData> for TextInfo {
    fn from(data: &text::TextData) -> Self {
        Self {
            selection: [data.selection().start, data.selection().end],
            selected_text: data.content[data.selection()].to_owned(),
            kind: data.kind,
            content: data.content.clone(),
            font_name: data.font_name.clone(),
            size: data.size,
            bold: data.bold,
            italic: data.italic,
            color: data.color.to_srgba_unmultiplied(),
            border: data.border.to_srgba_unmultiplied(),
            border_width: data.border_width,
            fill: data.fill.to_srgba_unmultiplied(),
            shadow: data.shadow,
            shadow_color: data.shadow_color.to_srgba_unmultiplied(),
            shadow_offset: data.shadow_offset,
            shadow_blur: data.shadow_blur,
            alignment: data.alignment,
            flip_horizontal: data.flip_horizontal,
            flip_vertical: data.flip_vertical,
        }
    }
}

#[derive(Clone)]
pub(crate) struct EditDraft {
    pub name: Option<String>,
    pub bounds: Rect,
    pub shape: Option<(shapes::ShapeKind, shapes::Style)>,
    pub text: Option<text::TextData>,
    pub image: Option<insert::ImageStyle>,
    pub rotation: f32,
    pub curve_editing: bool,
    pub text_changed: bool,
}

fn number(value: f32, minimum: f32, maximum: f32, name: &str) -> Result<()> {
    if !value.is_finite() || !(minimum..=maximum).contains(&value) {
        return Err(invalid(format!(
            "{name} must be finite and between {minimum} and {maximum}"
        )));
    }
    Ok(())
}

fn dimensions(width: u32, height: u32, maximum: u32) -> Result<()> {
    if width == 0 || height == 0 || width > maximum || height > maximum {
        return Err(invalid(format!(
            "Dimensions must be between 1 and {maximum}"
        )));
    }
    Ok(())
}

fn validate_draft(draft: &EditDraft) -> Result<()> {
    if draft.name.as_ref().is_some_and(|name| name.len() > 1024) {
        return Err(invalid("Object names cannot exceed 1024 bytes"));
    }
    if !draft.bounds.is_finite() || draft.bounds.min.to_vec2().abs().max_elem() > 1_000_000.0 {
        return Err(invalid(
            "Object position must be finite and within 1,000,000 pixels",
        ));
    }
    number(draft.bounds.width(), 1.0, 16384.0, "Object width")?;
    number(draft.bounds.height(), 1.0, 16384.0, "Object height")?;
    number(draft.rotation, -1_000_000.0, 1_000_000.0, "Rotation")?;
    if let Some((_, style)) = draft.shape {
        number(style.border_width, 0.0, 16384.0, "Shape border width")?;
        number(style.transparency, 0.0, 100.0, "Shape transparency")?;
        if style
            .curve_points
            .is_some_and(|points| points.iter().any(|point| !point.is_finite()))
        {
            return Err(invalid("Curve points must be finite"));
        }
    }
    if let Some(data) = &draft.text {
        if data.content.len() > 1_000_000 {
            return Err(invalid("Text content exceeds 1 MB"));
        }
        number(data.size, 1.0, 16384.0, "Font size")?;
        number(data.border_width, 0.0, 16384.0, "Text border width")?;
        number(data.shadow_blur, 0.0, 512.0, "Text shadow blur")?;
        if !data.shadow_offset.is_finite() || data.shadow_offset.abs().max_elem() > 16384.0 {
            return Err(invalid(
                "Text shadow offsets must be finite and within 16384 pixels",
            ));
        }
        if data.alignment > 2 {
            return Err(invalid(
                "Alignment must be 0 (left), 1 (center), or 2 (right)",
            ));
        }
        if !data.content.is_char_boundary(data.cursor)
            || data
                .anchor
                .is_some_and(|anchor| !data.content.is_char_boundary(anchor))
        {
            return Err(invalid(
                "Text selection must use UTF-8 character boundaries",
            ));
        }
    }
    if let Some(style) = draft.image {
        number(style.border_width, 0.0, 16384.0, "Image border width")?;
        number(style.border_transparency, 0.0, 100.0, "Border transparency")?;
        number(style.image_transparency, 0.0, 100.0, "Image transparency")?;
        number(style.shadow_blur, 0.0, 32.0, "Shadow blur")?;
        number(style.shadow_offset.x, -64.0, 64.0, "Shadow X offset")?;
        number(style.shadow_offset.y, -64.0, 64.0, "Shadow Y offset")?;
    }
    Ok(())
}

impl QuipIt {
    pub fn inspect(&self) -> DocumentInfo {
        let mut objects: Vec<_> = self
            .objects
            .iter()
            .map(|object| ObjectInfo {
                id: object.object.id,
                name: object.name.clone(),
                visible: object.visible,
                editing: false,
                bounds: object.object.rect,
                rotation: object.rotation,
                curve_editing: object.curve_edit,
                shape: object.shape,
                text: object.data.as_ref().map(TextInfo::from),
                image: object.object.image_style,
            })
            .collect();
        if let Some(object) = &self.inserted {
            let position = self
                .object_before
                .as_ref()
                .map_or(objects.len(), |(index, _)| *index);
            objects.insert(
                position,
                ObjectInfo {
                    id: object.id,
                    name: self.selected_object_name().unwrap_or_default(),
                    visible: self
                        .object_before
                        .as_ref()
                        .is_none_or(|(_, original)| original.visible),
                    editing: true,
                    bounds: object.rect,
                    rotation: self.shape_rotation,
                    curve_editing: self.curve_edit,
                    shape: self.active_shape,
                    text: self.active_text.as_ref().map(TextInfo::from),
                    image: object.image_style,
                },
            );
        }
        let mut counts = ObjectCounts::default();
        for object in &objects {
            counts.total += 1;
            if object.visible {
                counts.visible += 1;
            } else {
                counts.hidden += 1;
            }
            if object.text.is_some() {
                counts.text += 1;
            } else if object.shape.is_some() {
                counts.shapes += 1;
            } else {
                counts.images += 1;
            }
        }
        let background = self.pixels.as_ref().map(|pixels| {
            if let Some(color) = self.background_preview {
                return BackgroundInfo::Uniform {
                    color: color.to_array(),
                };
            }
            let first = pixels.get_pixel(0, 0).0;
            if pixels.pixels().all(|pixel| pixel.0 == first) {
                BackgroundInfo::Uniform { color: first }
            } else {
                BackgroundInfo::Mixed
            }
        });
        DocumentInfo {
            background,
            object_counts: counts,
            undo_steps: self.undo.len(),
            redo_steps: self.redo.len(),
            query_count: None,
            dimensions: self
                .pixels
                .as_ref()
                .map(|pixels| [pixels.width(), pixels.height()]),
            objects,
            selected: self.inserted.as_ref().map(|object| object.id),
            background_selected: self.background_selected,
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            dirty: self.dirty || self.inserted.is_some(),
        }
    }

    pub fn render_preview(&mut self) -> Option<RgbaImage> {
        self.update_shape();
        self.composed_canvas()
    }

    pub fn execute(&mut self, ctx: &egui::Context, command: Command) -> Result<DocumentInfo> {
        match command {
            Command::Inspect | Command::DescribeDocument => return Ok(self.inspect()),
            Command::CountObjects { kind, visible } => {
                let mut info = self.inspect();
                info.query_count = Some(
                    info.objects
                        .iter()
                        .filter(|object| {
                            visible.is_none_or(|visible| object.visible == visible)
                                && match kind {
                                    None => true,
                                    Some(ObjectKind::Text) => object.text.is_some(),
                                    Some(ObjectKind::Shape) => {
                                        object.text.is_none() && object.shape.is_some()
                                    }
                                    Some(ObjectKind::Image) => {
                                        object.text.is_none() && object.shape.is_none()
                                    }
                                }
                        })
                        .count(),
                );
                return Ok(info);
            }
            Command::PurgeHistory => {
                self.undo.clear();
                self.redo.clear();
                if let Some(text) = &mut self.active_text {
                    text.undo.clear();
                    text.redo.clear();
                }
                for object in &mut self.objects {
                    if let Some(text) = &mut object.data {
                        text.undo.clear();
                        text.redo.clear();
                    }
                }
                // An edit's cancel snapshot must not restore purged text history.
                if let Some((_, object)) = &mut self.object_before {
                    if let Some(text) = &mut object.data {
                        text.undo.clear();
                        text.redo.clear();
                    }
                }
                return Ok(self.inspect());
            }
            Command::NewDocument {
                width,
                height,
                background,
            } => {
                dimensions(width, height, crate::NEW_IMAGE_MAX_SIDE)?;
                self.new_width = width;
                self.new_height = height;
                self.new_background = Some(color(background));
                self.do_new_image(ctx);
            }
            Command::OpenDocument { path } => self.open_path(ctx, path)?,
            Command::SaveProject { path } => self.save_path(path)?,
            Command::ExportImage { path } => self.export_path(path)?,
            Command::InsertImage { path } => {
                self.require_document()?;
                let pixels = image::open(path).map_err(io_error)?.to_rgba8();
                dimensions(pixels.width(), pixels.height(), 16384)?;
                self.do_commit_insert(ctx);
                let canvas = self.pixels.as_ref().unwrap();
                let mut object = insert::InsertedImage::new(
                    pixels,
                    egui::vec2(canvas.width() as f32, canvas.height() as f32),
                    ctx,
                );
                object.image_style = Some(insert::ImageStyle::default());
                self.inserted = Some(object);
                self.curve_edit = false;
                self.shape_rotation = 0.0;
                self.selection = None;
                self.crop_mode = false;
                self.show_resize = false;
            }
            Command::AddShape { kind } => {
                self.require_document()?;
                self.do_place_shape(kind, ctx);
            }
            Command::AddText { kind } => {
                self.require_document()?;
                self.do_place_text(kind, ctx);
            }
            Command::SelectObject { id } => self.select_id(ctx, id)?,
            Command::SelectBackground => {
                self.require_document()?;
                self.do_select_background(ctx);
            }
            Command::UpdateObject { target, patch } => {
                let mut draft = self.draft_for(target)?;
                patch.apply(&mut draft)?;
                validate_draft(&draft)?;
                self.select_target(ctx, target)?;
                self.install_edit(draft);
            }
            Command::EditText { target, edit } => {
                let mut draft = self.draft_for(target)?;
                let Some(data) = &mut draft.text else {
                    return Err(error(
                        ErrorCode::WrongObjectType,
                        "Text editing requires a text object",
                    ));
                };
                match edit {
                    TextEdit::Select { start, end } => {
                        if !data.content.is_char_boundary(start)
                            || !data.content.is_char_boundary(end)
                        {
                            return Err(invalid(
                                "Text selection must use UTF-8 character boundaries",
                            ));
                        }
                        data.anchor = Some(start);
                        data.cursor = end;
                    }
                    TextEdit::Insert { content } => {
                        let content = content.replace("\r\n", "\n").replace('\r', "\n");
                        if data.content.len() - data.selection().len() + content.len() > 1_000_000 {
                            return Err(invalid("Text content exceeds 1 MB"));
                        }
                        data.insert(&content);
                    }
                    TextEdit::Backspace { word } | TextEdit::Delete { word } => {
                        let backward = matches!(edit, TextEdit::Backspace { .. });
                        if word && data.selection().is_empty() {
                            data.anchor = Some(data.word_boundary(backward));
                        }
                        data.erase(backward);
                    }
                    TextEdit::Undo => data.history(false),
                    TextEdit::Redo => data.history(true),
                }
                draft.text_changed = true;
                validate_draft(&draft)?;
                self.select_target(ctx, target)?;
                self.install_edit(draft);
            }
            Command::Transform {
                target,
                degrees,
                horizontal,
                vertical,
            } => {
                number(degrees, -36000.0, 36000.0, "Rotation")?;
                self.draft_for(target)?;
                self.select_target(ctx, target)?;
                self.transform_selected(degrees, horizontal, vertical);
            }
            Command::MoveCurvePoint {
                target,
                index,
                position,
            } => {
                let mut draft = self.draft_for(target)?;
                let Some((kind, style)) = &mut draft.shape else {
                    return Err(error(ErrorCode::WrongObjectType, "Select a curve"));
                };
                let count = match kind {
                    shapes::ShapeKind::Curve4 => 4,
                    shapes::ShapeKind::Curve5 => 5,
                    _ => {
                        return Err(error(
                            ErrorCode::WrongObjectType,
                            "Only four- and five-point curves have editable controls",
                        ));
                    }
                };
                if index >= count || !position.is_finite() {
                    return Err(invalid("Invalid curve point"));
                }
                shapes::enable_curve_edit(*kind, style, &mut draft.bounds);
                shapes::move_curve_point(*kind, style, &mut draft.bounds, index, position);
                draft.curve_editing = true;
                validate_draft(&draft)?;
                self.select_target(ctx, target)?;
                self.install_edit(draft);
            }
            Command::SetCurveEditing { enabled } => {
                let mut draft = self.draft_for(Target::Selected)?;
                if !draft.shape.is_some_and(|(kind, _)| {
                    matches!(kind, shapes::ShapeKind::Curve4 | shapes::ShapeKind::Curve5)
                }) {
                    return Err(error(
                        ErrorCode::WrongObjectType,
                        "Select a four- or five-point curve",
                    ));
                }
                draft.curve_editing = enabled;
                self.install_edit(draft);
            }
            Command::SetBackground { color: background } => {
                self.require_document()?;
                self.do_commit_insert(ctx);
                self.checkpoint();
                self.fill_background(background);
                self.dirty = true;
                self.upload_canvas(ctx);
            }
            Command::Rename { target, name } => {
                self.target_id(target)?;
                if name.trim().is_empty() || name.len() > 1024 {
                    return Err(invalid("Object name must contain 1 to 1024 bytes"));
                }
                self.select_target(ctx, target)?;
                let mut draft = self.edit_draft().unwrap();
                draft.name = Some(name);
                self.install_edit(draft);
            }
            Command::SetVisibility { target, visible } => {
                let id = self.target_id(target)?;
                self.do_commit_insert(ctx);
                let index = self.object_index(id)?;
                if self.objects[index].visible != visible {
                    self.checkpoint();
                    self.objects[index].visible = visible;
                    self.dirty = true;
                    self.refresh(ctx);
                }
            }
            Command::Delete { target } => {
                let id = self.target_id(target)?;
                if self.inserted.as_ref().is_some_and(|object| object.id == id) {
                    self.do_delete_object(ctx);
                } else {
                    self.do_commit_insert(ctx);
                    let index = self.object_index(id)?;
                    self.do_object_list_action(index, "delete", index, ctx);
                }
            }
            Command::Duplicate { target } => {
                let id = self.target_id(target)?;
                self.do_commit_insert(ctx);
                let index = self.object_index(id)?;
                self.do_object_list_action(index, "duplicate", index, ctx);
                let copy_id = self.objects[index + 1].object.id;
                self.select_id(ctx, copy_id)?;
            }
            Command::Reorder { target, index } => {
                let id = self.target_id(target)?;
                if index >= self.objects.len() + usize::from(self.inserted.is_some()) {
                    return Err(invalid("Object depth is out of range"));
                }
                self.do_commit_insert(ctx);
                let source = self.object_index(id)?;
                self.do_object_list_action(source, "reorder", index, ctx);
            }
            Command::Crop {
                rect: [x, y, width, height],
            } => {
                self.require_document()?;
                let pixels = self.pixels.as_ref().unwrap();
                if width == 0
                    || height == 0
                    || x.checked_add(width)
                        .is_none_or(|right| right > pixels.width())
                    || y.checked_add(height)
                        .is_none_or(|bottom| bottom > pixels.height())
                {
                    return Err(invalid("Crop rectangle must lie inside the canvas"));
                }
                self.do_commit_insert(ctx);
                self.selection = Some([x, y, width, height]);
                self.do_crop_to_selection(ctx);
            }
            Command::Resize { width, height } => {
                self.require_document()?;
                dimensions(width, height, 16384)?;
                self.do_commit_insert(ctx);
                self.width = width;
                self.height = height;
                self.do_resize_canvas(ctx);
            }
            Command::Apply => self.do_commit_insert(ctx),
            Command::Cancel => self.do_discard_selection(),
            Command::Undo => self.do_history(false, ctx),
            Command::Redo => self.do_history(true, ctx),
            Command::Batch { commands } => self.execute_batch(ctx, commands)?,
        }
        self.update_shape();
        if self.upload_pending {
            self.upload_canvas(ctx);
        }
        Ok(self.inspect())
    }

    fn require_document(&self) -> Result<()> {
        if self.pixels.is_none() {
            Err(error(
                ErrorCode::NoDocument,
                "Open or create a document first",
            ))
        } else {
            Ok(())
        }
    }

    fn target_id(&self, target: Target) -> Result<ObjectId> {
        let id = match target {
            Target::Selected => self
                .inserted
                .as_ref()
                .map(|object| object.id)
                .ok_or_else(|| error(ErrorCode::NoSelection, "Select an object first"))?,
            Target::Object { id } => id,
        };
        if self.inserted.as_ref().is_some_and(|object| object.id == id)
            || self.objects.iter().any(|object| object.object.id == id)
        {
            Ok(id)
        } else {
            Err(error(
                ErrorCode::NotFound,
                "Object ID does not exist in this document",
            ))
        }
    }

    fn object_index(&self, id: ObjectId) -> Result<usize> {
        self.objects
            .iter()
            .position(|object| object.object.id == id)
            .ok_or_else(|| {
                error(
                    ErrorCode::NotFound,
                    "Object ID does not exist in this document",
                )
            })
    }

    fn select_id(&mut self, ctx: &egui::Context, id: ObjectId) -> Result<()> {
        self.target_id(Target::Object { id })?;
        if !self.inserted.as_ref().is_some_and(|object| object.id == id) {
            let index = self.object_index(id)?;
            self.do_edit_object(index, ctx);
        }
        Ok(())
    }

    fn select_target(&mut self, ctx: &egui::Context, target: Target) -> Result<()> {
        let id = self.target_id(target)?;
        self.select_id(ctx, id)
    }

    pub(crate) fn edit_draft(&self) -> Option<EditDraft> {
        Some(EditDraft {
            name: self.active_name.clone(),
            bounds: self.inserted.as_ref()?.rect,
            shape: self.active_shape,
            text: self.active_text.clone(),
            image: self.inserted.as_ref()?.image_style,
            rotation: self.shape_rotation,
            curve_editing: self.curve_edit,
            text_changed: self.text_dirty,
        })
    }

    fn draft_for(&self, target: Target) -> Result<EditDraft> {
        let id = self.target_id(target)?;
        if self.inserted.as_ref().is_some_and(|object| object.id == id) {
            return Ok(self.edit_draft().unwrap());
        }
        let object = &self.objects[self.object_index(id)?];
        Ok(EditDraft {
            name: Some(object.name.clone()),
            bounds: object.object.rect,
            shape: object.shape,
            text: object.data.clone(),
            image: object.object.image_style,
            rotation: object.rotation,
            curve_editing: object.curve_edit,
            text_changed: false,
        })
    }

    fn install_edit(&mut self, draft: EditDraft) {
        self.active_name = draft.name;
        if let Some(object) = &mut self.inserted {
            object.rect = draft.bounds;
            object.image_style = draft.image;
        }
        self.active_shape = draft.shape;
        self.active_text = draft.text;
        self.shape_rotation = draft.rotation;
        self.curve_edit = draft.curve_editing;
        self.text_dirty |= draft.text_changed;
    }

    pub(crate) fn preview_object_patch(&mut self, patch: ObjectPatch) {
        let result = (|| {
            let mut draft = self.draft_for(Target::Selected)?;
            patch.apply(&mut draft)?;
            validate_draft(&draft)?;
            self.install_edit(draft);
            Ok::<_, CommandError>(())
        })();
        if let Err(error) = result {
            self.status = error.to_string();
        }
    }

    pub(crate) fn fill_background(&mut self, color: [u8; 4]) {
        if let Some(pixels) = &mut self.pixels {
            for pixel in pixels.pixels_mut() {
                *pixel = image::Rgba(color);
            }
            self.upload_pending = true;
        }
    }

    pub(crate) fn accept_ui_edit(&mut self, previous: Option<EditDraft>) {
        if let Some(draft) = self.edit_draft() {
            if let Err(error) = validate_draft(&draft) {
                if let Some(previous) = previous {
                    self.install_edit(previous);
                }
                self.status = error.to_string();
            } else {
                self.install_edit(draft);
            }
        }
    }

    pub(crate) fn apply_ui_draft(&mut self, draft: Option<EditDraft>) {
        if let Some(draft) = draft {
            if let Err(error) = validate_draft(&draft) {
                self.status = error.to_string();
            } else {
                self.install_edit(draft);
            }
        }
    }

    pub(crate) fn transform_selected(&mut self, degrees: f32, horizontal: bool, vertical: bool) {
        if let Some(object) = &mut self.inserted {
            if let Some((kind, style)) = &mut self.active_shape {
                shapes::transform_shape(
                    *kind,
                    style,
                    &mut object.rect,
                    degrees,
                    horizontal,
                    vertical,
                );
            } else if let Some(data) = &mut self.active_text {
                data.flip_horizontal ^= horizontal;
                data.flip_vertical ^= vertical;
                self.text_dirty |= horizontal || vertical;
                object.transform_image(degrees, false, false);
            } else {
                object.transform_image(degrees, horizontal, vertical);
            }
            self.shape_rotation = (self.shape_rotation + degrees + 180.0).rem_euclid(360.0) - 180.0;
        }
    }

    fn execute_batch(&mut self, ctx: &egui::Context, commands: Vec<Command>) -> Result<()> {
        self.require_document()?;
        if commands.is_empty() || commands.len() > 1024 {
            return Err(invalid("A batch must contain 1 to 1024 commands"));
        }
        if commands.iter().any(|command| {
            matches!(
                command,
                Command::NewDocument { .. }
                    | Command::OpenDocument { .. }
                    | Command::SaveProject { .. }
                    | Command::ExportImage { .. }
                    | Command::PurgeHistory
                    | Command::Undo
                    | Command::Redo
                    | Command::Batch { .. }
            )
        }) {
            return Err(error(
                ErrorCode::Unsupported,
                "Batches cannot replace documents, write files, change history, or contain nested batches",
            ));
        }
        let previous = self.clone();
        for command in commands {
            if let Err(error) = self.execute(ctx, command) {
                *self = previous;
                self.restore_command_textures(ctx);
                return Err(error);
            }
        }
        self.do_commit_insert(ctx);
        self.undo = previous.undo.clone();
        self.redo = previous.redo.clone();
        if let Some(state) = previous.snapshot() {
            if self.undo.len() >= 10 {
                self.undo.remove(0);
            }
            self.undo.push(state);
            self.redo.clear();
        }
        Ok(())
    }

    fn restore_command_textures(&mut self, ctx: &egui::Context) {
        for object in &mut self.objects {
            object.object.restore_texture(ctx);
        }
        if let Some(object) = &mut self.inserted {
            object.restore_texture(ctx);
        }
        self.upload_canvas(ctx);
        self.shape_render_key = None;
        self.text_dirty = true;
        self.update_shape();
    }

    fn open_path(&mut self, ctx: &egui::Context, path: PathBuf) -> Result<()> {
        let is_project = path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("qip") || extension.eq_ignore_ascii_case("quipit")
        });
        let (pixels, objects) = if is_project {
            project::load(&path, ctx).map_err(io_error)?
        } else {
            let pixels = image::open(&path).map_err(io_error)?.to_rgba8();
            dimensions(pixels.width(), pixels.height(), 16384)?;
            (pixels, Vec::new())
        };
        self.pixels = Some(pixels);
        self.objects = objects;
        self.project_path = is_project.then(|| path.clone());
        self.path = (!is_project).then_some(path);
        self.undo.clear();
        self.redo.clear();
        self.dirty = false;
        self.lock_ratio = true;
        self.refresh(ctx);
        self.status = if is_project {
            "Project opened"
        } else {
            "Image opened"
        }
        .into();
        Ok(())
    }

    fn save_path(&mut self, mut path: PathBuf) -> Result<()> {
        self.require_document()?;
        self.update_shape();
        if !path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("qip") || extension.eq_ignore_ascii_case("quipit")
        }) {
            path.set_extension("qip");
        }
        let objects = self.document_objects();
        project::save(&path, self.pixels.as_ref().unwrap(), &objects).map_err(io_error)?;
        self.status = format!("Project saved: {}", path.display());
        self.project_path = Some(path);
        self.dirty = false;
        Ok(())
    }

    fn document_objects(&self) -> Vec<objects::Object> {
        let mut objects = self.objects.clone();
        if let Some(object) = &self.inserted {
            let position = self
                .object_before
                .as_ref()
                .map_or(objects.len(), |(index, _)| *index);
            let name = self
                .selected_object_name()
                .unwrap_or_else(|| "Object".into());
            let visible = self
                .object_before
                .as_ref()
                .is_none_or(|(_, original)| original.visible);
            objects.insert(
                position,
                objects::Object {
                    name,
                    visible,
                    object: object.clone(),
                    data: self.active_text.clone(),
                    shape: self.active_shape,
                    rotation: self.shape_rotation,
                    curve_edit: self.curve_edit,
                },
            );
        }
        objects
    }

    fn export_path(&mut self, path: PathBuf) -> Result<()> {
        self.require_document()?;
        self.update_shape();
        let pixels = self.composed_canvas().unwrap();
        let jpeg = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg")
            });
        if jpeg {
            image::RgbImage::from_fn(pixels.width(), pixels.height(), |x, y| {
                let pixel = pixels.get_pixel(x, y).0;
                image::Rgb(std::array::from_fn(|channel| {
                    ((pixel[channel] as u32 * pixel[3] as u32 + 255 * (255 - pixel[3] as u32))
                        / 255) as u8
                }))
            })
            .save(&path)
            .map_err(io_error)?;
        } else {
            pixels.save(&path).map_err(io_error)?;
        }
        self.status = format!("Saved {}", path.display());
        self.path = Some(path);
        Ok(())
    }

    pub(crate) fn run_command(&mut self, ctx: &egui::Context, command: Command) {
        if let Err(error) = self.execute(ctx, command) {
            self.status = error.to_string();
        }
    }
}

impl ObjectPatch {
    fn apply(self, draft: &mut EditDraft) -> Result<()> {
        if let Some(bounds) = self.bounds {
            draft.bounds = bounds;
        }
        if let Some(patch) = self.shape {
            let Some((_, style)) = &mut draft.shape else {
                return Err(error(
                    ErrorCode::WrongObjectType,
                    "Shape properties require a shape",
                ));
            };
            if let Some(value) = patch.border_width {
                number(value, 0.0, 64.0, "Border width")?;
                style.border_width = value;
            }
            if let Some(value) = patch.border {
                style.border = color(value);
            }
            if let Some(value) = patch.fill {
                style.fill = color(value);
            }
            if let Some(value) = patch.transparency {
                style.transparency = value;
            }
        }
        if let Some(patch) = self.text {
            let Some(data) = &mut draft.text else {
                return Err(error(
                    ErrorCode::WrongObjectType,
                    "Text properties require a text object",
                ));
            };
            if let Some(content) = patch.content {
                if content.len() > 1_000_000 {
                    return Err(invalid("Text content exceeds 1 MB"));
                }
                data.content = content;
                data.cursor = data.content.len();
                data.anchor = None;
                data.undo.clear();
                data.redo.clear();
            }
            if let Some(font) = patch.font {
                match font {
                    FontSource::Sans => {
                        data.font = text::sans();
                        data.font_name = "Sans".into();
                    }
                    FontSource::Monospace => {
                        data.font = text::mono();
                        data.font_name = "Monospace".into();
                    }
                    FontSource::File { path } => {
                        data.font = FontArc::try_from_vec(std::fs::read(&path).map_err(io_error)?)
                            .map_err(io_error)?;
                        data.font_name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                    }
                }
            }
            if let Some(value) = patch.size {
                number(value, 4.0, 512.0, "Font size")?;
                data.size = value;
            }
            if let Some(value) = patch.bold {
                data.bold = value;
            }
            if let Some(value) = patch.italic {
                data.italic = value;
            }
            if let Some(value) = patch.color {
                data.color = color(value);
            }
            if let Some(value) = patch.border {
                data.border = color(value);
            }
            if let Some(value) = patch.border_width {
                number(value, 0.0, 64.0, "Border width")?;
                data.border_width = value;
            }
            if let Some(value) = patch.fill {
                data.fill = color(value);
            }
            if let Some(value) = patch.alignment {
                data.alignment = value;
            }
            if let Some(patch) = patch.shadow {
                if let Some(value) = patch.enabled {
                    data.shadow = value;
                }
                if let Some(value) = patch.color {
                    data.shadow_color = color(value);
                }
                if let Some(value) = patch.offset {
                    number(value.x, -32.0, 32.0, "Text shadow X")?;
                    number(value.y, -32.0, 32.0, "Text shadow Y")?;
                    data.shadow_offset = value;
                }
                if let Some(value) = patch.blur {
                    number(value, 0.0, 16.0, "Text shadow blur")?;
                    data.shadow_blur = value;
                }
            }
            draft.text_changed = true;
        }
        if let Some(patch) = self.image {
            if draft.shape.is_some() || draft.text.is_some() {
                return Err(error(
                    ErrorCode::WrongObjectType,
                    "Image properties require an imported image",
                ));
            }
            let style = draft.image.get_or_insert_default();
            if let Some(value) = patch.border_width {
                number(value, 0.0, 64.0, "Border width")?;
                style.border_width = value;
            }
            if let Some(value) = patch.border {
                style.border = color(value);
            }
            if let Some(value) = patch.border_transparency {
                style.border_transparency = value;
            }
            if let Some(value) = patch.transparency {
                style.image_transparency = value;
            }
        }
        if let Some(patch) = self.shadow {
            let style = draft.image.get_or_insert_default();
            if let Some(value) = patch.enabled {
                style.shadow = value;
            }
            if let Some(value) = patch.color {
                style.shadow_color = color(value);
            }
            if let Some(value) = patch.offset {
                style.shadow_offset = value;
            }
            if let Some(value) = patch.blur {
                style.shadow_blur = value;
            }
        }
        Ok(())
    }
}
