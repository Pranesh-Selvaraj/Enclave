#!/usr/bin/env bash
# Build the shared Rust core for the Android app and place the artifacts where
# Gradle expects them:
#
#   app/src/main/java/uniffi/...     generated Kotlin bindings (committed)
#   app/src/main/jniLibs/<abi>/      libcore_api.so (gitignored, built here)
#
# Requires the Android toolchain on PATH — source ~/.enclave-dev/android-env.sh
# (JDK 21 + Android SDK + NDK 29) and have cargo-ndk installed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CORE="$ROOT/core"
APP="$ROOT/apps/android/app"
ABIS="${ENCLAVE_ANDROID_ABIS:-arm64-v8a x86_64}"

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "cargo-ndk not found — run: cargo install cargo-ndk --locked" >&2
  exit 1
fi
if [[ -z "${ANDROID_HOME:-}" || -z "${NDK_HOME:-}" ]]; then
  echo "ANDROID_HOME/NDK_HOME unset — source ~/.enclave-dev/android-env.sh first" >&2
  exit 1
fi

cd "$CORE"

# The host cdylib (used only to generate the bindings) links SQLCipher, which
# needs OpenSSL headers. Most machines have them system-wide; this repo's dev
# image provides them through an env file — override with ENCLAVE_HOST_ENV.
HOST_ENV="${ENCLAVE_HOST_ENV:-$HOME/.enclave-dev/env.sh}"

if [[ -f "$HOST_ENV" ]]; then
  echo "== host cdylib + Kotlin bindings (via $HOST_ENV) =="
  (
    # shellcheck disable=SC1090
    source "$HOST_ENV"
    cargo build -p core-api --features uniffi
    cargo run -q -p core-api --features uniffi,cli --bin uniffi-bindgen -- \
      generate --library target/debug/libcore_api.so \
      --language kotlin --out-dir "$APP/src/main/java"
  )
else
  echo "== host cdylib + Kotlin bindings =="
  cargo build -p core-api --features uniffi
  cargo run -q -p core-api --features uniffi,cli --bin uniffi-bindgen -- \
    generate --library target/debug/libcore_api.so \
    --language kotlin --out-dir "$APP/src/main/java"
fi

for abi in $ABIS; do
  case "$abi" in
    arm64-v8a) triple=aarch64-linux-android ;;
    x86_64)    triple=x86_64-linux-android ;;
    *) echo "unsupported ABI: $abi" >&2; exit 1 ;;
  esac
  echo "== $abi ($triple) =="
  cargo ndk -t "$abi" build -p core-api --features uniffi
  mkdir -p "$APP/src/main/jniLibs/$abi"
  cp "target/$triple/debug/libcore_api.so" "$APP/src/main/jniLibs/$abi/libcore_api.so"
done

echo "core ready:"
ls -lh "$APP/src/main/jniLibs/"*/libcore_api.so
