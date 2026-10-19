#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    eframe::run_native(
        "QuipIt",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_title("QuipIt - untitled.qip")
                .with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!("../assets/quipit-256.png"))
                        .expect("Bundled application icon must be a valid PNG"),
                )
                .with_inner_size([1150.0, 780.0]),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::<quipit::QuipIt>::default())),
    )
}
