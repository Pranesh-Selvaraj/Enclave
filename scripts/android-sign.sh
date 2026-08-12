#!/usr/bin/env bash
# Sign the Android release APK (zipalign + apksigner) and AAB (jarsigner).
#
# Usage: scripts/android-sign.sh
#   Run from the repo root after `npx tauri android build --target <abi>`.
#
# Environment (all optional):
#   KEYSTORE_PATH     keystore file (default: $HOME/Android/keystore/enclave-release.jks)
#   KEYSTORE_B64      base64-encoded keystore (CI); takes precedence, decoded to a temp file
#   KEYSTORE_PASSWORD keystore password (default: read from $KEYSTORE_PATH.password)
#   KEY_PASSWORD      key password (default: same as KEYSTORE_PASSWORD)
#   KEY_ALIAS         key alias (default: enclave)
#   APK               input APK (default: tauri's app-universal-release-unsigned.apk)
#   AAB               input AAB (default: tauri's app-universal-release.aab)
#   ANDROID_HOME      Android SDK root (default: $HOME/Android/Sdk)
#   JAVA_HOME         JDK home (default: resolved from `which java`)
#
# With no keystore available (e.g. fork PRs) the script warns and exits 0 —
# CI stays green while release builds require the secrets.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Only temp files go here — never the real keystore.
TMP_FILES=()
cleanup() { rm -f "${TMP_FILES[@]:-}"; }
trap cleanup EXIT

ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
BUILD_TOOLS="$ANDROID_HOME/build-tools/36.0.0"
KEY_ALIAS="${KEY_ALIAS:-enclave}"
APK="${APK:-src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk}"
AAB="${AAB:-src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab}"
OUT_APK="${OUT_APK:-$(dirname "$APK")/enclave-android-release.apk}"

# ── Resolve keystore ─────────────────────────────────────────────────────────
KEYSTORE=""
if [[ -n "${KEYSTORE_B64:-}" ]]; then
  KEYSTORE="$(mktemp)"
  TMP_FILES+=("$KEYSTORE")
  echo "$KEYSTORE_B64" | base64 -d > "$KEYSTORE"
elif [[ -n "${KEYSTORE_PATH:-}" && -f "${KEYSTORE_PATH}" ]]; then
  KEYSTORE="$KEYSTORE_PATH"
elif [[ -f "$HOME/Android/keystore/enclave-release.jks" ]]; then
  KEYSTORE="$HOME/Android/keystore/enclave-release.jks"
fi

if [[ -z "$KEYSTORE" ]]; then
  echo "android-sign: no keystore found (KEYSTORE_PATH/KEYSTORE_B64) — skipping signing"
  exit 0
fi

KEYSTORE_PASSWORD="${KEYSTORE_PASSWORD:-$(cat "$KEYSTORE.password" 2>/dev/null || true)}"
KEY_PASSWORD="${KEY_PASSWORD:-$KEYSTORE_PASSWORD}"
if [[ -z "$KEYSTORE_PASSWORD" ]]; then
  echo "android-sign: keystore found but no password (KEYSTORE_PASSWORD or $KEYSTORE.password) — skipping signing" >&2
  exit 1
fi

JAVA_BIN="${JAVA_HOME:+$JAVA_HOME/bin}"

# ── APK: zipalign + apksigner ────────────────────────────────────────────────
if [[ -f "$APK" ]]; then
  ALIGNED="$(mktemp)"
  TMP_FILES+=("$ALIGNED")
  "$BUILD_TOOLS/zipalign" -f -p 4 "$APK" "$ALIGNED"
  "$BUILD_TOOLS/apksigner" sign \
    --ks "$KEYSTORE" --ks-key-alias "$KEY_ALIAS" \
    --ks-pass "pass:$KEYSTORE_PASSWORD" --key-pass "pass:$KEY_PASSWORD" \
    --out "$OUT_APK" "$ALIGNED"
  "$BUILD_TOOLS/apksigner" verify --print-certs "$OUT_APK" | head -1
  echo "android-sign: signed APK -> $OUT_APK"
else
  echo "android-sign: no APK at $APK — skipping APK signing"
fi

# ── AAB: jarsigner (JAR-style signing) ───────────────────────────────────────
if [[ -f "$AAB" ]]; then
  JARSIGNER=""
  if [[ -n "${JAVA_HOME:-}" && -x "$JAVA_HOME/bin/jarsigner" ]]; then
    JARSIGNER="$JAVA_HOME/bin/jarsigner"
  elif command -v jarsigner > /dev/null 2>&1; then
    JARSIGNER="$(command -v jarsigner)"
  else
    echo "android-sign: jarsigner not found — set JAVA_HOME to a JDK" >&2
    exit 1
  fi
  "$JARSIGNER" -keystore "$KEYSTORE" \
    -storepass "$KEYSTORE_PASSWORD" -keypass "$KEY_PASSWORD" \
    "$AAB" "$KEY_ALIAS" 2>&1 | tail -1
  "$JARSIGNER" -verify "$AAB" | head -1
  echo "android-sign: signed AAB -> $AAB"
else
  echo "android-sign: no AAB at $AAB — skipping AAB signing"
fi
