# Change notes

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
