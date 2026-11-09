#!/bin/sh
# Screenshots of every screen. Serve this folder first: python3 -m http.server 8791 --bind 127.0.0.1
# Usage: ./shoot.sh [name?query#route ...]   e.g. ./shoot.sh "drives-dark?theme=dark#/drives"
cd "$(dirname "$0")" && mkdir -p screens
C="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
set -- ${@:-"overview-light?theme=light#/overview" "overview-dark?theme=dark#/overview" "offload-light?theme=light#/offload" \
  "offload-dark?theme=dark#/offload" "drives-dark?theme=dark#/drives" "drives-light?theme=light#/drives" "memory-light?theme=light#/memory" \
  "settings-dark?theme=dark#/settings" "onboarding-light?theme=light#/onboarding" "popover-dark?theme=dark#/popover" \
  "offload-loading?theme=light&view=loading#/offload" "offload-empty?theme=dark&view=empty#/offload" "overview-error?theme=light&view=error#/overview" \
  "drives-empty?theme=light&view=empty#/drives"}
for spec in "$@"; do
  name=${spec%%\?*}; rest=${spec#*\?}
  "$C" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 --virtual-time-budget=2500 \
    --window-size=1260,860 --screenshot="$PWD/screens/$name.png" "http://127.0.0.1:8791/?$rest" 2>/dev/null
done
