#!/bin/sh
set -eu

binary_path=${1:-target/release/cellar-grid-perf}
if [ ! -x "$binary_path" ]; then
  echo "missing release binary: $binary_path" >&2
  exit 2
fi

output=$($binary_path)
echo "$output"
echo "$output" | rg -q 'PERF_RESULT=PASS'
