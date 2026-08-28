#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  echo "measure-gpui-startup currently supports macOS only" >&2
  exit 2
fi

binary_path=${1:-target/release/cellar-desktop-gpui}
if [ ! -x "$binary_path" ]; then
  echo "missing release binary: $binary_path" >&2
  exit 2
fi

start_time=$(perl -MTime::HiRes=time -e 'print time')
"$binary_path" >/tmp/cellar-gpui-startup.log 2>&1 &
app_pid=$!
cleanup() {
  if kill -0 "$app_pid" 2>/dev/null; then
    kill "$app_pid"
    wait "$app_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

attempt=0
while ! osascript -e 'tell application "System Events" to exists window 1 of process "cellar-desktop-gpui"' 2>/dev/null | rg -q true; do
  attempt=$((attempt + 1))
  if [ "$attempt" -ge 500 ]; then
    echo "GPUI window did not appear within 10 seconds" >&2
    exit 1
  fi
  sleep 0.02
done

ready_time=$(perl -MTime::HiRes=time -e 'print time')
startup_ms=$(awk -v start="$start_time" -v ready="$ready_time" 'BEGIN { printf "%.0f", (ready-start)*1000 }')
sleep 1
rss_kib=$(ps -o rss= -p "$app_pid" | tr -d ' ')

echo "startup_window_detected_ms=$startup_ms idle_rss_kib=$rss_kib"
test "$startup_ms" -lt 1000
test "$rss_kib" -lt 153600
