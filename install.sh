#!/bin/sh
# Installs the latest Stowaway into /Applications.
#   curl -fsSL https://github.com/wajedhossainbd/stowaway/releases/latest/download/install.sh | sh
set -eu
REPO=wajedhossainbd/stowaway
APP=/Applications/Stowaway.app
[ "$(uname -s)" = Darwin ] || { echo "Stowaway runs on macOS only."; exit 1; }
[ "$(uname -m)" = arm64 ] || { echo "This build is for Apple Silicon Macs (M1 or later)."; exit 1; }

TMP=$(mktemp -d)
MNT="$TMP/mnt"
trap 'hdiutil detach "$MNT" -quiet 2>/dev/null || true; rm -rf "$TMP"' EXIT

echo "Downloading Stowaway…"
# While the repository is private the download needs your GitHub sign-in, which the gh tool has.
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
  gh release download --repo "$REPO" --pattern Stowaway.dmg --dir "$TMP"
else
  curl -fL --progress-bar -o "$TMP/Stowaway.dmg" "https://github.com/$REPO/releases/latest/download/Stowaway.dmg"
fi

hdiutil attach "$TMP/Stowaway.dmg" -nobrowse -quiet -mountpoint "$MNT"
osascript -e 'tell application id "bd.wajed.stowaway" to quit' >/dev/null 2>&1 || true
# ditto replaces the old copy in place; your settings and moved folders are not touched.
ditto "$MNT/Stowaway.app" "$APP"
# Stowaway is not notarised by Apple yet, so macOS would refuse a downloaded copy without this.
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true
open "$APP"
echo "Stowaway is installed in Applications and open."
