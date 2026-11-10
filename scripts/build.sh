#!/usr/bin/env bash
# Compila o binário para cada plataforma cujo alvo Rust esteja instalado e
# copia para skill/flatmates/bin. Alvos sem linker disponível são pulados com aviso.
set -uo pipefail
cd "$(dirname "$0")/.."
out=skill/flatmates/bin
mkdir -p "$out"
targets=(aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-msvc aarch64-pc-windows-msvc)
installed=$(rustup target list --installed)
built=0
for t in "${targets[@]}"; do
  if ! grep -qx "$t" <<<"$installed"; then
    echo "pulado  $t (rode: rustup target add $t)"
    continue
  fi
  if cargo build --release --target "$t" >/dev/null 2>&1; then
    ext=""; [[ $t == *windows* ]] && ext=".exe"
    cp "target/$t/release/flatmates$ext" "$out/flatmates-$t$ext"
    echo "ok      $t"
    built=$((built + 1))
  else
    echo "falhou  $t (falta o linker dessa plataforma nesta máquina)"
  fi
done
echo "$built binário(s) em $out"
[[ $built -gt 0 ]]
