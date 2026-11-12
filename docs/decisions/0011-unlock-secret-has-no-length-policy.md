# 0011. The unlock secret has no length policy

- **Status:** Accepted
- **Date:** 2026-11-12
- **Deciders:** Pranesh Selvaraj
- **Supersedes:** — (replaces the 4–12 digit rule introduced with the native gate)
- **Superseded by:** —

## Context

The native gate shipped with a PIN policy copied from the web app: `/^\d{4,12}$/`. It was a reasonable default, but it is a policy the product cannot justify:

- It **refuses a vault that already exists.** A secret created elsewhere — including a one-character one — could not be typed into the gate, so the app was locked out of a vault it had every right to open.
- The **number is arbitrary.** Four digits is not a security boundary; it is a habit from bank cards. The vault key is protected by Argon2id, not by the length of the thing the user remembers.
- It **conflates input method with policy.** The keypad only produces digits because it is a keypad, not because a secret must be numeric. Someone who wants a passphrase should not have to discover a separate tab for it.

The request was explicit: no minimum, no maximum, any character — one number or one letter is a valid secret.

## Decision

**Any non-empty string is a valid unlock secret. There is no minimum length and no maximum length, and no character restriction.**

- `PinRules` (the Android gate's policy object) keeps only `isValid(secret) = secret.isNotEmpty()`. The numeric keypad keeps accepting digits without a cap; the dot row stops growing at twelve and shows the count beyond that, so the display stays sane without limiting the secret.
- The Password tab and the recovery/restore dialogs accept arbitrary text (letters, symbols, spaces), because their fields are ordinary text fields.
- **The UI states the trade-off instead of enforcing it**: the registration note reads "Any PIN or password works, with no minimum length — but a longer one is much harder to guess."
- Nothing changes in the core: `create_vault`/`set_vault_password` derive the key blob with Argon2id over whatever string they are given, which is why the app, not the core, was the wrong place to hold this rule.

## Consequences

- **A one-character secret is weak, and this is now possible on purpose.** An attacker holding the vault file can try every digit (ten candidates) or every printable character (about a hundred) against the key blob. Each guess still costs a full Argon2id derivation at 64 MiB / 3 iterations, so it is not instant, but it is minutes of work, not years. The recovery phrase and the vault's encryption stay intact; what is lost is the *cost of guessing the unlock secret* for vaults whose owner chose a short one.
- **The app can open any vault it is given**, which was the point: a vault created by the desktop app with a 1–3 character password, or with a passphrase containing spaces, is no longer a dead end on the phone.
- **The gate no longer implies a format.** Labels say "Create a PIN", "Enter your PIN", "New PIN or password"; the keypad is described as the numeric input method rather than as the rule.
- **The docs and the shared web app disagree for now.** `apps/frontend` still enforces `/^\d{4,12}$/` for its PIN tab. That is deliberate: this decision is about the native gate, and the web app is the retiring product. If the web app should follow, it is a one-line change in `VaultGuard.svelte` and belongs to its own change.
- **Copy is the only guard rail.** There is no strength meter and no nagging. A user who wants a single digit gets it, and the recovery phrase keeps being the answer when that digit is forgotten.
- **The gate must not reveal the length, visually or aloud.** The dot row keeps four slots for any secret up to four characters and is hidden from accessibility services: a screen reader that said "5 characters entered" would tell everyone in the room the length, which is the same leak as drawing a single dot for a one-character secret. The field's own label is the only announcement. Each keypress is still audible to someone sitting next to the user — that is inherent to any on-screen keypad read by TalkBack, and the answer for that case is a headset or the Braille keyboard, not a silent keypad.
- **Attempts must be throttled, and now are.** With no length policy, the *only* barrier to guessing a short secret is the time between attempts: Argon2id costs a fraction of a second per try, so ten taps would walk the whole digit space. The gate therefore records consecutive failures (device-local, surviving a restart) and refuses attempts on a schedule that starts gentle — the first two cost nothing, then 1 s, 3 s, 10 s, 30 s, a minute each, capped at five — while the button counts down ("Try again in 2s"). A successful unlock clears it. If that throttling is ever removed, this record's policy stops being defensible and must be revisited.

## Alternatives considered

| Alternative | Why it lost |
| --- | --- |
| Keep 4–12 digits | The status quo: refuses existing vaults, refuses passphrases, and pretends a length rule is a security control. |
| No minimum, but digits only | Half a policy: still refuses a vault whose secret has a letter, which was the reported failure. |
| A strength meter or a warning dialog for short secrets | Turns a documented trade-off into a nag on every unlock, and the user who wants a single digit is not confused about its strength. The copy on the registration screen says it once, where the choice is made. |
| A minimum of one *and* a maximum (say 64) | Any maximum has to be justified to the person whose passphrase is longer. The dot row is a display concern and is handled as one. |

## Prior art / what this replaces

The 4–12 rule came from the web app's `VaultGuard.svelte` (v1.12.0's PIN unlock) and was carried into the native gate when it was rebuilt. This record does not change the web app.
