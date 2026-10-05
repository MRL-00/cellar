#!/bin/sh
set -eu
plugin_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$plugin_dir"
export CELLAR_EXTENSION_CONFIG="${CELLAR_EXTENSION_CONFIG:-$plugin_dir/.fixtures/connections.json}"
export CELLAR_EXTENSION_STATE="${CELLAR_EXTENSION_STATE:-$HOME/.cellar/extensions/cellar-browser}"
node_path=${CODEX_MCP_NODE_PATH:-node}
exec "$node_path" "$plugin_dir/dist/mcp.mjs"
