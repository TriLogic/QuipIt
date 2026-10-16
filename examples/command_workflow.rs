use quipit::{QuipIt, commands::Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .ok_or("Pass an output PNG path")?;
    let ctx = egui::Context::default();
    let mut editor = QuipIt::default();
    editor.execute(
        &ctx,
        Command::NewDocument {
            width: 480,
            height: 360,
            background: [255; 4],
        },
    )?;
    let command = serde_json::from_str(
        r#"{
        "command": "batch",
        "commands": [
            {"command":"add_shape", "kind":"Star5"},
            {"command":"update_object", "target":{"kind":"selected"}, "patch":{
                "shape":{"fill":[255,200,0,255]},
                "shadow":{"enabled":true,"color":[0,0,0,128],"offset":{"x":6,"y":6},"blur":4}
            }},
            {"command":"rename", "target":{"kind":"selected"}, "name":"Featured star"}
        ]
    }"#,
    )?;
    editor.execute(&ctx, command)?;
    editor.execute(
        &ctx,
        Command::ExportImage {
            path: output.into(),
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&editor.inspect())?);
    Ok(())
}
