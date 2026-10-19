# QuipIt application icon

A squared blue speech balloon with a thin black border and three white dots on a transparent background.
Generated with the built-in image generation tool, then converted to platform
formats without changing the artwork.

- `quipit.png`: original generated artwork.
- `quipit-256.png`: embedded window icon and Linux package icon.
- `quipit.ico`: Windows executable and shortcut icon, with 16–256 pixel sizes.
- `quipit.icns`: macOS application bundle icon.
- `quipit.rc`: Windows resource definition, compiled by `build.rs` using the Windows SDK.

To regenerate the platform formats after replacing the master artwork, install
Pillow and run `python scripts/prepare_icon.py`. Pillow is only needed when
regenerating assets; application builds use the checked-in files.

## Generation prompt

Use case: logo-brand. Create a polished desktop application icon for QuipIt, an image caption editor. A single bold rounded speech balloon with a short playful tail and exactly three evenly spaced round dots centered inside. Simple clean geometry, friendly expressive silhouette, crisp edges, excellent readability at 16 and 32 pixels. Rich saturated blue speech balloon, white three dots, restrained dimensional finish. Square composition, generous enough safe margins, transparent outside the speech balloon. No text, no letters, no badge, no extra symbols, no mockup, no background square, no cast shadow beyond the icon. Deliver one standalone centered icon.

## Squared-balloon revision prompt

Edit target: the supplied QuipIt application icon. Replace the round bubble silhouette with a SQUARED text/speech balloon: straight horizontal top and bottom, straight vertical sides, almost square rectangular body with only slightly softened corners and a small angular speech tail at lower left. Fill solid vivid blue, add a THIN crisp BLACK outline around the entire balloon and tail. Keep EXACTLY THREE evenly spaced WHITE circular dots in a centered horizontal row inside. Flat clean graphic icon style, no glossy highlights, no 3D bevel, no shading or shadows. Preserve the transparent background and centered square composition with safe margins. No text, no additional symbols. It must clearly read as a squared blue text balloon with thin black border and three white dots, readable at small desktop-icon sizes.
