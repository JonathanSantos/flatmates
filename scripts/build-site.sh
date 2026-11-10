#!/usr/bin/env bash
# Monta a página do projeto em _site/: HTML, imagens, exemplos e o motor em
# WebAssembly. O GitHub Pages publica essa pasta.
# Para ver localmente: python3 -m http.server 8732 --directory _site
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null
cargo build --release --locked --lib --no-default-features --target wasm32-unknown-unknown
rm -rf _site
mkdir -p _site/demo _site/img _site/examples
cp site/* _site/
cp demo/* _site/demo/
cp docs/img/*.svg _site/img/
cp examples/*.json _site/examples/
cp target/wasm32-unknown-unknown/release/flatmates.wasm _site/flatmates.wasm
touch _site/.nojekyll
echo "_site pronto: $(du -sh _site | cut -f1)"
