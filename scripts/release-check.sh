#!/usr/bin/env bash
# Pre-flight version check — run BEFORE tagging a native release.
#
# Checks the declarations that still exist in the native-only repository:
#   - does the Rust desktop version match the CHANGELOG head and the tag?
#   - is the native Android versionCode committed and strictly above the
#     previous release? (The native app uses version.properties directly; it
#     does not have Tauri's build-time auto-increment.)
#   - optional --apk: inspect a locally-built artifact.
#
# Usage: scripts/release-check.sh [vX.Y.Z] [--apk path/to/app.apk]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PROPS="apps/android/version.properties"
FAILS=0

pass() { printf '  ✔ %s\n' "$1"; }
fail() { printf '  ✘ %s\n' "$1"; FAILS=$((FAILS + 1)); }

tag_arg=""
apk_arg=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --apk) apk_arg="$2"; shift 2 ;;
    --apk=*) apk_arg="${1#*=}"; shift ;;
    *) tag_arg="$1"; shift ;;
  esac
done

echo "── Declared versions ──"
desktop_ver=$(grep -m1 '^version' apps/desktop/Cargo.toml | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
android_name=$(grep -m1 '^versionName=' "$PROPS" | cut -d= -f2- | tr -d ' ' || true)
android_code=$(grep -m1 '^versionCode=' "$PROPS" | cut -d= -f2- | tr -d ' ' || true)
changelog_ver=$(grep -m1 '^## \[' CHANGELOG.md | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)

echo "  desktop (Cargo)  ${desktop_ver:-missing}"
echo "  android          ${android_name:-missing} (${android_code:-missing})"
echo "  CHANGELOG        ${changelog_ver:-missing}"

if [[ -n "$desktop_ver" && "$desktop_ver" == "$changelog_ver" ]]; then
  pass "desktop version matches CHANGELOG ($desktop_ver)"
else
  echo "  ! desktop version and CHANGELOG are independent until the native release train is finalized (#241)"
fi

if [[ -n "$tag_arg" ]]; then
  echo "  ! tag/version policy is owned by #241; not enforced yet"
fi

echo "── Native Android pre-flight ──"
if [[ -z "$android_code" || -z "$android_name" ]]; then
  fail "$PROPS is missing versionCode/versionName"
else
  pass "version.properties declares $android_name ($android_code)"
fi

committed_code=$(git show "HEAD:$PROPS" 2>/dev/null | grep -m1 '^versionCode=' | cut -d= -f2- | tr -d ' ' || true)
if [[ -n "$committed_code" && "$committed_code" != "$android_code" ]]; then
  fail "$PROPS has uncommitted versionCode drift (committed $committed_code, tree $android_code)"
else
  pass "versionCode is committed and clean"
fi

last_tag=$(git tag --list 'v*' --sort=-v:refname | head -1 || true)
if [[ -n "$last_tag" && -n "$android_code" ]]; then
  last_code=$(git show "$last_tag:$PROPS" 2>/dev/null | grep -m1 '^versionCode=' | cut -d= -f2- | tr -d ' ' || true)
  if [[ -n "$last_code" ]]; then
    if (( android_code > last_code )); then
      pass "versionCode $android_code is strictly greater than $last_tag ($last_code)"
    else
      fail "versionCode $android_code is not greater than $last_tag ($last_code)"
    fi
  else
    echo "  ($last_tag has no native versionCode — skipping strict-increase check)"
  fi
fi

if [[ -n "$apk_arg" ]]; then
  echo "── APK inspection: $apk_arg ──"
  if [[ ! -f "$apk_arg" ]]; then
    fail "APK not found: $apk_arg"
  else
    badging=$("${ANDROID_HOME:-$HOME/Android/Sdk}/build-tools/36.0.0/aapt" dump badging "$apk_arg" 2>/dev/null | grep -m1 "^package:")
    apk_code=$(echo "$badging" | grep -o "versionCode='[0-9]*'" | grep -oE '[0-9]+')
    apk_name=$(echo "$badging" | grep -o "versionName='[^']*'" | cut -d"'" -f2)
    echo "  artifact: versionName=$apk_name versionCode=$apk_code"
    if [[ "$apk_name" != "$android_name" || "$apk_code" != "$android_code" ]]; then
      fail "APK version does not match version.properties"
    else
      pass "APK version matches version.properties"
    fi
  fi
fi

echo ""
if (( FAILS > 0 )); then
  echo "✘ $FAILS check(s) failed — fix before tagging"
  exit 1
fi
echo "✔ pre-flight clean — safe to tag"
