# QuipIt

A Rust desktop image editor for Windows, Linux, and macOS. One canvas, no tabs,
with a straightforward workflow inspired by Paint 3D.

## Downloads and releases

The **Build draft release** GitHub Action builds Windows, Linux, Apple Silicon Mac,
and Intel Mac downloads, then creates a draft release for review. See
[the release guide](RELEASE.md) for browser-only instructions and packaging limits.

## Run

Install Rust (rustup will use the pinned toolchain), then run:

```sh
cargo run
```

Build an optimized executable with `cargo build --release`.
Linux requires desktop development libraries (on Ubuntu/Debian:
`sudo apt install build-essential pkg-config libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxinerama-dev libgl1-mesa-dev`).
A graphical desktop session is required to launch the app. Docker is not required.

## Shared command API

The status text sits flush along the bottom with no inset or separator line.
The AI toolbar button reveals a bounded command area above it, anchored across
the canvas width beside the Objects/Properties sidebar, above the full-width
status bar, with a slightly darker gray interior. Drag its upper edge to adjust
the terminal height. Hiding AI returns to the compact status footer.
The AI header has a provider setup/switch button beside its close button.
Choose OpenAI, Gemini, or Grok. Each option has a Configure button for a masked
API key, model picker, optional Remember key, and Test connection. Enter your
key and click Load models to fetch the provider’s available model IDs, then
choose one from the dropdown. Refresh models reloads the list; changing the
key clears it. You can also enter a model ID manually. Save stores the
provider's configuration; Use provider saves the active selection. Cancel discards
unsaved changes. Provider/model preferences persist in `ai.json` in the OS
application configuration directory (Linux: `~/.config/QuipIt`). API keys are never
written there or into projects. Remembered keys use Windows Credential Manager,
macOS Keychain, or Linux Secret Service; otherwise keys live only for the session.
Turn off Remember key and save to remove a previously stored key. On Linux,
remembering keys requires an unlocked Secret Service such as GNOME Keyring or
KWallet. Credential-store failures are shown in the configuration dialog.
Test connection checks key/model access in the background with a 20-second
timeout; it does not generate content or test generation quota.

The AI panel is a scrolling chat with an `ai>` composer. Enter sends; Shift+Enter
adds a newline. Replies can execute validated editing commands against the current
canvas. Each edit plan is atomic and undoable; failed plans leave the document
unchanged. Stop discards the pending reply (the provider request may finish in the
background). Changing the document while AI is thinking rejects that reply's edits.
Chat history and document properties/text are sent to the selected provider; image
pixels and local files are not sent. Chat history is held only for this session;
the latest 24 messages provide conversation context. File operations use the
normal toolbar. OpenAI and Grok use Responses; Gemini uses generateContent.
Choose a model that supports text generation. Provider errors appear in chat.
Objects and Properties use smaller titles on lighter gray header bars.
The canvas, Objects, Properties, and AI terminal use rounded panels with matching
borders and spacing.

The desktop editor is also a Rust library with serializable editing commands for
document operations, all object types and properties, transforms, layering,
crop/resize, save/export, and undo/redo. AI or scripting adapters can use the same
validated operations as the UI. Objects have persistent IDs; command batches
roll back on failure and undo as one action. See [the command guide](docs/commands.md)
for the API, JSON examples, and a runnable workflow that needs no desktop window.

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
  A resizable right-hand sidebar stacks the Objects list above the properties panel,
  with a draggable divider and independent scrolling. It keeps the canvas layout stable
  when selecting, applying or canceling a shape. Shape properties adjust border thickness, border color,
  fill color and transparency. Fill is disabled for open lines/curves.
  All shapes, lines and curves also support a drop shadow with color/transparency,
  X/Y offsets and blur. Shadows follow the visible fill and border, extend beyond
  the selector, and remain editable when saving and reopening a project.
  Numeric adjustments use one row with a slider, decrease button, editable number
  and increase button, including shadow settings, rotation, sizes, transparency
  and custom RGB channels. Step buttons change values by one within their limits.
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
  Apply, Enter, or clicking away accepts the shape as an editable object.
  Cancel or Escape removes a new shape or restores an existing one. Shape pixels use the same
  renderer for preview and export. Speech/thought balloon choices open text objects.
