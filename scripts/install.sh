#!/usr/bin/env bash
# Instala a skill do Flatmates.
#   scripts/install.sh               pessoal: ~/.claude/skills/flatmates
#   scripts/install.sh ~/meu-projeto no projeto: ~/meu-projeto/.claude/skills/flatmates
set -euo pipefail
cd "$(dirname "$0")/.."
dest="${1:-$HOME}/.claude/skills/flatmates"
if ! ls skill/flatmates/bin/flatmates-* >/dev/null 2>&1; then
  echo "erro: nenhum binário em skill/flatmates/bin. Rode scripts/build.sh antes." >&2
  exit 1
fi
mkdir -p "$dest"
cp -R skill/flatmates/. "$dest/"
chmod +x "$dest/scripts/flatmates" "$dest"/bin/flatmates-* 2>/dev/null || true
echo "skill instalada em $dest"
