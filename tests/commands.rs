use egui::{Context, Pos2, Rect, Vec2};
use quipit::{QuipIt, commands::*, shapes::ShapeKind, text::Kind};

fn editor() -> (QuipIt, Context) {
    let ctx = Context::default();
    let mut editor = QuipIt::default();
    editor
        .execute(
            &ctx,
            Command::NewDocument {
                width: 240,
                height: 200,
                background: [0; 4],
            },
        )
        .unwrap();
    (editor, ctx)
}

fn json(editor: &mut QuipIt, ctx: &Context, value: serde_json::Value) -> DocumentInfo {
    editor
        .execute(ctx, serde_json::from_value(value).unwrap())
        .unwrap()
}

fn state(editor: &QuipIt) -> serde_json::Value {
    serde_json::to_value(editor.inspect()).unwrap()
}

#[test]
fn commands_are_strict_json_and_invalid_edits_leave_the_document_unchanged() {
    let ctx = Context::default();
    let mut empty = QuipIt::default();
    let error = empty
        .execute(
            &ctx,
            Command::AddShape {
                kind: ShapeKind::Circle,
            },
        )
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::NoDocument);
    assert!(empty.inspect().dimensions.is_none());
    assert!(
        serde_json::from_str::<Command>(
            r#"{"command":"resize","width":100,"height":100,"typo":true}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<Command>(
            r#"{"command":"update_object","target":{"kind":"selected"},"patch":{"shdaow":{}}}"#
        )
        .is_err()
    );

    let (mut editor, ctx) = editor();
    editor
        .execute(
            &ctx,
            Command::AddShape {
                kind: ShapeKind::Circle,
            },
        )
        .unwrap();
    let before = state(&editor);
    let pixels = editor.render_preview().unwrap();
    for patch in [
        ObjectPatch {
            bounds: Some(Rect::from_min_size(Pos2::ZERO, Vec2::ZERO)),
            ..Default::default()
        },
        ObjectPatch {
            shape: Some(ShapePatch {
                transparency: Some(f32::NAN),
                ..Default::default()
            }),
            ..Default::default()
        },
        ObjectPatch {
            shape: Some(ShapePatch {
                fill: Some([255, 0, 0, 255]),
                ..Default::default()
            }),
            shadow: Some(ShadowPatch {
                blur: Some(-1.0),
                ..Default::default()
            }),
            ..Default::default()
        },
        ObjectPatch {
            text: Some(TextPatch {
                content: Some("Wrong kind".into()),
                ..Default::default()
            }),
            ..Default::default()
        },
    ] {
        assert!(
            editor
                .execute(
                    &ctx,
                    Command::UpdateObject {
                        target: Target::Selected,
                        patch
                    }
                )
                .is_err()
        );
        assert_eq!(state(&editor), before);
        assert!(editor.render_preview().unwrap() == pixels);
    }
    for rect in [[u32::MAX, 0, 2, 2], [0, 0, 0, 1], [0, 0, 241, 200]] {
        assert!(editor.execute(&ctx, Command::Crop { rect }).is_err());
        assert_eq!(state(&editor), before);
    }
    assert!(
        editor
            .execute(
                &ctx,
                Command::OpenDocument {
                    path: "missing-project.qip".into()
                }
            )
            .is_err()
    );
    assert_eq!(state(&editor), before);
}

