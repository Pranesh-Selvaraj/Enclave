# Enclave Mobile (Android) — Status, Issues & Release Engineering

Working document for the Android effort (`feat/android-mobile`). Each issue
below is fixed on its own branch, documented here, and merged to `main`.

## Build & run

Prereqs: JDK 21, Android SDK (platform 36, build-tools 36.0.0, NDK 29.0.13846066),
rustup targets `aarch64-linux-android` / `x86_64-linux-android`.

```bash
export ANDROID_HOME=~/Android/Sdk NDK_HOME=$ANDROID_HOME/ndk/29.0.13846066 JAVA_HOME=<jdk21>
cd src-tauri
npx tauri android build --target aarch64        # signed release APK/AAB (see scripts/android-sign.sh)
npx tauri android dev                           # hot-reload on emulator/device
```

Emulator (KVM required — `sudo usermod -aG kvm $USER` then re-login):

```bash
~/Android/Sdk/emulator/emulator -avd <avd> -no-window -no-audio -no-snapshot -gpu swiftshader_indirect
```

## Issues & fixes

| # | Issue | Severity | Fix branch | Status |
|---|-------|----------|------------|--------|
| 1 | Release APK blocks P2P sync: generated gradle sets `usesCleartextTraffic=false` for release, and Android enforces cleartext policy at the platform level — `ws://` sockets fail before app code runs. Safe to allow now: sync payloads are AEAD-sealed (XChaCha20-Poly1305) at the app layer. | High | `fix/android/sync-cleartext` | fixed |
| 2 | No lock on background: the vault stays unlocked in memory when the app loses visibility (phones: app switcher / screen off). | Medium | `fix/android/lock-on-background` | fixed |
| 3 | `gen/android/` was gitignored, so native customizations (cleartext, icons, future MulticastLock) were regenerated and lost on every `tauri android init`. | Medium | `fix/android/sync-cleartext` | fixed |
| 4 | Release signing was manual (`apksigner`/`jarsigner` commands); keystore handling undocumented in-repo. | Medium | `feat/android-signing` | fixed |
| 5 | No Android CI — only desktop builds in `.github/workflows/build.yml`. | Medium | `feat/android-ci` | fixed |
| 6 | Default Tauri icon on Android (no adaptive icon). | Low | `feat/android-branding` | fixed |
| 7 | versionCode not managed (`tauri.properties` drift; Play requires strictly increasing codes). | Low | `feat/android-versioning` | fixed |

## Open items (not yet fixed)

- **P2P sync on real hardware** — mDNS discovery on Android phones is unverified
  (Android multicast behavior; may need a native `MulticastLock`). Encrypted
  transport is unit-tested; discovery is not device-tested.
- **RAG on arm64 device** — model download + inference compiled but not run on-device.
- **Mobile UX pass** — desktop UI in a WebView: touch targets, hover menus,
  TipTap keyboard handling, safe areas, graph view.
- **File dialogs / backup / import-export on Android** — dialog plugin wired, untested.
- Old desktop builds (v1.1.x) cannot sync with this protocol (auth handshake is
  a wire-format bump) — desktop must ship the same sync code.

## Release engineering notes

- versionCode: semver-derived (`major*1e6 + minor*1e3 + patch`) unless
  `autoIncrementVersionCode` is set — see `tauri.conf.json > bundle.android`.
- Keystore lives OUTSIDE the repo (`~/Android/keystore/enclave-release.jks`);
  CI reads it from `ANDROID_KEYSTORE_B64` secrets.
- Signing: `scripts/android-sign.sh` (zipalign + apksigner for APK,
  jarsigner for AAB).
