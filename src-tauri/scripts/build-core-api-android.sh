#!/usr/bin/env bash
# Build core-api (the native shell's Rust core) for the Android ABIs and copy
# the .so files into the app's jniLibs so UniFFI/JNA can load them.
#
# Requires: cargo-ndk + a Rust toolchain with the Android targets, and
# NDK_HOME (or ANDROID_NDK_HOME) pointing at the installed NDK. See
# ~/.enclave-dev/ui-harness/setup-android.sh for the one-time setup.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)" # src-tauri
OUT="$ROOT/gen/android/app/src/main/jniLibs"
PROFILE="${CORE_API_PROFILE:-debug}"

if ! command -v cargo-ndk >/dev/null 2>&1 && [[ ! -x "$HOME/.cargo/bin/cargo-ndk" ]]; then
	echo "cargo-ndk not found — install it with: cargo install cargo-ndk" >&2
	exit 1
fi
export PATH="$HOME/.cargo/bin:$PATH"
: "${NDK_HOME:=${ANDROID_NDK_HOME:-}}"
if [[ -z "$NDK_HOME" ]]; then
	echo "NDK_HOME is not set — source ~/.enclave-dev/android-env.sh" >&2
	exit 1
fi

# `--profile debug` is rejected by cargo (reserved); omit it for debug.
profile_args=()
[[ "$PROFILE" == "release" ]] && profile_args=(--release)
for abi in arm64-v8a x86_64; do
	echo "▶ core-api · $abi ($PROFILE)"
	(cd "$ROOT" && cargo ndk -t "$abi" build -p core-api --features uniffi "${profile_args[@]}")
done

for pair in "arm64-v8a:aarch64-linux-android" "x86_64:x86_64-linux-android"; do
	abi="${pair%%:*}"
	triple="${pair##*:}"
	src="$ROOT/target/$triple/$PROFILE/libcore_api.so"
	[[ -f "$src" ]] || { echo "missing $src" >&2; exit 1; }
	mkdir -p "$OUT/$abi"
	cp "$src" "$OUT/$abi/libcore_api.so"
done

echo "✔ jniLibs: $OUT/{arm64-v8a,x86_64}/libcore_api.so"
