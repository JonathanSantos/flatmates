#!/usr/bin/env bash
# Compila o núcleo para WebAssembly e deixa pronto para o editor em web/.
# Depois: python3 -m http.server 8731  e abra http://localhost:8731/web/
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null
cargo build --release --lib --no-default-features --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/flatmates.wasm web/flatmates.wasm
echo "web/flatmates.wasm: $(wc -c < web/flatmates.wasm | tr -d ' ') bytes"
