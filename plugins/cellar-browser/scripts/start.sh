#!/bin/sh
set -eu
plugin_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
node_path=${CODEX_MCP_NODE_PATH:-node}
exec "$node_path" "$plugin_dir/scripts/runtime.mjs"
