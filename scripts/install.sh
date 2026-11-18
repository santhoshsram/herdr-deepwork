#!/bin/sh
# Runs at `herdr plugin install` and `herdr plugin update`, with no plugin
# env vars. Fetches the prebuilt binary for this machine, keeps a copy in the
# plugin folder and one at a path that survives plugin updates.
set -eu

fail() {
  echo "herdr-focus install: $1" >&2
  exit 1
}

# herdr runs build commands from the plugin folder; this also makes a direct
# run from elsewhere work.
case "$0" in
  */*) cd "${0%/*}" || fail "cannot enter the scripts folder" ;;
esac
[ -f herdr-plugin.toml ] || cd .. || fail "cannot find the plugin folder"

system=$(uname -s)
machine=$(uname -m)
case "$system $machine" in
  "Darwin arm64") asset=herdr-focus-macos-arm64 ;;
  "Linux x86_64") asset=herdr-focus-linux-x86_64 ;;
  "Linux aarch64") asset=herdr-focus-linux-arm64 ;;
  *) fail "no prebuilt binary for $system $machine" ;;
esac

version=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' herdr-plugin.toml | head -n 1)
[ -n "$version" ] || fail "no version in herdr-plugin.toml"

base=${HERDR_FOCUS_RELEASE_URL:-https://github.com/santhoshsram/herdr-focus/releases/download}
url="$base/v$version/$asset"

tmp=$(mktemp -d) || fail "cannot make a temporary folder"
trap 'rm -rf "$tmp"' EXIT

# curl's own message is dropped so a failure is one line, ours.
curl -fsSL -o "$tmp/herdr-focus" "$url" 2>/dev/null || fail "download failed: $url"
chmod +x "$tmp/herdr-focus"

dest="$HOME/.local/state/herdr/plugins/focus"
mkdir -p bin "$dest" || fail "cannot create install folders"
# The tab bar runs $dest/herdr-focus every second. Writing over it in place
# fails on Linux (text file busy) and on macOS gets later runs killed for a
# stale code signature, so each copy lands beside its target and is renamed
# over it. Both are staged first so a failure installs neither.
cp "$tmp/herdr-focus" bin/.herdr-focus.new || fail "cannot copy into the plugin folder"
cp "$tmp/herdr-focus" "$dest/.herdr-focus.new" || {
  rm -f bin/.herdr-focus.new
  fail "cannot copy to $dest"
}
mv -f bin/.herdr-focus.new bin/herdr-focus || fail "cannot copy into the plugin folder"
mv -f "$dest/.herdr-focus.new" "$dest/herdr-focus" || fail "cannot copy to $dest"
