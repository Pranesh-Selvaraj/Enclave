# Enclave Android — native app

The native Android app: **Kotlin + Jetpack Compose**, consuming the shared Rust core through the committed **UniFFI** bindings. There is no webview and no Svelte frontend.

Tracking: [native Android device pass #188](https://github.com/Pranesh-Selvaraj/Enclave/issues/188). Architecture: [ADR 0005](../../docs/decisions/0005-android-ui-stack.md).

## Status

In development, not yet the signed release build. The gate, notes home, editor, sync service, widgets, backup/restore and settings are built; the remaining real-device checklist and native release pipeline are in progress.

## Build and test

Prerequisites: JDK 21, Android SDK platform 36, build-tools 36.0.0, NDK 29.0.13846066, `cargo-ndk`, and the Android Rust targets.

```bash
source ~/.enclave-dev/android-env.sh
./apps/android/scripts/build-core.sh   # builds libcore_api.so + regenerates the Kotlin bindings
cd apps/android
./gradlew assembleDebug testDebugUnitTest
```

The core is shared with the Rust desktop; `core/` is the only implementation of vault, crypto, storage, search and sync. A core change must update the committed Kotlin bindings and the contract fixtures.

## Signing and release

`scripts/android-sign.sh` signs the release APK/AAB with the keystore secrets. The native release pipeline is being completed in [#241](https://github.com/Pranesh-Selvaraj/Enclave/issues/241); tag builds are a first iteration until proven.

## What is here

```
app/src/main/java/com/enclave/app/
├── MainActivity.kt, EnclaveIntents.kt    # entry points, share target, shortcuts
├── core/Core.kt                          # UniFFI/JNA handle, sync controller
├── editor/                               # ProseMirror block model, editor surface, templates
├── ui/                                   # gate, home, note screen, settings, sync, theme
├── SyncService.kt                        # foreground sync service
└── *Widget.kt, NewNoteTileService.kt     # Glance widgets, Quick Settings tile
```

## Notes

- The applicationId is `com.enclave.app`, shared with the retired Tauri Android app so an existing data directory can be picked up during migration.
- The editor writes the same ProseMirror JSON as the Rust desktop; `block-schema-v1.json` is the golden fixture.
- Widgets read a Keystore-wrapped cache; the vault key never leaves the Rust core.
- Sync is LAN-only, mutually authenticated and encrypted (protocol v3). Protocol v4 is planned in the core.
