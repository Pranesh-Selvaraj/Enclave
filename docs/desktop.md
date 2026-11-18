# Enclave desktop guide

The desktop app is a **development preview**, not a release build yet. This
guide describes the behaviour that exists in `apps/desktop` on `main`; the
remaining parity work is tracked in
[`plans/desktop-feature-parity.md`](plans/desktop-feature-parity.md).

## Launching

```bash
cargo run --manifest-path apps/desktop/Cargo.toml
```

The app uses the same app-data directory as the retired Tauri build
(`com.enclave.app`), so it can open an existing vault. Do not run both apps
against one vault at the same time. For development, point it elsewhere:

```bash
ENCLAVE_APP_DIR=/tmp/enclave-desktop-dev cargo run --manifest-path apps/desktop/Cargo.toml
```

On WSL2/WSLg the default GPU adapter may fail; force the OpenGL/software path
with `WGPU_BACKEND=gl LIBGL_ALWAYS_SOFTWARE=1`, or try `WGPU_BACKEND=vulkan`.
See [`apps/desktop/README.md`](../apps/desktop/README.md) for build details.

## Vault gate

The gate opens with the Enclave lockup, then reveals the form.

- **First run:** choose a password, confirm it and select **Create vault**. The
  core creates the encrypted vault and shows the 12-word recovery phrase once.
  Write it down before continuing.
- **Returning:** enter the password and select **Unlock**, or press Enter.
- The password has no length policy in the core. A longer password is harder to
  guess; the app does not impose a PIN-style rule.
- The desktop gate is currently password-only and dark-theme only. Android's
  PIN mode, restore, forgotten-password and auto-lock flows are not wired into
  the desktop UI yet.
- **Lock** in the top bar saves the open note and returns to the gate.

The vault is local and encrypted at rest. There is no account, cloud service or
telemetry.

## Notes

- **+ New note** creates a page and opens it.
- The title is an inline field above the document; edits are saved through the
  core.
- The left panel lists notes. Selecting another note flushes pending title and
  document edits first.
- The empty state also allows creating the first note.

## Document editing

Text blocks are native `TextEdit` widgets backed by the headless ProseMirror
engine. The canonical document shape is
[`core/core-db/tests/fixtures/block-schema-v1.json`](../core/core-db/tests/fixtures/block-schema-v1.json).

- **Paragraphs, headings, quotes, code blocks and rules** render and edit.
- **Marks:** select text, then use **B**, *I*, **S** or `</>` in the bottom
  toolbar.
- **Task lists:** click the checkbox to toggle a task through the core.
- **Lists:** put the caret in a block and use **• List**, **1. List** or
  **☑ Task** in the toolbar to create a list or change its type.
- **Tables:** click a cell and edit it. **Tab** moves to the next cell and
  **Shift+Tab** to the previous cell, wrapping at the table edges. Cell
  navigation does not mark the note dirty or create undo history. The
  **Table** toolbar menu inserts a 2 × 2 table, adds rows/columns below or to
  the right, and deletes the current row/column while keeping at least one of
  each.
- **Images** currently render as placeholders; attachment storage and upload
  are not implemented.
- **Saving:** edits autosave after roughly 700 ms of inactivity. **Ctrl/Cmd+S**
  saves immediately, and locking or switching notes flushes pending edits.
- **Undo/redo** are the arrow buttons in the toolbar. Keyboard shortcuts are
  still pending.

## Keyboard shortcuts

| Key | Behaviour |
| --- | --- |
| Enter | Split the current block; in a list or task item, create a new sibling item |
| Backspace at the start | Merge a non-list block with the previous one; on an empty list item, remove the item |
| Tab | Move to the next table cell; otherwise nest a list item |
| Shift+Tab | Move to the previous table cell; otherwise outdent a list item |
| Ctrl/Cmd+S | Save the open note immediately |
| Enter on the gate | Submit the password form when it is ready |

Tab is deliberately kept inside the document editor. In a code block it inserts
a literal tab; outside a list or table it does nothing rather than moving focus.

## Known gaps

The desktop is still missing the native editing-bar gate (real IME coverage,
AccessKit/screen-reader verification and a 2,000-block latency pass), table
merged cells, configurable column widths, image/attachment storage, search, folders, tags, trash, restore,
auto-lock, backup import/export, settings, sync UI, packaging and updates.

Do not treat this preview as a replacement for the historical Tauri desktop for
daily data. The vault and block formats are shared, but the surrounding
workflows are still being transferred.

## Related documents

- [Desktop feature parity](plans/desktop-feature-parity.md)
- [Architecture decision 0013](decisions/0013-fully-rust-desktop.md)
- [Native-only repository decision 0014](decisions/0014-native-only-repository.md)
- [Brand assets](brand/README.md)
- [Desktop build and test notes](../apps/desktop/README.md)
