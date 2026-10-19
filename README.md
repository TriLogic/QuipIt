<p align="center">
  <img src="assets/quipit-256.png" alt="QuipIt blue speech balloon icon" width="112" height="112">
</p>

<h1 align="center">QuipIt</h1>
<p align="center"><strong>Version 0.3.0</strong><br>Images, captions, and a little conversation.</p>

QuipIt is a desktop image editor for **Windows, Linux, and macOS**. Add captions,
speech balloons, shapes, and images to a single canvas, then arrange and style
those elements as editable objects. Use the mouse and keyboard, or ask the built-in
AI assistant to make changes in plain language.

Save an editable QuipIt project to continue later, or save a flattened image for
sharing. The interface brings together a symbolic toolbar, an Objects list,
contextual Properties, and an optional AI chat panel.

## Capabilities

### Files and projects

- Create a canvas with precise dimensions, optional locked proportions, and a
  white, colored, or transparent background. New canvases support up to
  **5000 × 5000 pixels**.
- Open QuipIt `.qip` projects, legacy `.quipit` projects, and raster images including
  PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, AVIF, PNM, TGA, DDS, EXR, HDR, and QOI.
- Drag a file onto the canvas. QuipIt files open as projects; images offer
  **Open** or **Import** when a document is already loaded. Drop one file at a time.
- **Save** writes back to the current file and format. New documents default to
  `untitled.qip`; opening a raster image makes that image the default save target.
- **Save As** provides filename, folder, and format controls in one dialog. Changing
  the format updates the extension while retaining the filename. Later saves use
  the new destination.
- Save As supports **QuipIt, PNG, JPEG, WebP, BMP, GIF, TIFF, ICO, and AVIF**.
  QOI files can be opened, but cannot be saved; choose another format instead.
- Projects preserve the background, object order, names, visibility, transforms,
  text and styling, embedded fonts, and original imported image pixels.
- Image saves flatten the visible composition on disk while keeping objects
  editable in the current session. Reopening a flattened image restores its pixels,
  rather than its editable objects.
- Save, discard, or cancel before replacing a document or closing with pending
  changes. Failed saves display the destination and error, and retain unsaved edits.
- The window title shows the current filename, abbreviating long names. The
  rightmost toolbar button opens an About dialog with the app name and version.

### Text and captions

- Add a **text rectangle**, **speech balloon**, or **thought bubble**.
- Type directly on the canvas with word wrapping and resize-to-reflow text.
- Use bundled Sans and Monospace fonts, or load a local **TTF/OTF** font.
- Adjust font size, bold, italic, alignment, text color, container fill,
  border color, and border thickness.
- Apply independent **text shadows** and **container shadows**, each with color,
  transparency, horizontal/vertical offsets, and blur.
- Move, resize, rotate, and reflect text with its container. New text objects
  reuse the most recently used font settings during the session.
- Select text, navigate with the keyboard, copy, cut, paste, and undo typing.
  Overflow is clipped to the text area and flagged so you can adjust the box or font.
- Accept text and select it again later to edit it. Cancel restores an existing
  object's previous state or removes a newly created object.

### Shapes, lines, and curves

The shape picker offers **22 choices**:

| Group | Shapes |
| --- | --- |
| Lines and curves | Straight line; three-, four-, and five-point curves |
| Basic shapes | Circle, capsule, square, rounded square, triangle, rounded triangle, pentagon, hexagon, diamond |
| Symbols | Arrow, five-point star, six-point star, X mark, check mark, lightning bolt, heart |
| Text containers | Speech balloon and thought bubble |

- Adjust fill, border color and width, and transparency. Open lines and curves
  use a stroke without a fill.
- Add shadows with color, transparency, offsets, and blur.
- Move, stretch, rotate freely or in quarter turns, and reflect horizontally
  or vertically. Selection bounds follow the transformed shape.
- Edit four- and five-point curves by dragging their endpoints and control
  handles. Endpoints can move independently beyond the original selection.
- Switch between curve editing and whole-object movement/resizing.
- Keep accepted shapes editable, with styling, shadows, and transforms preserved
  in QuipIt projects.

### Imported images

- Insert images as separate editable objects and retain their original pixels
  while resizing and editing.
- Move, resize, rotate, and flip image objects using handles and Properties.
- Set image transparency and an inset border with its own color, thickness,
  and transparency.
- Add a shadow that follows the image silhouette, with adjustable color,
  transparency, offsets, and blur.
- Position objects partly outside the canvas; flattened output clips to the
  canvas boundaries.

### Objects and layout

- Manage text, shapes, and images in one ordered object stack.
- Select objects from the canvas or Objects list; rename, duplicate, delete,
  and show or hide them.
- Move objects forward or backward, send them to the front or back, or drag
  items in the list to reorder them. Right-click menus provide the same actions.
- Select the fixed **Canvas** entry to change the background color and
  transparency while preserving the objects above it.
- Use a resizable Objects/Properties sidebar with a draggable divider and
  independent scrolling.
- Apply edits without merging objects into the background. Cancel restores
  the previous version of an existing object.

### Canvas, color, and history

- Fit the document to the available canvas area; transparent pixels display
  against a checkerboard.
- Crop with a movable selection and eight resize handles. A live readout shows
  the selection's position and dimensions in image pixels.
- Resize the canvas and its objects with precise dimensions, optional locked
  proportions, and high-quality resampling.
- Choose from 20 basic colors, two recent accepted colors, an eyedropper, or a
  custom RGB picker with exact channel values.
