#!/usr/bin/env bash
# Pre-flight version check — run BEFORE tagging a release.
#
# Tells you locally what previously required pushing a tag and waiting for CI:
#   - do all version declarations agree? (package.json, tauri.conf.json,
#     Cargo.toml, CHANGELOG head)
#   - what versionName/versionCode will the next Android build actually ship?
#     CI checks out the *committed* tauri.properties counter and ships
#     counter+1 (bundle.android.autoIncrementVersionCode bumps at build time),
#     so an uncommitted local bump is invisible to CI.
#   - is that code strictly greater than the last release's? (Play Store rule.
#     Android tolerates reinstalling an equal versionCode, which makes a
#     stuck counter silent — everything since v1.2.0 shipped 1001003.)
#   - optional --apk: inspect a locally-built artifact (the old
#     "release a tag to see the version" loop, now offline).
#
# Usage: scripts/release-check.sh [vX.Y.Z] [--apk path/to/app.apk]
#   vX.Y.Z   optional — verify the tag matches the declared version
#   --apk    optional — dump versionCode/versionName of a built APK
#
# Convention after every release: commit the tauri.properties counter the
# build bumped (see the warning this script prints when it drifts).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PROPS="src-tauri/gen/android/app/tauri.properties"
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
npm_ver=$(node -p "require('./package.json').version")
conf_ver=$(node -p "require('./src-tauri/tauri.conf.json').version")
cargo_ver=$(grep -m1 '^version' src-tauri/Cargo.toml | grep -o '[0-9][0-9.]*')
changelog_ver=$(grep -m1 '^## \[' CHANGELOG.md | grep -o '[0-9][0-9.]*' | head -1)

echo "  package.json     $npm_ver"
echo "  tauri.conf.json  $conf_ver"
echo "  Cargo.toml       $cargo_ver"
echo "  CHANGELOG        $changelog_ver"

if [[ "$npm_ver" == "$conf_ver" && "$conf_ver" == "$cargo_ver" && "$cargo_ver" == "$changelog_ver" ]]; then
  pass "all four declarations agree ($npm_ver)"
else
  fail "version declarations disagree (see above)"
fi

if [[ -n "$tag_arg" ]]; then
  want="v$npm_ver"
  if [[ "$tag_arg" == "$want" ]]; then
    pass "tag $tag_arg matches declared version"
  else
    fail "tag $tag_arg ≠ declared version (expected $want)"
  fi
fi

echo "── Android pre-flight (what CI will ship) ──"
committed_counter=$(git show "HEAD:$PROPS" 2>/dev/null | grep -m1 'versionCode=' | grep -oE '[0-9]+' || true)
tree_counter=$(grep -m1 'versionCode=' "$PROPS" | grep -oE '[0-9]+' || true)
if [[ -z "$committed_counter" ]]; then
  fail "no versionCode in committed $PROPS"
elif [[ "$committed_counter" != "$tree_counter" ]]; then
  fail "$PROPS drifted: committed $committed_counter vs working tree $tree_counter — CI builds the committed value. Commit the counter: git add $PROPS && git commit"
else
  pass "counter committed and clean ($committed_counter)"
fi

if [[ -n "$committed_counter" ]]; then
  next_code=$((committed_counter + 1))
  echo "  next release ships: versionName=$conf_ver versionCode=$next_code"

  last_tag=$(git tag --list 'v*' --sort=-v:refname | head -1 || true)
  if [[ -n "$last_tag" ]]; then
    last_counter=$(git show "$last_tag:$PROPS" 2>/dev/null | grep -m1 'versionCode=' | grep -oE '[0-9]+' || true)
    if [[ -n "$last_counter" ]]; then
      shipped_code=$((last_counter + 1))
      echo "  $last_tag shipped:  versionCode=$shipped_code"
      if (( next_code > shipped_code )); then
        pass "next code is strictly greater than $last_tag's"
      else
        fail "next code $next_code is not > what $last_tag shipped ($shipped_code) — commit a bumped $PROPS before tagging (Play Store rejects duplicates; sideloads reinstall the old app silently)"
      fi
    else
      echo "  ($last_tag has no tracked counter — skipping strict-increase check)"
    fi
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
    if [[ "$apk_name" != "$conf_ver" ]]; then
      fail "APK versionName $apk_name ≠ declared $conf_ver — stale build, rebuild before tagging"
    else
      pass "APK versionName matches declared version"
    fi
  fi
fi

echo ""
if (( FAILS > 0 )); then
  echo "✘ $FAILS check(s) failed — fix before tagging"
  exit 1
fi
echo "✔ pre-flight clean — safe to tag"
