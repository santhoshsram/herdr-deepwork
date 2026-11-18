#!/bin/sh
# Runs at `herdr plugin install` and `herdr plugin update`, with no plugin
# env vars. Fetches the prebuilt binary for this machine into the plugin's
# state folder: the tab bar and the popup both run it there, because the
# plugin folder's path changes with every update.
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
mkdir -p "$dest" || fail "cannot create $dest"
# The tab bar runs $dest/herdr-focus every second. Writing over it in place
# fails on Linux (text file busy) and on macOS gets later runs killed for a
# stale code signature, so the copy lands beside it and is renamed over it.
cp "$tmp/herdr-focus" "$dest/.herdr-focus.new" || fail "cannot copy to $dest"
mv -f "$dest/.herdr-focus.new" "$dest/herdr-focus" || fail "cannot copy to $dest"
