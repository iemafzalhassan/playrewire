#!/usr/bin/env bash
# Task 4 spike: cold start in the WebKitGTK MiniBrowser. The page reports to the
# static server as GET /__lab?cold_ms=..., read from the server's access log.
# Usage: apps/web/spike/measure-lab-webkit.sh [runs]
set -euo pipefail
runs="${1:-5}"; port=8768
here="$(cd "$(dirname "$0")/.." && pwd)"
log="$(mktemp)"
"$here/spike/build-spike.sh"
python3 "$here/spike/server.py" "$port" 2>"$log" & server=$!
trap 'kill $server 2>/dev/null || true; rm -f "$log"' EXIT
sleep 1
for i in $(seq "$runs"); do
  # fresh process + fresh private data dir each run => cold HTTP/code cache
  d="$(mktemp -d)"
  XDG_CACHE_HOME="$d/c" XDG_DATA_HOME="$d/d" /usr/lib/webkit2gtk-4.1/MiniBrowser "http://127.0.0.1:$port/spike/lab.html" >/dev/null 2>&1 & b=$!
  for _ in $(seq 120); do grep -q "__lab" "$log" && [ "$(grep -c __lab "$log")" -ge "$i" ] && break; sleep 0.5; done
  kill $b 2>/dev/null || true; wait $b 2>/dev/null || true; rm -rf "$d"
done
grep -o '__lab?[^ ]*' "$log" | sed 's/&ua=.*//'
