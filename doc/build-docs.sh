#!/usr/bin/env bash
# Build workspace rustdoc with a root index page.
#
# Usage:
#   ./doc/build-docs.sh          # build docs
#   ./doc/build-docs.sh --open   # build and open in browser
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOC_DIR="$WORKSPACE_ROOT/target/doc"

echo "Building rustdoc for workspace..."
cargo doc --workspace --no-deps

echo "Installing root index page..."
cp "$SCRIPT_DIR/index.html" "$DOC_DIR/index.html"

echo "Documentation ready at: $DOC_DIR/index.html"

if [[ "${1:-}" == "--open" ]]; then
  if command -v xdg-open &>/dev/null; then
    xdg-open "$DOC_DIR/index.html"
  elif command -v open &>/dev/null; then
    open "$DOC_DIR/index.html"
  else
    echo "Open $DOC_DIR/index.html in your browser."
  fi
fi
