#!/bin/sh
# Installs the latest iStash into /Applications.
#   curl -fsSL https://github.com/wajedhossainbd/iStash/releases/latest/download/install.sh | sh
set -eu
REPO=wajedhossainbd/iStash
APP=/Applications/iStash.app
[ "$(uname -s)" = Darwin ] || { echo "iStash runs on macOS only."; exit 1; }
[ "$(uname -m)" = arm64 ] || { echo "This build is for Apple Silicon Macs (M1 or later)."; exit 1; }

TMP=$(mktemp -d)
MNT="$TMP/mnt"
trap 'hdiutil detach "$MNT" -quiet 2>/dev/null || true; rm -rf "$TMP"' EXIT

echo "Downloading iStash…"
# While the repository is private the download needs your GitHub sign-in, which the gh tool has.
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
  gh release download --repo "$REPO" --pattern iStash.dmg --dir "$TMP"
else
  curl -fL --progress-bar -o "$TMP/iStash.dmg" "https://github.com/$REPO/releases/latest/download/iStash.dmg"
fi

hdiutil attach "$TMP/iStash.dmg" -nobrowse -quiet -mountpoint "$MNT"
# Never replace iStash while it is moving a folder.
if pgrep -f 'stowaway-app --engine' >/dev/null 2>&1; then
  echo "iStash is moving a folder right now. Run this again when it has finished."; exit 1
fi
# Quit the running copy for real (a normal Quit only hides iStash to the menu bar), then wait for it to exit.
pkill -x stowaway-app 2>/dev/null || true
i=0; while pgrep -x stowaway-app >/dev/null 2>&1 && [ $i -lt 20 ]; do sleep 0.5; i=$((i+1)); done
# ditto replaces the old copy in place; your settings and moved folders are not touched.
ditto "$MNT/iStash.app" "$APP"
# iStash is not notarised by Apple yet, so macOS would refuse a downloaded copy without this.
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true
# iStash used to be called Stowaway. Remove that older copy, but only if it really is this app.
OLD=/Applications/Stowaway.app
if [ -d "$OLD" ] && [ "$(defaults read "$OLD/Contents/Info" CFBundleIdentifier 2>/dev/null)" = bd.wajed.stowaway ]; then rm -rf "$OLD"; fi
open "$APP"
echo "iStash is installed in Applications and open."