#[test]
fn every_shape_and_text_kind_can_be_created_styled_and_transformed_without_ui() {
    let (mut editor, ctx) = editor();
    for kind in ShapeKind::LINES.into_iter().chain(ShapeKind::SHAPES) {
        editor.execute(&ctx, Command::AddShape { kind }).unwrap();
        editor
            .execute(
                &ctx,
                Command::UpdateObject {
                    target: Target::Selected,
                    patch: ObjectPatch {
                        shape: Some(ShapePatch {
                            fill: Some([12, 34, 56, 180]),
                            border: Some([200, 40, 20, 255]),
                            border_width: Some(4.0),
                            transparency: Some(20.0),
                        }),
                        shadow: Some(ShadowPatch {
                            enabled: Some(true),
                            color: Some([0, 0, 0, 128]),
                            offset: Some(Vec2::new(-8.0, 6.0)),
                            blur: Some(3.0),
                        }),
                        ..Default::default()
                    },
                },
            )
            .unwrap();
        editor
            .execute(
                &ctx,
                Command::Transform {
                    target: Target::Selected,
                    degrees: 30.0,
                    horizontal: true,
                    vertical: false,
                },
            )
            .unwrap();
        let view = editor.inspect();
        let shape = view.objects.last().unwrap();
        assert!(shape.shape.unwrap().0 == kind);
        assert!(shape.image.unwrap().shadow);
        assert!(shape.bounds.is_finite());
        editor.execute(&ctx, Command::Apply).unwrap();
    }
    for kind in [Kind::Area, Kind::Speech, Kind::Thought] {
        editor.execute(&ctx, Command::AddText { kind }).unwrap();
        editor
            .execute(
                &ctx,
                Command::UpdateObject {
                    target: Target::Selected,
                    patch: ObjectPatch {
                        text: Some(TextPatch {
                            content: Some("Hello, 世界!\nEditable text".into()),
                            size: Some(24.0),
                            font: Some(FontSource::Monospace),
                            bold: Some(true),
                            italic: Some(true),
                            alignment: Some(1),
                            color: Some([20, 30, 40, 220]),
                            shadow: Some(ShadowPatch {
                                enabled: Some(true),
                                blur: Some(2.0),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                },
            )
            .unwrap();
        editor
            .execute(
                &ctx,
                Command::Transform {
                    target: Target::Selected,
                    degrees: 15.0,
                    horizontal: true,
                    vertical: true,
                },
            )
            .unwrap();
        let view = editor.inspect();
        let data = view.objects.last().unwrap().text.as_ref().unwrap();
        assert_eq!(data.content, "Hello, 世界!\nEditable text");
        assert!(
            data.bold && data.italic && data.flip_horizontal && data.flip_vertical && data.shadow
        );
        assert_eq!(data.font_name, "Monospace");
        editor.execute(&ctx, Command::Apply).unwrap();
    }
    assert_eq!(editor.inspect().objects.len(), 25);
    assert!(
        editor
            .render_preview()
            .unwrap()
            .pixels()
            .any(|pixel| pixel[3] > 0)
    );
}

#[test]
fn json_workflow_saves_stable_ids_and_exports_the_same_preview() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("commands.qip");
    let png = directory.path().join("commands.png");
    let (mut editor, ctx) = editor();
    let created = json(
        &mut editor,
        &ctx,
        serde_json::json!({"command":"add_shape","kind":"Star5"}),
    );
    let id = created.selected.unwrap();
    json(
        &mut editor,
        &ctx,
        serde_json::json!({
            "command":"update_object", "target":{"kind":"object","id":id},
            "patch":{"shadow":{"enabled":true,"offset":{"x":8,"y":6},"blur":3}}
        }),
    );
    json(
        &mut editor,
        &ctx,
        serde_json::json!({"command":"rename","target":{"kind":"selected"},"name":"Featured star"}),
    );
    let preview = editor.render_preview().unwrap();
    editor
        .execute(&ctx, Command::SaveProject { path: path.clone() })
        .unwrap();
    editor
        .execute(&ctx, Command::ExportImage { path: png.clone() })
        .unwrap();
    assert!(image::open(&png).unwrap().to_rgba8() == preview);
    let mut reopened = QuipIt::default();
    reopened
        .execute(&ctx, Command::OpenDocument { path })
        .unwrap();
    let view = reopened.inspect();
    assert_eq!(view.objects[0].id, id);
    assert_eq!(view.objects[0].name, "Featured star");
    assert!(reopened.render_preview().unwrap() == preview);
    reopened
        .execute(&ctx, Command::SelectObject { id })
        .unwrap();
    reopened
        .execute(
            &ctx,
            Command::Rename {
                target: Target::Selected,
                name: "Canceled name".into(),
            },
        )
        .unwrap();
    reopened
        .execute(
            &ctx,
            Command::UpdateObject {
                target: Target::Selected,
                patch: ObjectPatch {
                    shadow: Some(ShadowPatch {
                        enabled: Some(false),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            },
        )
        .unwrap();
    reopened.execute(&ctx, Command::Cancel).unwrap();
    assert_eq!(reopened.inspect().objects[0].name, "Featured star");
    assert!(reopened.render_preview().unwrap() == preview);
    reopened
        .execute(
            &ctx,
            Command::Duplicate {
                target: Target::Object { id },
            },
        )
        .unwrap();
    let objects = reopened.inspect().objects;
    assert_eq!(objects.len(), 2);
    assert_ne!(objects[0].id, objects[1].id);
}

#[test]
fn batches_are_atomic_and_undo_and_redo_as_one_action() {
    let (mut editor, ctx) = editor();
    editor
        .execute(
            &ctx,
            Command::AddShape {
                kind: ShapeKind::Square,
            },
        )
        .unwrap();
    let id = editor.inspect().selected.unwrap();
    editor.execute(&ctx, Command::Apply).unwrap();
    let before = state(&editor);
    let pixels = editor.render_preview().unwrap();
    editor
        .execute(
            &ctx,
            Command::Batch {
                commands: vec![
                    Command::UpdateObject {
                        target: Target::Object { id },
                        patch: ObjectPatch {
                            shadow: Some(ShadowPatch {
                                enabled: Some(true),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    },
                    Command::Rename {
                        target: Target::Selected,
                        name: "Shadowed square".into(),
                    },
                    Command::AddText { kind: Kind::Speech },
                    Command::UpdateObject {
                        target: Target::Selected,
                        patch: ObjectPatch {
                            text: Some(TextPatch {
                                content: Some("One transaction".into()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    },
                ],
            },
        )
        .unwrap();
    assert_eq!(editor.inspect().objects.len(), 2);
    assert!(editor.inspect().selected.is_none());
    let after = state(&editor);
    let after_pixels = editor.render_preview().unwrap();
    editor.execute(&ctx, Command::Undo).unwrap();
    assert_eq!(state(&editor)["objects"], before["objects"]);
    assert!(editor.render_preview().unwrap() == pixels);
    editor.execute(&ctx, Command::Redo).unwrap();
    assert_eq!(state(&editor)["objects"], after["objects"]);
    assert!(editor.render_preview().unwrap() == after_pixels);
    let before_failure = state(&editor);
    let failed = editor.execute(
        &ctx,
        Command::Batch {
            commands: vec![
                Command::SetBackground { color: [255; 4] },
                Command::Delete {
                    target: Target::Object { id },
                },
                Command::Resize {
                    width: 0,
                    height: 10,
                },
            ],
        },
    );
    assert_eq!(failed.err().unwrap().code, ErrorCode::InvalidArgument);
    assert_eq!(state(&editor), before_failure);
    assert!(editor.render_preview().unwrap() == after_pixels);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("must-not-write.png");
    assert!(
        editor
            .execute(
                &ctx,
                Command::Batch {
                    commands: vec![Command::ExportImage { path: path.clone() }]
                }
            )
            .is_err()
    );
    assert!(!path.exists());
}

#[test]
fn ids_target_the_same_objects_during_selection_reordering_deletion_and_history() {
    let (mut editor, ctx) = editor();
    let mut ids = Vec::new();
    for kind in [ShapeKind::Circle, ShapeKind::Triangle, ShapeKind::Heart] {
        editor.execute(&ctx, Command::AddShape { kind }).unwrap();
        ids.push(editor.inspect().selected.unwrap());
        editor.execute(&ctx, Command::Apply).unwrap();
    }
    editor
        .execute(&ctx, Command::SelectObject { id: ids[0] })
        .unwrap();
    editor
        .execute(
            &ctx,
            Command::Delete {
                target: Target::Object { id: ids[2] },
            },
        )
        .unwrap();
    assert_eq!(
        editor
            .inspect()
            .objects
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>(),
        ids[..2]
    );
    editor.execute(&ctx, Command::Undo).unwrap();
    assert_eq!(
        editor
            .inspect()
            .objects
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>(),
        ids
    );
    editor
        .execute(
            &ctx,
            Command::Reorder {
                target: Target::Object { id: ids[0] },
                index: 2,
            },
        )
        .unwrap();
    assert_eq!(editor.inspect().objects[2].id, ids[0]);
    editor
        .execute(
            &ctx,
            Command::SetVisibility {
                target: Target::Object { id: ids[1] },
                visible: false,
            },
        )
        .unwrap();
    assert!(
        !editor
            .inspect()
            .objects
            .iter()
            .find(|object| object.id == ids[1])
            .unwrap()
            .visible
    );
}

#[test]
fn image_font_curve_crop_and_resize_commands_work_without_dialogs() {
    let directory = tempfile::tempdir().unwrap();
    let png = directory.path().join("source.png");
    image::RgbaImage::from_pixel(40, 30, image::Rgba([10, 20, 30, 180]))
        .save(&png)
        .unwrap();
    let font = directory.path().join("custom.ttf");
    std::fs::write(&font, epaint_default_fonts::UBUNTU_LIGHT).unwrap();
    let (mut editor, ctx) = editor();
    editor
        .execute(&ctx, Command::InsertImage { path: png })
        .unwrap();
    editor
        .execute(
            &ctx,
            Command::UpdateObject {
                target: Target::Selected,
                patch: ObjectPatch {
                    image: Some(ImagePatch {
                        border_width: Some(3.0),
                        border: Some([255, 0, 0, 255]),
                        border_transparency: Some(20.0),
                        transparency: Some(30.0),
                    }),
                    ..Default::default()
                },
            },
        )
        .unwrap();
    editor
        .execute(&ctx, Command::AddText { kind: Kind::Area })
        .unwrap();
    editor
        .execute(
            &ctx,
            Command::UpdateObject {
                target: Target::Selected,
                patch: ObjectPatch {
                    text: Some(TextPatch {
                        font: Some(FontSource::File { path: font }),
                        content: Some("Custom font".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            },
        )
        .unwrap();
    assert_eq!(
        editor
            .inspect()
            .objects
            .last()
            .unwrap()
            .text
            .as_ref()
            .unwrap()
            .font_name,
        "custom"
    );
    editor
        .execute(
            &ctx,
            Command::AddShape {
                kind: ShapeKind::Curve4,
            },
        )
        .unwrap();
    let id = editor.inspect().selected.unwrap();
    editor
        .execute(
            &ctx,
            Command::MoveCurvePoint {
                target: Target::Selected,
                index: 0,
                position: egui::pos2(20.0, 20.0),
            },
        )
        .unwrap();
    editor
        .execute(&ctx, Command::SetCurveEditing { enabled: false })
        .unwrap();
    editor
        .execute(
            &ctx,
            Command::Crop {
                rect: [10, 10, 200, 160],
            },
        )
        .unwrap();
    let bounds = editor
        .inspect()
        .objects
        .iter()
        .find(|object| object.id == id)
        .unwrap()
        .bounds;
    editor
        .execute(
            &ctx,
            Command::Resize {
                width: 100,
                height: 80,
            },
        )
        .unwrap();
    let view = editor.inspect();
    assert_eq!(view.dimensions, Some([100, 80]));
    let resized = view
        .objects
        .iter()
        .find(|object| object.id == id)
        .unwrap()
        .bounds;
    assert!((resized.width() - bounds.width() / 2.0).abs() < 0.01);
}

#[test]
fn text_selection_insertion_deletion_and_local_history_preserve_unicode() {
    let (mut editor, ctx) = editor();
    editor
        .execute(&ctx, Command::AddText { kind: Kind::Area })
        .unwrap();
    let insert = |content: &str| Command::EditText {
        target: Target::Selected,
        edit: TextEdit::Insert {
            content: content.into(),
        },
    };
    editor.execute(&ctx, insert("A🙂B")).unwrap();
    editor
        .execute(
            &ctx,
            Command::EditText {
                target: Target::Selected,
                edit: TextEdit::Select { start: 1, end: 5 },
            },
        )
        .unwrap();
    let view = editor.inspect();
    let text = view.objects[0].text.as_ref().unwrap();
    assert_eq!(text.selected_text, "🙂");
    assert_eq!(text.selection, [1, 5]);
    assert!(
        !serde_json::to_value(text)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("font")
    );
    let selected = state(&editor);
    assert!(
        editor
            .execute(
                &ctx,
                Command::EditText {
                    target: Target::Selected,
                    edit: TextEdit::Select { start: 2, end: 5 }
                }
            )
            .is_err()
    );
    assert_eq!(state(&editor), selected);
    editor.execute(&ctx, insert("é\r\n")).unwrap();
    assert_eq!(
        editor.inspect().objects[0].text.as_ref().unwrap().content,
        "Aé\nB"
    );
    editor
        .execute(
            &ctx,
            Command::EditText {
                target: Target::Selected,
                edit: TextEdit::Undo,
            },
        )
        .unwrap();
    assert_eq!(
        editor.inspect().objects[0].text.as_ref().unwrap().content,
        "A🙂B"
    );
    editor
        .execute(
            &ctx,
            Command::EditText {
                target: Target::Selected,
                edit: TextEdit::Redo,
            },
        )
        .unwrap();
    editor
        .execute(
            &ctx,
            Command::EditText {
                target: Target::Selected,
                edit: TextEdit::Backspace { word: false },
            },
        )
        .unwrap();
    assert_eq!(
        editor.inspect().objects[0].text.as_ref().unwrap().content,
        "AéB"
    );
    editor
        .execute(
            &ctx,
            Command::EditText {
                target: Target::Selected,
                edit: TextEdit::Delete { word: false },
            },
        )
        .unwrap();
    assert_eq!(
        editor.inspect().objects[0].text.as_ref().unwrap().content,
        "Aé"
    );
}

#[test]
fn failed_batches_restore_an_active_text_edit_and_its_selection() {
    let (mut editor, ctx) = editor();
    editor
        .execute(&ctx, Command::AddText { kind: Kind::Speech })
        .unwrap();
    editor
        .execute(
            &ctx,
            Command::EditText {
                target: Target::Selected,
                edit: TextEdit::Insert {
                    content: "Keep this draft".into(),
                },
            },
        )
        .unwrap();
    editor
        .execute(
            &ctx,
            Command::EditText {
                target: Target::Selected,
                edit: TextEdit::Select { start: 0, end: 4 },
            },
        )
        .unwrap();
    let before = state(&editor);
    let pixels = editor.render_preview().unwrap();
    assert!(
        editor
            .execute(
                &ctx,
                Command::Batch {
                    commands: vec![
                        Command::EditText {
                            target: Target::Selected,
                            edit: TextEdit::Insert {
                                content: "Replace".into()
                            }
                        },
                        Command::AddShape {
                            kind: ShapeKind::Square
                        },
                        Command::UpdateObject {
                            target: Target::Selected,
                            patch: ObjectPatch {
                                text: Some(TextPatch {
                                    size: Some(30.0),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }
                        },
                    ]
                }
            )
            .is_err()
    );
    assert_eq!(state(&editor), before);
    assert!(editor.render_preview().unwrap() == pixels);
}

#[test]
fn older_projects_receive_unique_ids_and_duplicate_saved_ids_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.qip");
    let (mut editor, ctx) = editor();
    for kind in [ShapeKind::Square, ShapeKind::Circle] {
        editor.execute(&ctx, Command::AddShape { kind }).unwrap();
    }
    editor
        .execute(&ctx, Command::SaveProject { path: path.clone() })
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let mut legacy: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for object in legacy["objects"].as_array_mut().unwrap() {
        object.as_object_mut().unwrap().remove("id");
    }
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    editor
        .execute(&ctx, Command::OpenDocument { path: path.clone() })
        .unwrap();
    let objects = editor.inspect().objects;
    assert_ne!(objects[0].id, objects[1].id);
    let mut duplicate: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    duplicate["objects"][1]["id"] = duplicate["objects"][0]["id"].clone();
    std::fs::write(&path, serde_json::to_vec(&duplicate).unwrap()).unwrap();
    let before = state(&editor);
    assert!(
        editor
            .execute(&ctx, Command::OpenDocument { path })
            .is_err()
    );
    assert_eq!(state(&editor), before);
}

#[test]
fn describe_and_count_report_background_and_include_pending_and_hidden_objects_once() {
    let (mut app, ctx) = editor();
    json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"add_shape","kind":"Square"}),
    );
    json(&mut app, &ctx, serde_json::json!({"command":"apply"}));
    let shape_id = app.inspect().objects[0].id;
    app.execute(
        &ctx,
        Command::SetVisibility {
            target: Target::Object { id: shape_id },
            visible: false,
        },
    )
    .unwrap();
    json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"add_text","kind":"Area"}),
    );
    let before = state(&app);
    let info = json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"describe_document"}),
    );
    assert_eq!(info.dimensions, Some([240, 200]));
    assert_eq!(
        serde_json::to_value(info.background).unwrap(),
        serde_json::json!({"kind":"uniform","color":[0,0,0,0]})
    );
    assert_eq!(info.object_counts.total, 2);
    assert_eq!(info.object_counts.shapes, 1);
    assert_eq!(info.object_counts.text, 1);
    assert_eq!(info.object_counts.visible, 1);
    assert_eq!(info.object_counts.hidden, 1);
    for (query, expected) in [
        (serde_json::json!({"command":"count_objects"}), 2),
        (
            serde_json::json!({"command":"count_objects","kind":"shape"}),
            1,
        ),
        (
            serde_json::json!({"command":"count_objects","kind":"shape","visible":true}),
            0,
        ),
        (
            serde_json::json!({"command":"count_objects","visible":false}),
            1,
        ),
        (
            serde_json::json!({"command":"count_objects","kind":"text","visible":true}),
            1,
        ),
        (
            serde_json::json!({"command":"count_objects","kind":"image"}),
            0,
        ),
    ] {
        assert_eq!(json(&mut app, &ctx, query).query_count, Some(expected));
    }
    assert_eq!(
        state(&app),
        before,
        "queries must not edit the document or history"
    );
    let mut empty = QuipIt::default();
    let info = empty.execute(&ctx, Command::DescribeDocument).unwrap();
    assert!(info.dimensions.is_none());
    assert!(info.background.is_none());
    assert_eq!(info.object_counts.total, 0);
}

#[test]
fn purge_clears_both_history_stacks_preserves_preview_and_future_edits_are_undoable() {
    let (mut app, ctx) = editor();
    json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"add_shape","kind":"Circle"}),
    );
    json(&mut app, &ctx, serde_json::json!({"command":"apply"}));
    json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"add_shape","kind":"Square"}),
    );
    json(&mut app, &ctx, serde_json::json!({"command":"apply"}));
    json(&mut app, &ctx, serde_json::json!({"command":"undo"}));
    assert!(app.inspect().can_undo && app.inspect().can_redo);
    let preview = app.render_preview().unwrap();
    let objects = serde_json::to_value(app.inspect().objects).unwrap();
    let info = json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"purge_history"}),
    );
    assert!(!info.can_undo && !info.can_redo);
    assert_eq!((info.undo_steps, info.redo_steps), (0, 0));
    assert_eq!(app.render_preview().unwrap(), preview);
    assert_eq!(serde_json::to_value(info.objects).unwrap(), objects);
    json(&mut app, &ctx, serde_json::json!({"command":"undo"}));
    json(&mut app, &ctx, serde_json::json!({"command":"redo"}));
    assert_eq!(app.render_preview().unwrap(), preview);
    json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"batch","commands":[{"command":"add_shape","kind":"Star5"}]}),
    );
    assert!(app.inspect().can_undo);
    json(&mut app, &ctx, serde_json::json!({"command":"undo"}));
    assert_eq!(app.render_preview().unwrap(), preview);
    let before = state(&app);
    let error = app
        .execute(
            &ctx,
            Command::Batch {
                commands: vec![Command::PurgeHistory],
            },
        )
        .err()
        .unwrap();
    assert!(matches!(error.code, ErrorCode::Unsupported));
    assert_eq!(state(&app), before);
}

