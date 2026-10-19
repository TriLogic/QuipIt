# Shared editing commands

QuipIt exposes its editing engine as a Rust library. The desktop UI and future
AI, scripting, or batch adapters use the same document operations, property
validation, renderers, and history. No AI service or network server is started.

## Entry points

- `QuipIt::execute(&egui::Context, Command)` runs a command and returns a
  `Result<DocumentInfo, CommandError>`.
- `QuipIt::inspect()` returns the document dimensions, ordered object properties,
  selected ID, background selection, pending edits, and undo/redo availability.
- `QuipIt::render_preview()` returns the composited RGBA image, including the
  current edit. Export uses the same composition.

`egui::Context::default()` is sufficient for commands without opening a window.
It maintains the texture handles used by the desktop renderer. Run commands on
the editor's owning thread; an eventual provider adapter should send commands
to that thread and receive the results.

The executable in `src/main.rs` launches the library's desktop application.
`src/commands.rs` owns the serializable command contract and dispatcher;
`src/commands/operations.rs` owns the shared editing operations. UI widgets
edit local property drafts or produce geometry/text gestures, then submit them
through the command module's validated edit pipeline. Native dialogs choose
paths; commands perform the actual file operations without opening dialogs.

## Command coverage

| Area | Commands |
| --- | --- |
| Inspection | `inspect`, `describe_document`, `count_objects` |
| Documents | `new_document`, `open_document`, `save_project`, `export_image` |
| Creation | `add_shape`, `add_text`, `insert_image` |
| Selection | `select_object`, `select_background`, `apply`, `cancel` |
| Properties | `update_object`, `edit_text`, `rename`, `set_visibility`, `set_background` |
| Geometry | `transform`, `move_curve_point`, `set_curve_editing`, `reorder` |
| Objects | `duplicate`, `delete` |
| Canvas | `crop`, `resize` |
| History | `undo`, `redo`, `purge_history`, `batch` |

`update_object` accepts a partial `ObjectPatch`; omitted fields remain unchanged:

- `bounds`: position and size in canvas pixels; movement/resizing use the same
  bounds as the visible selector. JSON uses
  `{"min":{"x":20,"y":30},"max":{"x":120,"y":110}}` for a 100×80 object.
- `shape`: fill, border color/width, and overall transparency.
- `text`: content, font, size, bold/italic, text/border/background colors,
  border width, alignment, and glyph shadow. Fonts accept `sans`, `monospace`,
  or `file` with a TTF/OTF path. Text stays editable and fonts remain embedded
  in saved projects. Full content replacement also supports programmatic
  insertion, deletion, and replacement of selected text.
- `image`: border color/width/transparency and image transparency.
- `shadow`: object/container shadow enabled state, color, offset, and blur.
  On text objects this is independent of `text.shadow`, the glyph shadow.

`edit_text` provides `select`, `insert`, `backspace`, `delete`, `undo`, and `redo`
actions using the same text operations as the keyboard editor. Selection uses
UTF-8 byte boundaries. Insert replaces the selected range and normalizes line
endings. Backspace/Delete accept a `word` flag. These Undo/Redo actions affect
the active text session; document-level Undo/Redo remain separate commands.
Inspection returns the selected range and text, allowing a clipboard adapter
to copy it or cut it by inserting an empty string.

`transform` takes rotation in degrees plus horizontal/vertical reflection flags.
`move_curve_point` takes a control index and destination in canvas coordinates
for four- and five-point curves. `crop` takes `[x, y, width, height]`.
`resize` scales the canvas and its objects; callers calculate any desired
aspect-ratio constraint before submitting the final dimensions.

Colors in command patches use `[red, green, blue, alpha]`, each from 0 to 255.
Transparency is 0–100 percent. Shadow offsets can be negative. Values must be
finite and within their supported ranges; invalid edits return an error before
switching selection or changing the document. Unknown JSON fields are rejected.
`CommandError` exposes a machine-readable `code` and a human-readable `message`.

## Object identity and depth