- Text toolbar button: Text rectangle, Speech balloon, Thought bubble.
  New text objects reuse the most recently used font, size, bold/italic, text color
  and alignment during the session. New text rectangles start with a 3px black
  border and white background.
  Type directly on the canvas; words wrap within padded container boundaries.
  Ctrl+Enter inserts a forced line break. Click elsewhere or press Enter to accept.
  Escape removes a new text object or restores the previous version when reediting.
  Accepted text stays selectable and editable during the session; click it to edit again.
  Drag the outer 12-pixel edge to move a text object, or Alt-drag anywhere inside it.
  Ordinary interior drags select text. Resize handles reflow text. Shift preserves proportions; right-click a handle toggles
  rotation. Rotation and reflection affect the text and container together.
  Properties include bundled Sans/Monospace fonts, loading a TTF/OTF font, font size,
  synthetic bold/italic, alignment, text/border/background colors, border thickness,
  and text-shadow color, transparency, offsets and blur. A separate Border shadow
  option shadows the container outline of text rectangles, speech balloons and thought
  bubbles, extending outside their bounds. Both shadows have independent controls
  and are preserved in projects and exports. Colors use the shared dialog.
  Overflow clips to the text area and shows a warning so the box/font can be adjusted.
  Ctrl+A, clipboard copy/cut/paste, arrow keys, Shift-selection and local Ctrl+Z work
  while typing. Text objects participate in document undo, crop, resize and image export.
  Text, shapes and imported images share one front-to-back object stack.
  Save a QuipIt project to preserve them, or save as an image to flatten them.
- Objects panel (upper right, above Properties): select an object from the list or click it on the canvas to edit it.
  The list runs from front to back. Rename the selected object, move it forward/backward
  or to the front/back, duplicate it, delete it, or toggle visibility using the eye button.
  Drag list items up or down to reorder them; a line marks the insertion position.
  Right-click an object for Duplicate, Delete, Move up/down, or Move to top/bottom.
  Canvas is a fixed item at the bottom of the list. Select it (or click empty
  canvas space) to fill the base canvas with a color and transparency using Properties.
  This replaces the base raster, preserves all added objects, and supports undo/redo;
  canceling the color dialog restores the original background image.
  Clicking away accepts changes without merging the object into the background.
  Escape or Cancel restores the previous version of an existing object.
  Hidden objects become visible when selected for editing. Order, visibility, deletion,
  duplication and object edits participate in undo/redo.
- Click the QuipIt icon at the right end of the toolbar to view the application
  name, version, and brief description. Dismiss the About dialog with OK.
- File menu: New, Open, Insert, Save, Save As, and Exit.
  Save (Ctrl+S) writes back to the current file, including PNG, JPEG, WebP and other
  supported raster formats. New documents default to an editable `.qip` project.
  Save As opens a dialog with filename, folder, and a format dropdown, retaining
  the current filename with the chosen extension. Subsequent saves use the new file.
  QOI saving is not supported.
  Projects preserve objects, text styling, embedded fonts and imported pixels.
  Image saves flatten the canvas on disk while keeping objects editable in the
  current session. Reopening an image cannot restore its objects.
  Open accepts `.qip`, legacy `.quipit`, and raster images.
  Drop a file onto the canvas to open it. With a document loaded, dropped images
  offer Open or Import; Import adds an editable image object to the current canvas.
  Dropped projects open directly. Replacing a document offers Save, Discard changes,
  or Cancel when edits are pending. Drop one file at a time.
  Version 1 projects are limited to 256 MB on disk.
- New is limited to 5000 × 5000 pixels. It provides width/height sliders with exact pixel entry and optional locked
  proportions, plus a basic background color
  or transparency. White is the initial default. Transparent areas display a checkerboard.
- Insert imports an image as a movable object with eight resize handles. Click
  elsewhere or press Enter to accept it; Escape cancels. Accepted images remain editable.
  Image properties include border thickness/color, independent border transparency
  and image transparency, plus the color palette and eyedropper. The border is
  a frame inside the selector; image content fits within it without overlapping.
  Image objects also support a drop shadow with color/transparency, X/Y offsets,
  and blur. Shadows extend outside the selector, follow the image silhouette, and
  appear in previews, exports and saved projects.
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
- Ctrl+Z undoes the last edit. Ctrl+S saves the current file, asking for a filename when needed.
  Command+Z and Command+S also work on macOS.
- Undo and redo the most recent ten edits. Deleting a selected shape with Delete,
  Cancel or Escape can be undone: its selector, colors, transparency, size and
  transformations are restored for continued editing. Redo removes it again.
- Save As supports projects and images. The format dropdown determines the output
  format and filename extension; QOI is not available for saving.
- Confirm unsaved changes before opening another file or closing.

JPEG transparency is composited onto white. Image saving does not preserve source metadata. Large images and history are held in memory.

## Next milestones

1. Zoom/pan, keyboard shortcuts and image orientation handling.
2. Project thumbnails and additional object organization tools.
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
