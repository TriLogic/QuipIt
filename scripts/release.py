"""Build portable release archives using Python's standard library."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tarfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {
    "windows-x64": ("x86_64-pc-windows-msvc", "quipit.exe"),
    "linux-x64": ("x86_64-unknown-linux-gnu", "quipit"),
    "macos-apple-silicon": ("aarch64-apple-darwin", "quipit"),
    "macos-intel": ("x86_64-apple-darwin", "quipit"),
}


def version():
    value = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", value):
        raise ValueError("Cargo.toml must contain a release version such as 0.1.0")
    return value


def plan():
    tag = "v" + version()
    if os.environ.get("GITHUB_REF_TYPE") == "tag" and os.environ["GITHUB_REF_NAME"] != tag:
        raise ValueError(f"Tag must match Cargo.toml: {tag}")
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write(f"tag={tag}\n")
    print(f"Preparing draft QuipIt {tag}")


def package(target, binary=None, output=None):
    target_triple, executable = TARGETS[target]
    binary = binary or ROOT / "target" / target_triple / "release" / executable
    output = output or ROOT / "dist"
    output.mkdir(parents=True, exist_ok=True)
    name = f"QuipIt-{version()}-{target}"
    stage = output / name
    stage.mkdir()  # Refuse to overwrite an earlier staging directory.
    shutil.copy2(ROOT / "README.md", stage)
    shutil.copy2(ROOT / "CHANGELOG.md", stage)
    shutil.copy2(ROOT / "RELEASE.md", stage)
    shutil.copytree(ROOT / "licenses", stage / "licenses")
    if target.startswith("macos"):
        contents = stage / "QuipIt.app" / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        shutil.copytree(ROOT / "licenses", contents / "Resources" / "licenses")
        shutil.copy2(ROOT / "assets" / "quipit.icns", contents / "Resources" / "quipit.icns")
        shutil.copy2(binary, contents / "MacOS" / "quipit")
        (contents / "MacOS" / "quipit").chmod(0o755)
        with (contents / "Info.plist").open("wb") as stream:
            plistlib.dump({
                "CFBundleName": "QuipIt", "CFBundleDisplayName": "QuipIt",
                "CFBundleIdentifier": "com.trilogic.quipit", "CFBundleExecutable": "quipit",
                "CFBundlePackageType": "APPL", "CFBundleShortVersionString": version(),
                "CFBundleVersion": version(), "NSHighResolutionCapable": True,
                "CFBundleIconFile": "quipit.icns",
            }, stream)
        # Local signing makes the bundle internally consistent, but is not Developer ID signing.
        subprocess.run(["codesign", "--force", "--deep", "--sign", "-", str(stage / "QuipIt.app")], check=True)
        subprocess.run(["codesign", "--verify", "--deep", "--strict", str(stage / "QuipIt.app")], check=True)
        with zipfile.ZipFile(output / f"{name}.zip", "w", zipfile.ZIP_DEFLATED) as archive:
            for path in sorted(stage.rglob("*")):
                archive.write(path, path.relative_to(output))
    elif target == "windows-x64":
        shutil.copy2(ROOT / "assets" / "quipit.ico", stage / "quipit.ico")
        shutil.copy2(binary, stage / executable)
        with zipfile.ZipFile(output / f"{name}.zip", "w", zipfile.ZIP_DEFLATED) as archive:
            for path in sorted(stage.rglob("*")):
                archive.write(path, path.relative_to(output))
    else:
        shutil.copy2(ROOT / "assets" / "quipit-256.png", stage / "quipit.png")
        shutil.copy2(binary, stage / executable)
        (stage / executable).chmod(0o755)
        dependencies = subprocess.run(["ldd", str(stage / executable)], check=True, capture_output=True, text=True).stdout
        (stage / "runtime-libraries.txt").write_text(dependencies)
        if "not found" in dependencies:
            raise RuntimeError("Missing runtime libraries in Linux build")
        with tarfile.open(output / f"{name}.tar.gz", "w:gz") as archive:
            archive.add(stage, arcname=name)
    shutil.rmtree(stage)


def publish():
    tag = "v" + version()
    output = ROOT / "dist"
    assets = sorted([*output.glob("*.zip"), *output.glob("*.tar.gz")])
    expected = {f"QuipIt-{version()}-{target}" + (".tar.gz" if target == "linux-x64" else ".zip") for target in TARGETS}
    if {p.name for p in assets} != expected:
        raise RuntimeError("All four platform packages must be present before creating the draft")
    checksums = output / "SHA256SUMS.txt"
    lines = []
    for asset in assets:
        with asset.open("rb") as stream:
            lines.append(f"{hashlib.file_digest(stream, 'sha256').hexdigest()}  {asset.name}\n")
    checksums.write_text("".join(lines))
    notes = output / "release-notes.md"
    notes.write_text(
        f"# QuipIt {tag}\n\nDraft for testing before publication.\n\n"
        "## Downloads\n\n"
        "- Windows x64: extract the ZIP and run quipit.exe. Requires the Microsoft Visual C++ v14 x64 runtime. "
        "If VCRUNTIME140.dll is missing, install it from https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist.\n"
        "- Mac: choose Apple Silicon or Intel, extract the ZIP, and move QuipIt.app to Applications.\n"
        "- Linux x64: extract the tar.gz and run ./quipit in a graphical desktop session. "
        "Built on Ubuntu 22.04; runtime library names are included in runtime-libraries.txt.\n\n"
        "These initial packages have no Windows publisher signature or Apple Developer ID/notarization. "
        "Operating systems may block or warn about them. Mac packages are locally ad-hoc signed only.\n\n"
        "Save/Ctrl+S updates the current project or image; use Save As to choose another format. "
        "File associations and native installers are not included yet.\n\n"
        + (ROOT / "CHANGELOG.md").read_text()
    )
    existing = subprocess.run(["gh", "release", "view", tag, "--json", "isDraft,targetCommitish,url"], capture_output=True, text=True)
    sha = os.environ["GITHUB_SHA"]
    if existing.returncode == 0:
        release = json.loads(existing.stdout)
        if not release["isDraft"] or release["targetCommitish"] != sha:
            raise RuntimeError("Refusing to replace a published release or a draft for another commit")
        subprocess.run(["gh", "release", "upload", tag, *map(str, assets), str(checksums), "--clobber"], check=True)
    else:
        subprocess.run(["gh", "release", "create", tag, *map(str, assets), str(checksums),
                        "--draft", "--target", sha, "--title", f"QuipIt {tag}", "--notes-file", str(notes)], check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["plan", "package", "publish"])
    parser.add_argument("--platform", choices=TARGETS)
    args = parser.parse_args()
    if args.command == "package":
        if not args.platform:
            parser.error("package requires --platform")
        package(args.platform)
    elif args.command == "plan":
        plan()
    else:
        publish()
