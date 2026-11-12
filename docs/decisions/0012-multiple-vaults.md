# 0012. Multiple vaults with independent unlock secrets

- **Status:** Proposed — the plan in [`docs/plans/multiple-vaults.md`](../plans/multiple-vaults.md) is the execution criteria; nothing is implemented yet
- **Date:** 2026-11-12
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** —
- **Superseded by:** —

## Context

The app holds one vault. Someone who takes the unlocked phone, or who forces the owner to unlock it, gets everything in it: there is no smaller answer to give, and refusing is visible.

The requested feature is a set of **independent vaults** in one install, each opened by its own secret, with the app never revealing that more than one exists. A user can then open a vault holding ordinary notes while the important one stays closed, and the app has nothing to refuse.

That is a security claim, so the first thing this record does is bound it.

## Decision

**Support several independent vaults per install, each with its own key, database, sync identity and backups, unlocked from one field by trying the secret against every slot — and make the app's surface incapable of revealing how many exist.**

- One unlock field. No picker, no list, no count, no "switch vault" affordance that implies another exists.
- A wrong secret, an empty slot and a decoy all fail with the same message, the same field state and comparable duration. The unlock performs a **fixed** number of key derivations per attempt, so the cost does not scale with the number of vaults.
- Vaults do not share an index, a search, a recents list, a widget cache or a settings view. Only one is open at a time.
- A fresh install with one vault behaves exactly like today's app.

## What this does not buy

**It does not hide vaults from a forensic attacker.** With the app's files on disk, the number of databases and their sizes are observable — a padded, fixed-slot container would be needed to hide even the count, and that costs the direct file access SQLCipher gives the core today. This record therefore claims **coercion resistance, not plausible deniability against device forensics**, and the plan keeps the container as an optional, explicitly costed phase.

It does not hide the app or the feature, and it does not protect against an attacker who already learned a secret was used.

## Consequences

- **Storage becomes a store, not a path.** `core-api` stops owning "the vault" and owns "a slot in a store"; the FFI gains creation of an additional slot and loses any call that enumerates the old single path.
- **Timing is part of the contract.** Any code path that derives against a number of slots proportional to the number of vaults leaks that number. The fixed-cost derivation has to be a test, not a habit.
- **Every surface must be re-read for leakage**, because the guarantee is negative: settings copy, the drawer, a notification's text, the widget placeholder, the share target's failure message and the restore flow can each break it in one word.
- **Sync and backup multiply.** Each vault pairs separately and exports separately; an export that carries a store index would be a leak in a file the user mails to themselves.
- **The desktop app has to answer**, or the extra vaults are phone-only by decision. Leaving it unstated is how a user loses the second vault when they restore on the desktop.
- **The default stays simple.** A single-vault install must not pay for the feature with extra setup, extra taps or a new concept on the lock screen.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| A visible vault picker after unlock | The most usable and the least useful: it advertises exactly what a coerced user needs to hide, and "which one do you want?" is the question the feature exists to avoid. |
| A separate app (e.g. a discreet second package) | Discoverable by the same person holding the phone, and two apps to install, update and explain. |
| One vault with an "emergency password" that opens a restricted view | Restricted views leak: the owner cannot produce a plausible full app, and the restricted view has to explain what is hidden. |
| Full plausible deniability via a padded fixed-slot container first | The right end state for forensics, but it changes the storage engine before the feature has a single user. Costed as Phase 4, not assumed. |
| Do nothing | Leaves the coercion case with no answer at all, which is the case the owner raised. |

## Prior art / what this replaces

There is no multi-vault code to replace; the store is single today (`enclave.db` + `vault.key` in the app's data directory) and ADR 0007's restore replaces that one vault. The decoy-vault pattern is long-established (TrueCrypt/VeraCrypt hidden volumes, duress passwords in phone lockers); this record borrows the *honesty* convention of those designs — the guarantee is stated precisely, including where it stops.
