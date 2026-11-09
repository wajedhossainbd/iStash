#!/bin/sh
# Copies real app icons from /Applications into icons/ for local previews only (icons/ is git-ignored:
# they are other companies' trademarks). The app itself reads icons from bundles at runtime.
cd "$(dirname "$0")" && mkdir -p icons
grab() { # grab <id> <App.app>
  app="/Applications/$2"; [ -d "$app" ] || return 0
  name=$(defaults read "$app/Contents/Info" CFBundleIconFile 2>/dev/null) || return 0
  case "$name" in *.icns) ;; *) name="$name.icns" ;; esac
  [ -f "$app/Contents/Resources/$name" ] && sips -s format png -Z 64 "$app/Contents/Resources/$name" --out "icons/$1.png" >/dev/null && echo "icons/$1.png"
}
grab ollama Ollama.app; grab xcode Xcode.app; grab docker Docker.app; grab claude Claude.app
grab lmstudio "LM Studio.app"; grab chrome "Google Chrome.app"; grab finder ../System/Library/CoreServices/Finder.app
