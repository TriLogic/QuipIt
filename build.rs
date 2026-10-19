fn main() {
    println!("cargo:rerun-if-changed=assets/quipit.rc");
    println!("cargo:rerun-if-changed=assets/quipit.ico");
    #[cfg(windows)]
    embed_windows_icon();
}

#[cfg(windows)]
fn embed_windows_icon() {
    let target = std::env::var("TARGET").expect("Cargo target");
    if !target.contains("windows") {
        return;
    }
    let output =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
    let resource = output.join("quipit.res");
    let status = cc::windows_registry::find_tool(&target, "rc.exe")
        .expect("Windows SDK resource compiler rc.exe is required to embed the application icon")
        .to_command()
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg("assets/quipit.rc")
        .status()
        .expect("Run Windows resource compiler");
    assert!(status.success(), "Compiling the application icon failed");
    println!("cargo:rustc-link-arg-bin=quipit={}", resource.display());
}