Object IDs are opaque numbers returned by inspection. Use
`{"kind":"object","id":...}` to target an existing object, or
`{"kind":"selected"}` for the current edit. IDs survive changes in depth,
editing, undo/redo, and project save/reopen. Duplicates receive a fresh ID and the duplicate command selects the new copy,
so a following rename or property update can target `selected`.
Old projects without IDs acquire unique IDs when opened; duplicate saved IDs
are rejected. An ID from another document need not refer to the same object.

Inspection returns objects from back to front, including the active edit at
its original depth. Reorder indices use that same order: zero is the back and
`objects.len() - 1` is the front. Selection by ID avoids dependence on list
positions. Selecting a hidden object makes it visible, matching desktop behavior.
Font bytes and image pixels are omitted from inspection; request a rendered
preview when visual context is needed.

## Preview, Apply, Cancel, and transactions

Creating or selecting an object starts an edit. Property updates, transforms,
curve edits, and renaming preview changes in that session. `apply` accepts the
edit and creates the normal document undo checkpoint. `cancel` restores an
existing object's original version or removes a new object. Selecting another
object or performing a document-wide operation finishes the current edit, as
in the desktop app.

Save and export include pending edits without requiring Apply. Save writes the
editable project; export writes a flattened image and keeps objects editable.
New/Open replace the document and clear its history. Their callers decide when
to request user confirmation; the desktop retains its unsaved-changes dialogs.
All file commands use explicit paths supplied by the caller.

`batch` runs 1–1024 commands, accepts the final edit, and creates one undo step
for the entire sequence. Any failing command restores the pre-batch document,
pending edit, history, and rendering. Batches reject file writes, New/Open,
Undo/Redo, and nested batches before executing anything. File reads such as
image/font insertion are allowed. This lets one AI request produce one
reviewable, undoable edit.

## JSON example

Deserialize each object as `quipit::commands::Command`, then call `execute`.
Shape and text kind names use their Rust enum spelling, such as `Star5`,
`Circle`, `Area`, `Speech`, and `Thought`. Command/target/font names use snake case.

```json
{
  "command": "batch",
  "commands": [
    { "command": "add_shape", "kind": "Star5" },
    {
      "command": "update_object",
      "target": { "kind": "selected" },
      "patch": {
        "shape": { "fill": [255, 200, 0, 255] },
        "shadow": {
          "enabled": true,
          "color": [0, 0, 0, 128],
          "offset": { "x": 6, "y": 6 },
          "blur": 4
        }
      }
    },
    {
      "command": "rename",
      "target": { "kind": "selected" },
      "name": "Featured star"
    }
  ]
}
```

Create or open a document first. The runnable example creates a canvas, executes
this kind of batch, and exports a PNG:

```sh
cargo run --example command_workflow -- /tmp/quipit-command-preview.png
```

## Verification

`cargo test --locked` includes the desktop regressions and public API integration
tests. Coverage includes every shape/text kind, style validation, Unicode text,
fonts and images, curves, crop/resize, stable IDs, legacy projects, save/export
parity, cancellation, atomic failure, and grouped undo/redo.

## Document descriptions, counts, and purging history

`describe_document` returns the same read-only information as `inspect`, including
`dimensions`, `background`, `object_counts`, `undo_steps`, and `redo_steps`.
A uniform background is `{"kind":"uniform","color":[r,g,b,a]}`; a background
with differing pixel colors is `{"kind":"mixed"}`. With no document, dimensions
and background are null and object counts are zero. Background excludes overlays.
Counts include pending edits exactly once and exclude the canvas background.
`object_counts` contains total, visible, hidden, shapes, text, and images.
Text balloons count as text, while symbolic shapes count as shapes.

`{"command":"count_objects"}` returns `query_count` for all objects. Optional
`kind` (`shape`, `text`, `image`) and `visible` (true/false) filters combine:
`{"command":"count_objects","kind":"shape","visible":true}` counts visible
shapes. These queries do not change selection, document, or history.

`{"command":"purge_history"}` clears document undo/redo stacks and text editing
undo/redo histories. It preserves pixels, objects, and the pending edit. Purging
cannot be undone. Future edits create new undo steps normally. Aliases `purge`,
`clear_history`, and `clear_undo_history` are accepted. Purge is rejected inside
batches. In AI chat, use purge, description, and count commands on their own;
query results are printed in the conversation without creating undo steps.
