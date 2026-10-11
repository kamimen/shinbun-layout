#!/bin/sh
# Builds the WebAssembly package into pkg/ (pkg/web for browsers, pkg/node for Node).
# Needs the wasm32 target (rustup target add wasm32-unknown-unknown) and wasm-bindgen-cli at the version in Cargo.lock.
set -eu

want=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/[^0-9.]/, "", $0); print; exit }' Cargo.lock)
have=$(wasm-bindgen --version 2>/dev/null | awk '{ print $2 }' || true)
if [ "$want" != "$have" ]; then
  echo "wasm-bindgen-cli $want is needed (found: ${have:-none}): cargo install wasm-bindgen-cli --version $want --locked" >&2
  exit 1
fi

cargo build -p shinbun-layout-wasm --target wasm32-unknown-unknown --release
wasm=${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/shinbun_layout_wasm.wasm
wasm-bindgen --target web --out-dir pkg/web "$wasm"
wasm-bindgen --target nodejs --out-dir pkg/node "$wasm"
