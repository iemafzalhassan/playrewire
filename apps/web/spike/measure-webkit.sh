#!/usr/bin/env bash
# Task 2 spike: measure the world page in the WebKitGTK MiniBrowser (same engine as
# the Tauri webview on Linux). The page mirrors each 5 s FPS log line to the static
# server as GET /__fps?avg5s=..., so we read the numbers from the server's access log.
# Usage: apps/web/spike/measure-webkit.sh [seconds] [screenshot.png]
# The screenshot is captured in-page (see world.html ?shot=) and saved by server.py.
set -euo pipefail
seconds="${1:-60}"
shot="${2:-}"
port=8766
here="$(cd "$(dirname "$0")/.." && pwd)" # apps/web
log="$(mktemp)"
browser=""

python3 "$here/spike/server.py" "$port" ${shot:+--shot "$shot"} 2>"$log" &
server=$!
trap 'kill $server $browser 2>/dev/null || true; rm -f "$log"' EXIT
sleep 1

# Two extra windows: the first (startup + shader compile) is dropped, one is slack.
/usr/lib/webkit2gtk-4.1/MiniBrowser --full-screen \
  "http://127.0.0.1:$port/spike/world.html${shot:+?shot=20}" >/dev/null 2>&1 &
browser=$!
total=$((seconds + 12))
sleep "$total"
kill "$browser" 2>/dev/null || true

grep -o 'avg5s=[0-9.]*' "$log" | cut -d= -f2 | tail -n +2 | head -n $((seconds / 5)) >"$log.fps"
echo "windows: $(wc -l <"$log.fps")"
paste -sd' ' "$log.fps"
awk '{s+=$1; if(min==""||$1<min)min=$1; if($1>max)max=$1} END{printf "{\"browser\":\"webkitgtk-minibrowser\",\"windows\":%d,\"mean\":%.1f,\"min\":%.1f,\"max\":%.1f}\n", NR, s/NR, min, max}' "$log.fps"
grep -o 'ua=[^ &]*' "$log" | head -1 | python3 -c 'import sys,urllib.parse;print("ua:",urllib.parse.unquote(sys.stdin.read().strip()[3:]))'
rm -f "$log.fps"
