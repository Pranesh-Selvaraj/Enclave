# 0010. User-created note templates live in the vault's settings table

- **Status:** Accepted
- **Date:** 2026-11-03
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** —

## Context

The create stack offers four starter documents — Text, Checklist, Journal, Meeting notes — and they are compiled into the app as Kotlin builders (`apps/android/app/src/main/java/com/enclave/app/editor/Templates.kt`) that emit the same ProseMirror JSON as `packages/editor/src/templates.ts`. That set is fixed: a user who writes the same kind of page every week either re-types the skeleton or copies an old note and deletes its content.

The request was to let users create their own templates from a page they already like, and to have them appear in the create stack next to the built-ins.

The constraint that shapes the answer is where a template *lives*. Enclave's rule is that anything the user authored belongs in the vault (ADR 0006): a template is a document skeleton, it can contain the user's own headings and prompts, and it must survive a reinstall and a restore (ADR 0007). That rules out app-private storage, but the vault offers three places to put it:

1. A **new table** (`templates`) with its own migration.
2. The existing **`settings`** key/value table.
3. Templates as **ordinary documents** in a special folder.

## Decision

**Templates are stored in the vault's `settings` table, as one JSON array under the key `android.templates.v1`.**

Each entry is `{id, name, doc}`, where `doc` is the note's canonical `doc` block JSON — byte-for-byte what the editor and the core already read and write, so a page created from a template is identical to the page it was saved from. The list is capped at 50 entries and names at 60 characters; an entry that fails to parse is dropped on load instead of breaking the create stack.

The UI is two taps each way: **Save as template** in a note's options sheet (name prefilled with the note's title) adds one; the create stack lists them under a `YOUR TEMPLATES` heading after the built-ins, and a long press deletes one after a confirmation that states the pages already made from it are untouched.

Journal and Meeting notes are **no longer offered**, and Text, Checklist and Folder stay. The reasoning: once a user can author a template, a fixed "meeting" layout is a guess about someone else's meeting — this one keeps three headings and no attendees — and offering both a built-in and a user copy of the same thing makes the user wonder which one is theirs. What remains are the three *creation primitives* (a page, a page of checkboxes, a folder), not templates.

That is a deliberate divergence from the desktop: `packages/editor/src/templates.ts` still offers Journal and Meeting notes in the Tauri app. The documents remain compatible — a Journal page made on the desktop opens on the phone unchanged, because the JSON format is the shared contract — but the native app's create stack no longer reproduces those two skeletons. If the desktop should follow, that is a change to the shared package, not to this decision.

## Consequences

- **No schema change and no migration.** `SCHEMA_VERSION` stays at 1, the vault contract document is untouched, and a build that does not know about templates simply ignores the key. A future sync or desktop client can read it without a fixture update.
- **Templates travel with the vault but do not sync between devices.** `docs/CONTRACT.md` states that sync carries documents, blocks and folders — settings are local. A template made on the phone therefore reaches the desktop through a **backup/restore** (ADR 0007), not through LAN sync. That is a real limitation of this decision, chosen because a synced template store means a third synced surface with its own conflict rules (two devices renaming the same template, deleting while another adds) for a feature that is not worth that complexity yet. If templates should sync, that is a new decision: either a synced `templates` table, or documents in a well-known folder.
- **The whole list is rewritten on every change.** Fine at 50 small JSON documents; it would not be fine for a synced, frequently-edited collection, and that is precisely the point at which the new-table option becomes the right one.
- **Templates are user data, so they are the user's responsibility.** A template can contain anything a note can, including the twelve words of a recovery phrase if the user saves such a page as a template. It is stored encrypted like the rest of the vault, so the exposure is the same as the note it came from.
- **The `doc` JSON is trusted only as far as the parser.** Loading validates that the entry is an object with `type: "doc"`; the editor's `EditorDocument.parse` remains the thing that turns it into blocks, so an older template whose node types have moved on degrades exactly like an older note would.
- **The key is namespaced `android.`** because the setting is written by one app. A desktop template store would either read the same key (a small, explicit compatibility step) or get its own.
- **Creating a template does not require a note.** The create stack has a "New template" entry that opens the same editor with no document behind it, so a user who wants to sketch a skeleton first can; "Save as template" on an existing page covers the other direction. Both write the same entry to the same store.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| A `templates` table with a migration | The honest long-term shape if templates ever sync or grow a UI of their own, but it means `SCHEMA_VERSION` 2, a migration, a fixture update and a contract-document change for a list of at most 50 small JSON values. Not yet worth a vault upgrade. |
| Templates as documents in a hidden folder | Reuses documents, blocks and folders — so it *would* sync — but templates would then appear in searches, widgets, exports and the note count unless every surface learns to exclude a folder, and "delete the template" becomes "delete a document", which is a different and much scarier action. |
| App-private storage (`SharedPreferences`/files) | Loses the templates on reinstall, keeps user-authored content outside the encrypted vault, and contradicts ADR 0006's rule that user data lives in the vault. |
| Ship more built-in templates | Cheap, but it cannot know that this user's "meeting" is a standup with three headings and no attendees. It also grows the shared cross-platform set that must stay identical on desktop, for something personal — which is why Journal and Meeting notes were dropped in favour of user templates instead of adding a fifth. |
| Let the user edit the built-in JSON directly | An editor for a data format, not a feature. The page editor already exists and is the natural place to author a template. |

## Prior art / what this replaces

There is no earlier template store to replace: the four built-ins were compiled into the app from the start (v1.9.0's Android shell and, before it, `packages/editor/src/templates.ts`). This extends that set without changing it, and deliberately leaves ADR number 0008 unassigned — the unmerged Figma work cites it but never committed a record.
