# Plan: multiple vaults with independent unlock secrets

Companion to [ADR 0012](../decisions/0012-multiple-vaults.md). Criteria-driven: phases, dependencies and exit criteria — no dates, no durations.

## Goal

One installed app can hold **several independent vaults**, each opened by its own secret, and the app's surface never says how many exist — not in the UI, not in a string, not in a log, not in a notification, and not by which screen appears when a secret is wrong.

The motivating case is coercion: someone is asked to open the app. They enter the secret of a vault that holds innocuous notes; the app opens it and looks like the whole product. The real vault is not discoverable from inside the app, so there is nothing to refuse to open.

## Threat model — say what this does and does not buy

**In scope: coercion at the lock screen.** A person holding the unlocked phone, or forcing the owner to unlock it, must not be able to learn whether other vaults exist. That means: one unlock field, one generic failure, an identical response whether the secret matches nothing or matches a decoy, no "vault 2 of 3" anywhere, and no settings screen that lists or counts them.

**Out of scope: forensic analysis of the device.** With filesystem access (root, or an extracted app-data backup) an attacker can count files and compare sizes. Two vaults produce two databases of different sizes; a fixed-slot, size-padded container is the only way to hide that, and it costs the direct-SQLite storage this project depends on. **This plan does not promise deniability against a forensic adversary**, and the ADR records that limitation explicitly so nobody assumes it later.

**Also out of scope: resisting an attacker who already knows a secret was used.** Timing, error text and screen transitions are equalised as far as is practical, but the app cannot hide that opening a vault takes *some* time.

## Invariants

1. **No count, anywhere.** No UI text, accessibility label, notification, log line, crash report or analytics event may mention the number of vaults or imply that more than one exists. Strings are reviewed for this the way they are reviewed for the encryption claims.
2. **One attempt is indistinguishable from another.** A wrong secret, a secret that matches an empty slot, and a secret for a decoy all fail identically: same message, same field state, same screen, and comparable duration.
3. **Vaults are truly independent.** Own master key, own SQLCipher database, own sync identity, own backups, own widgets. No shared indexes, no shared FTS, no cross-vault search.
4. **The default install is exactly today's app.** A fresh install with one vault must look and behave like the current product.

## Phases

### Phase 1 — storage and unlock (blocking)

- [ ] A **vault store** in the core: an ordered set of vault slots, each with its own database and key blob, addressed by an opaque id the UI never shows. Opening a slot is "try the secret against every slot's key blob".
- [ ] **Uniform failure and timing.** Derive the secret against a *fixed* number of slots per attempt, so the work does not scale with how many vaults exist; a wrong secret pays the same cost as a right one. Decide the fixed number (e.g. always try N=2 slots even when one is empty) and record it in the ADR.
- [ ] **Creation** adds a slot without any visible confirmation of how many now exist ("Vault ready", not "Vault 2 created").
- [ ] **Locking** closes every open session; only one vault is open at a time.
- [ ] **Migration**: an existing single-vault install becomes a one-slot store with no data change and no user action.

**Exit criteria:** on a device, with two vaults created, an onlooker inspecting every screen — lock, home, drawer, settings, search, about — cannot tell that two exist; a wrong secret on a two-vault install produces the same message and a duration within noise of the single-vault install; `cargo test` proves a wrong secret performs the fixed number of derivations regardless of how many slots are present.

### Phase 2 — everything that touches the open vault (blocking)

- [ ] **Sync**: each vault is its own peer (its identity already derives from its key). Verify that two vaults on one phone do not discover each other over the LAN, and that peers see only the open vault.
- [ ] **Backups**: export names and contents carry nothing about the store — no index, no count. A backup of a decoy vault must be an entirely ordinary backup.
- [ ] **Widgets and the QS tile**: read only the open vault; while locked they render the placeholder from the last opened vault, never a merged list.
- [ ] **Share target and quick capture** must not be able to write into a vault that is not open without unlocking it first.
- [ ] **Settings**: no row may enumerate or count vaults. "Forget this vault" acts on the open one only, and says so without hinting at the others.

**Exit criteria:** the full manual pass from `docs/STATUS.md` runs on a two-vault install with no cross-vault leak; the settings screen, widgets and notifications mention only "this vault" / "the vault".

### Phase 3 — the second surface (blocking before any release that claims the feature)

- [ ] The **desktop app** must open a multi-vault store (or the native app's extra vaults must be documented as phone-only, which is a product decision, not an oversight).
- [ ] Cross-device behaviour is decided per vault: each vault pairs and syncs separately, and pairing one must not reveal another.
- [ ] **Restore** semantics (ADR 0007) are extended to "restore into a new slot" without overwriting an existing vault silently.

**Exit criteria:** a written statement in `STATUS.md` — either "desktop opens all vaults" or "extra vaults are phone-only, by decision N" — plus the corresponding tests.

### Phase 4 — the optional hardening (not blocking)

- [ ] A **fixed-slot, size-padded container** for forensic deniability, if it can be done without giving up SQLCipher's direct file access, or with a documented decryption-to-temp compromise. This is the only phase that changes the threat model, and it may be rejected on cost.

## Storage options considered

| Option | Hides the count from the app UI | Hides it from a forensic attacker | Cost |
| --- | --- | --- | --- |
| N files, opaque names (recommended for Phase 1) | yes | **no** — file count and sizes | none beyond the store abstraction |
| N files, padded to a fixed size | yes | partially — count still visible if names are enumerable, sizes equal | padding wastes space and breaks SQLite's free-page reuse |
| One container, fixed K slots, each padding-padded | yes | yes for the count, sizes still leak | the database no longer lives in a file SQLCipher can open directly |
| A master vault index with a master secret | yes | no | breaks the premise: one more secret to be coerced for, and a single point of failure |

## Open questions

- **How many slots does the fixed-cost unlock try?** Two is cheap and makes the common case fast; it also means an attacker learns nothing from time when the store holds one or two vaults. More slots is a stronger statement and a slower unlock — a decision for the ADR.
- **Does the decoy need a distinct sync identity?** It does if it should look like a working vault on the LAN; that is more "real" but also more surface.
- **What happens when the app is forced to open a vault that is empty?** An empty decoy is suspicious; the guidance may be to seed it with a few ordinary notes at creation.
- **Do vaults share the widget cache?** The Keystore-wrapped cache must be per vault, or a widget reveals the last vault's titles after a switch.

## Non-goals

- Hiding the *existence of the app*, or of the vault feature itself.
- Deniability against someone with the device's filesystem (see the threat model).
- Cross-vault search, a merged recents list, or any UI that spans vaults.
- Hiding a vault from the operating system's own backup or from a rooted device.
