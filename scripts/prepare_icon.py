"""Convert the approved PNG artwork to platform icon files (requires Pillow)."""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent


def main():
    assets = ROOT / "assets"
    with Image.open(assets / "quipit.png") as source:
        image = source.convert("RGBA")
        image.resize((256, 256), Image.Resampling.LANCZOS).save(assets / "quipit-256.png")
        image.save(assets / "quipit.ico", sizes=[(n, n) for n in [16, 24, 32, 48, 64, 128, 256]])
        image.resize((1024, 1024), Image.Resampling.LANCZOS).save(assets / "quipit.icns")


if __name__ == "__main__":
    main()
