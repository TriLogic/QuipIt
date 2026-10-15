# QuipIt

A Rust desktop image editor for Windows, Linux, and macOS. One canvas, no tabs,
with a straightforward workflow inspired by Paint 3D.

## Run

Install a current stable Rust toolchain, then run:

```sh
cargo run
```

Build an optimized executable with `cargo build --release`.
Linux requires desktop development libraries (on Ubuntu/Debian:
`sudo apt install build-essential pkg-config libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxinerama-dev libgl1-mesa-dev`).
A graphical desktop session is required to launch the app. Docker is not required.

## First milestone

- Pix-inspired symbolic toolbar, left to right: New, Open, Insert, Save; Undo,
  Redo; Crop, Resize; Text. Buttons have hover labels and unavailable actions are disabled.
  Apply crop appears beside the crop tools when a selection exists.
- A square toolbar button with hollow circle, square and triangle icons opens
  the 2-D Shapes picker, grouped into Lines and Curves and 2D Shapes.
  Picker buttons are icon-only, match the toolbar button size and show names on hover.
  Choosing any of the 22 options places it on the image with move/resize handles.
  Its initial size is one third of the image’s shorter dimension.
  Four- and five-point curves open in Edit Curve mode: drag endpoint and control
  handles along the visible guide lines to shape their bends. Endpoints move
  independently across the image, beyond the original selector, while other points
  stay fixed. Move / Resize switches
  back to the whole-object selector. Controls are initially fitted into the canvas.
  Curve edits support colors, rotation, reflection, export and undo restoration.
  A fixed-width right-hand properties panel keeps the canvas layout stable
  when selecting, applying or canceling a shape. Shape properties adjust border thickness, border color,
  fill color and transparency. Fill is disabled for open lines/curves.
  A rotation slider, 90-degree turn buttons and horizontal/vertical reflection
  buttons transform the selected shape before applying it. The selector follows
  the transformed bounds. Right-click any shape selector handle to toggle between
  resize (directional resize cursor) and rotate (curved-arrow cursor); left-drag a handle
  to perform the active operation. All selector handles stay bright green with black
  borders in either mode. Escape discards the shape and all its transformations.
  Border, fill/background, image and new-canvas background swatch buttons open
  one shared color/transparency dialog. It includes 20 basic colors, transparency,
  a custom picker, an eyedropper and two recent accepted colors per session.
  The custom picker is modal, with Red/Green/Blue sliders and editable 0–255
  channel values, plus its own OK/Cancel controls. Enter accepts the innermost
  dialog; Escape cancels it. Edits preview live; OK keeps them, while Cancel, Escape or closing the dialog
  restores the previous settings without discarding the selected object.
  Apply or Enter merges the shape as an undoable edit; Cancel or Escape removes it.
  Clicking elsewhere on the canvas also merges it. Shape pixels use the same
  renderer for preview and export. Speech/thought balloon choices open text objects.
- Text toolbar button: Text rectangle, Speech balloon, Thought bubble.
  Type directly on the canvas; words wrap within padded container boundaries.
  Ctrl+Enter inserts a forced line break. Click elsewhere or press Enter to accept.
  Escape removes a new text object or restores the previous version when reediting.
  Accepted text stays selectable and editable during the session; click it to edit again.
  Resize handles reflow text. Shift preserves proportions; right-click a handle toggles
  rotation. Rotation and reflection affect the text and container together.
  Properties include bundled Sans/Monospace fonts, loading a TTF/OTF font, font size,
  synthetic bold/italic, alignment, text/border/background colors, border thickness,
  and drop-shadow color, transparency, offsets and blur. Colors use the shared dialog.
  Overflow clips to the text area and shows a warning so the box/font can be adjusted.
  Ctrl+A, clipboard copy/cut/paste, arrow keys, Shift-selection and local Ctrl+Z work
  while typing. Text objects participate in document undo, crop, resize and image export.
  Text layers are drawn above the raster image. Saving PNG/JPEG/WebP flattens them;
  reopening an exported image does not restore editable text. Project-file saving is pending.
- File menu: New, Open, Insert, Save, Save As, and Exit.
- New is limited to 5000 × 5000 pixels. It provides width/height sliders with exact pixel entry and optional locked
  proportions, plus a basic background color
  or transparency. White is the initial default. Transparent areas display a checkerboard.
- Insert imports an image as a movable object with eight resize handles. Click
  elsewhere or press Enter to merge it; Escape cancels. Merging is undoable.
  Image properties include border thickness/color, independent border transparency
  and image transparency, plus the color palette and eyedropper. The border is
  a frame inside the selector; image content fits within it without overlapping.
  Original imported pixels are retained while editing and resizing. Inserted images
  support a rotation slider, quarter-turn/flip icons and right-click handle toggling
  between resize and rotate. Image transparency opens the shared live-preview dialog.
  Objects may extend beyond the canvas; export clips them to the canvas bounds.
- Open common raster formats supported by the Rust `image` crate. Animated inputs
  are treated as a single image; this is not an animation editor.
- Fit the image to a single canvas.
- Image → Resize opens a dialog with draggable width/height sliders, precise
  numeric entry, optional locked proportions and high-quality resampling.
- Image → Crop places a rectangle on the image. Move it
  or resize it with handles at all four corners and four edge midpoints.
  A live canvas readout shows X/Y offsets and width/height in image pixels.
  Enter (or Apply crop) immediately crops the image to the selector.
  Escape cancels cropping without changing the image.
- Hold Shift while dragging any crop or inserted-image resize handle to preserve
  its aspect ratio at the start of the drag (crop dimensions round to whole pixels).
- Ctrl+Z undoes the last edit. Ctrl+S saves, opening Save As for a new image.
  Command+Z and Command+S also work on macOS.
- Undo and redo the most recent ten edits. Deleting a selected shape with Delete,
  Cancel or Escape can be undone: its selector, colors, transparency, size and
  transformations are restored for continued editing. Redo removes it again.
- Save or Save As, including PNG, JPEG and WebP. The filename extension determines
  the output format; unsupported output formats report an error.
- Confirm unsaved changes before opening another file or closing.

JPEG transparency is composited onto white. Saving exports raster pixels and does
not preserve source metadata. Large images and history are held in memory.

## Next milestones

1. Zoom/pan, keyboard shortcuts and image orientation handling.
2. Editable project files preserving text objects alongside flattened image exports.
3. More text editing refinements, system font discovery and richer typography.
4. Native packaging and hands-on validation on Windows, Linux and macOS.

The initial app uses eframe/egui for a portable native desktop window. Its UI has
one image canvas, a top menu bar with File, Edit and Image dropdowns; no tabbed document interface.

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