#[test]
fn duplicate_selects_copy_for_chained_renaming_and_one_step_undo() {
    let (mut app, ctx) = editor();
    json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"batch","commands":[
            {"command":"add_shape","kind":"Hexagon"},
            {"command":"rename","target":{"kind":"selected"},"name":"Original hexagon"}
        ]}),
    );
    let original = app.inspect().objects[0].id;
    assert!(app.inspect().selected.is_none());
    let info = json(
        &mut app,
        &ctx,
        serde_json::json!({"command":"batch","commands":[
            {"command":"select_object","id":original},
            {"command":"rename","target":{"kind":"selected"},"name":"Hex 1"},
            {"command":"duplicate","target":{"kind":"selected"}},
            {"command":"rename","target":{"kind":"selected"},"name":"Hex 2"},
            {"command":"duplicate","target":{"kind":"selected"}},
            {"command":"rename","target":{"kind":"selected"},"name":"Hex 3"}
        ]}),
    );
    assert_eq!(
        info.objects
            .iter()
            .map(|o| o.name.as_str())
            .collect::<Vec<_>>(),
        ["Hex 1", "Hex 2", "Hex 3"]
    );
    assert_eq!(info.objects[0].id, original);
    let ids: std::collections::HashSet<_> = info.objects.iter().map(|o| o.id).collect();
    assert_eq!(ids.len(), 3);
    assert!(
        info.objects
            .iter()
            .all(|o| o.shape.unwrap().0 == ShapeKind::Hexagon)
    );
    json(&mut app, &ctx, serde_json::json!({"command":"undo"}));
    assert_eq!(app.inspect().objects.len(), 1);
    assert_eq!(app.inspect().objects[0].name, "Original hexagon");
    json(&mut app, &ctx, serde_json::json!({"command":"redo"}));
    assert_eq!(app.inspect().objects.len(), 3);
    let info = app
        .execute(
            &ctx,
            Command::Duplicate {
                target: Target::Object { id: original },
            },
        )
        .unwrap();
    let selected = info.selected.unwrap();
    assert_ne!(selected, original);
    assert!(
        info.objects
            .iter()
            .any(|object| object.id == selected && object.editing)
    );
}
