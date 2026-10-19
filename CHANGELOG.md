# Change notes

## 0.3.0 — 2026-10-19

- Support mouse-driven AI chat detaching, window movement, redocking, and closing,
  preserving the draft, conversation, and running request. Remember detached
  window placement during the session. Let the window manager handle native
  dragging and avoid geometry feedback during movement and edge resizing.
- Stack Send above Stop at the right of the AI composer. Support docking from
  both the native title bar and the internal AI header by tracking window movement
  and mouse release. Add a visible dock strip and a Dock button as direct
  mouse-operated reattachment controls. Disable native close controls where supported.
- Move the Thinking status into the internal AI title bar instead of below the
  command entry. Use a muted, fully visible input border with scrolling for
  longer drafts.

## 0.2.0 — 2026-10-19

- Add AI-assisted editing through OpenAI, Google Gemini, and xAI Grok, with
  provider configuration, model selection, connection testing, and optional
  credential storage in the operating system's key store.
- Add a resizable AI chat panel and validated, atomic editing plans that can be
  undone. Support document descriptions, object counts, and history management.
- Expose the shared editing engine through serializable Rust commands with
  persistent object IDs, validation, and transactional batches.
- Save images back to their current path and format. Replace the Export menu
  with a Save As dialog offering filename, folder, and a format dropdown.
  Exclude QOI output and display save failures without discarding edits.
- Add canvas file drops with Open/Import choices and the existing unsaved-change
  preservation flow.
- Show the current filename in the window title, abbreviating long names.
- Add a blue speech-balloon application icon, platform icon packaging, and an
  About dialog with a matching monochrome toolbar button.
- Fix the heart picker icon's downward stroke spike.
- Rewrite the README to describe the current capabilities and AI integrations.

## 2026-10-15

- Add a manually triggered or version-tagged release workflow with four native
  builds, portable Windows/Linux archives, ad-hoc signed macOS app bundles,
  checksums, and a draft release for review. Include a browser-only release guide.
  Pin Rust 1.92.0 for consistent local, CI and release builds.

- Keep text, shapes, and imported images as editable objects instead of merging
  them into the canvas. Support naming, visibility, duplication, deletion,
  front-to-back ordering, and undo/redo.
- Add a resizable right sidebar with Objects above Properties, eye visibility
  toggles, drag ordering, and right-click object actions. Highlight the selected
  object and display its name as a single-line properties heading.
- Add editable `.qip` project saving and opening, with support for legacy
  `.quipit` filenames. Embed original images, fonts, styling, and object order.
  Save/Ctrl+S saves a project; Export image writes a flattened raster image without
  marking the project saved. Write projects atomically.
- Add a fixed Canvas item with fill color and transparency properties. Preview,
  cancel, undo, and redo preserve added objects and restore the previous base image.
- Preserve object editing through crop, resize, export, and project round trips.
- Make text objects easier to move using a wider draggable edge or Alt-drag,
  while ordinary interior drags select text. Improve caret hit testing on lines
  with different horizontal insets and retain keyboard focus for arrow navigation.
- Reuse the latest font, size, bold/italic, text color, and alignment for new text
  objects during the session. Default text boxes to a 3px black border on white.
- Add image and text-container border shadows with color/transparency, offsets,
  and blur. Keep text shadows independent. Shadows default to opaque black;
  zero blur now bypasses the image library's minimum blur.
- Fix confirmed New/Open/Exit actions leaving an object in edit mode or reopening
  the unsaved-changes prompt. Cancel continues to preserve the active edit.
- Expand regression coverage for editing, ordering, rendering, project persistence,
  shadow edges, canvas colors, and document transitions.
