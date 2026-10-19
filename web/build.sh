#!/usr/bin/env bash
# Build the web version into web/dist: the game as WebAssembly, macroquad's
# JavaScript loader (from the macroquad crate in use) and the page.
#   rustup target add wasm32-unknown-unknown
#   web/build.sh
#   python3 -m http.server -d web/dist   # then open http://localhost:8000
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release --target wasm32-unknown-unknown

out=web/dist
rm -rf "$out"
mkdir -p "$out"
cp target/wasm32-unknown-unknown/release/gravity_engine.wasm "$out/"
macroquad=$(cargo metadata --format-version 1 --locked |
    jq -r '.packages[] | select(.name == "macroquad") | .manifest_path')
cp "$(dirname "$macroquad")/js/mq_js_bundle.js" "$out/"
cp web/index.html web/gravity_web.js "$out/"
echo "Built $out ($(du -sh "$out" | cut -f1))"
