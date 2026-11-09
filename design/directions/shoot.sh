#!/bin/sh
# Screenshot each direction in light and dark: ./shoot.sh a-manifest b-patchbay ...
cd "$(dirname "$0")" && mkdir -p screens
C="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
for f in "$@"; do for t in light dark; do
  "$C" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 --virtual-time-budget=4000 \
    --window-size=1312,1420 --screenshot="$PWD/screens/$f-$t.png" "file://$PWD/$f.html?theme=$t" 2>/dev/null
done; done
