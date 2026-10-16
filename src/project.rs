use crate::{
    insert::{ImageStyle, InsertedImage},
    objects::Object,
    shapes, text,
};
use eframe::egui::{self, Rect};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::{
    io::{Cursor, Read, Write},
    path::Path,
};

#[derive(Serialize, Deserialize)]
struct Project {
    version: u32,
    background: Vec<u8>,
    objects: Vec<SavedObject>,
}
#[derive(Serialize, Deserialize)]
struct SavedObject {
    name: String,
    visible: bool,
    pixels: Vec<u8>,
    rect: Rect,
    image_style: Option<ImageStyle>,
    text: Option<text::TextData>,
    shape: Option<(shapes::ShapeKind, shapes::Style)>,
    rotation: f32,
    curve_edit: bool,
}
fn png(pixels: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut bytes = Cursor::new(Vec::new());
    pixels
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(bytes.into_inner())
}
fn decode(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map(|i| i.to_rgba8())
        .map_err(|e| e.to_string())
}
pub fn save(path: &Path, background: &RgbaImage, objects: &[Object]) -> Result<(), String> {
    let objects = objects
        .iter()
        .map(|o| {
            Ok(SavedObject {
                name: o.name.clone(),
                visible: o.visible,
                pixels: png(&o.object.pixels)?,
                rect: o.object.rect,
                image_style: o.object.image_style,
                text: o.data.clone(),
                shape: o.shape,
                rotation: o.rotation,
                curve_edit: o.curve_edit,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let project = Project {
        version: 1,
        background: png(background)?,
        objects,
    };
    let bytes = serde_json::to_vec(&project).map_err(|e| e.to_string())?;
    if bytes.len() > 256 * 1024 * 1024 {
        return Err("Project exceeds 256 MB".into());
    }
    // Persist from the same directory so replacing an existing project is atomic.
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temporary.write_all(&bytes).map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn load(path: &Path, ctx: &egui::Context) -> Result<(RgbaImage, Vec<Object>), String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 256 * 1024 * 1024 {
        return Err("Project exceeds 256 MB".into());
    }
    let project: Project = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if project.version != 1 {
        return Err("Unsupported QuipIt project version".into());
    }
    let background = decode(&project.background)?;
    let canvas = egui::vec2(background.width() as f32, background.height() as f32);
    let mut objects = Vec::new();
    for saved in project.objects {
        if !saved.rect.is_finite()
            || saved.rect.width() <= 0.0
            || saved.rect.height() <= 0.0
            || saved.rect.width() > 16384.0
            || saved.rect.height() > 16384.0
        {
            return Err("Invalid object dimensions".into());
        }
        let mut object = InsertedImage::new(decode(&saved.pixels)?, canvas, ctx);
        object.rect = saved.rect;
        object.image_style = saved.image_style;
        object.prepare();
        let mut data = saved.text;
        if let Some(data) = &mut data {
            data.cursor = data.content.len();
            data.anchor = None;
            data.undo.clear();
            data.redo.clear();
        }
        objects.push(Object {
            name: saved.name,
            visible: saved.visible,
            object,
            data,
            shape: saved.shape,
            rotation: saved.rotation,
            curve_edit: saved.curve_edit,
        });
    }
    Ok((background, objects))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_and_malformed_projects_are_rejected() {
        let ctx = egui::Context::default();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            file.path(),
            br#"{"version":999,"background":[],"objects":[]}"#,
        )
        .unwrap();
        assert!(
            load(file.path(), &ctx)
                .err()
                .unwrap()
                .contains("Unsupported")
        );
        std::fs::write(file.path(), b"not a project").unwrap();
        assert!(load(file.path(), &ctx).is_err());
    }

    #[test]
    fn project_save_replaces_existing_file_and_keeps_transparency() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("test.qip");
        let ctx = egui::Context::default();
        let first = RgbaImage::new(3, 4);
        save(&file, &first, &[]).unwrap();
        let second = RgbaImage::from_pixel(5, 2, image::Rgba([23, 45, 67, 89]));
        save(&file, &second, &[]).unwrap();
        let (loaded, objects) = load(&file, &ctx).unwrap();
        assert_eq!(loaded, second);
        assert!(objects.is_empty());
    }
}