- Preview colors and transparency live; canceling restores the previous values.
- Use sliders, editable numbers, and increment/decrement buttons for numeric
  properties such as sizes, rotation, transparency, and shadows.
- Undo and redo the most recent **10 document edits**, including object changes,
  ordering, duplication, deletion, background edits, cropping, and resizing.
  Text editing also has its own undo history.

## AI integrations

QuipIt's optional AI assistant connects to **OpenAI**, **Google Gemini**, and
**xAI Grok** using your provider API key and chosen text-capable model. You can
switch providers from the AI panel without leaving the editor.

### What the assistant can do

- Describe the document's dimensions, background, objects, and history state.
- Count objects, optionally by type or visibility.
- Create text and shapes; edit captions, fonts, colors, borders, transparency,
  and shadows.
- Move, resize, rotate, and reflect objects; adjust curve control points.
- Select, rename, duplicate, hide, delete, and reorder objects.
- Change the background, crop or resize the canvas, and create a new document.
- Undo or redo edits, and clear undo/redo history when requested.

For example, try: “Add a speech balloon saying Hello,” “Make the selected shape
blue with a white border,” or “How many visible text objects are there?”

Editing plans use the same validated commands as the editor. Multi-step edits
apply together as one undoable action; a failed plan leaves the document
unchanged. Replies based on an older document state are rejected if you change
the document while a request is running. Clearing history cannot be undone.

### Setup and chat

1. Click **AI** on the toolbar, then open the provider setup control.
2. Choose a provider and configure its API key.
3. **Load models** and choose an available model, or enter its model ID manually.
   **Refresh models** updates the list.
4. Use **Test connection**, save the configuration, and select **Use provider**.
5. Type into the chat composer. **Enter** sends; **Shift+Enter** adds a newline.

The chat panel scrolls and can be resized, detached, docked, or hidden using the mouse:

- Drag the AI header out of the docked panel and release to detach it.
- Drag the detached window by either its native title bar or its AI header.
  Release either header over the dock strip at the bottom of the editor to dock it again. The strip remains visible while detached;
  you can also click the strip or the window's **Dock** button.
- Click QuipIt's **×** to hide the chat. Native close controls are disabled where supported. The toolbar's **AI** button reopens it in its last
  docked or detached mode.

Draft text, conversation, provider settings, and running requests survive these
changes. Detached size and position are remembered during the session. Native
windows can move onto another monitor; on backends without global window
coordinates, including Wayland, the floating window stays inside the editor so
mouse docking remains available.

**Send** and **Stop** are stacked at the right of the input, with Send above Stop.
Stop is disabled when no request is running. **Stop** discards a pending reply, although the provider request may finish in the background. Test connection
checks access to the selected model; it does not generate content or verify
content-generation quota.

### Data and credentials

- The selected provider receives conversation context and document properties,
  including object names and text. **Image pixels and local files are not sent**;
  the assistant cannot inspect the visual contents of an imported image.
- Chat history lasts for the current session; the latest 24 messages provide
  conversation context.
- Provider and model preferences persist locally. API keys are never stored
  in projects or the preferences file.
- Optional **Remember key** uses Windows Credential Manager, macOS Keychain,
  or Linux Secret Service. Otherwise, keys stay in memory for the session.
  Linux requires an unlocked credential store for remembered keys.
- Opening, saving, importing images, and loading local fonts remain toolbar or
  dialog actions; the AI assistant cannot perform those file operations.

## Useful controls

| Action | Control |
| --- | --- |
| Save | Ctrl+S; Command+S on macOS |
| Undo | Ctrl+Z; Command+Z on macOS |
| Accept an object or crop | Enter, Apply, or the relevant toolbar action |
| Cancel an edit or crop | Escape or Cancel |
| Preserve proportions when resizing an object or crop | Hold Shift while dragging |
| Toggle a selection handle between resize and rotate | Right-click the handle |
| Move a text object | Drag its outer edge, or Alt-drag inside it |
| Insert a forced line break on the canvas | Ctrl+Enter |
| Select all text while typing | Ctrl+A |

## Get QuipIt

See [GitHub Releases](https://github.com/TriLogic/QuipIt/releases) for published
packages and [the release guide](RELEASE.md) for build, packaging, and installation
instructions. Release packaging targets Windows x64, Linux x64, Apple Silicon Mac,
and Intel Mac.

To run from source, install Rust and use the repository's pinned toolchain:

```sh
cargo run --locked
```

Build an optimized executable with:

```sh
cargo build --release --locked
```

On Ubuntu/Debian, install the desktop build dependencies first:

```sh
sudo apt install build-essential pkg-config libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxinerama-dev libgl1-mesa-dev
```

A graphical desktop session is required to launch the application.

## Library and automation

QuipIt also exposes its editing engine as a Rust library with serializable
commands, persistent object IDs, document inspection, and composited previews.
Validated batches roll back on failure and undo as one action. The library API
also provides file operations for scripts, separately from the AI assistant.

See [the command guide](docs/commands.md) and
[the command workflow example](examples/command_workflow.rs).

## Current limits

- One document canvas at a time; no tabs or animation editing. Animated inputs
  are treated as a single image.
- QuipIt projects are limited to 256 MB on disk. Large images and undo history
  are held in memory.
- JPEG transparency is composited onto white. Saved images do not preserve source
  metadata.
- Zoom/pan, system font discovery, and native installers are future work.

## Development checks

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 -m unittest discover -s scripts -p 'test_*.py'
```

Written in Rust with eframe/egui. Licensed under MIT; bundled font licenses are
included in [licenses/fonts](licenses/fonts).
