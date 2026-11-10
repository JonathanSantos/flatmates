# Instala a skill do Flatmates no Windows.
#   scripts\install.ps1                 pessoal: %USERPROFILE%\.claude\skills\flatmates
#   scripts\install.ps1 C:\meu-projeto  no projeto: C:\meu-projeto\.claude\skills\flatmates
param([string]$Root = $env:USERPROFILE)
$ErrorActionPreference = "Stop"
$source = Join-Path $PSScriptRoot "..\skill\flatmates"
if (-not (Get-ChildItem (Join-Path $source "bin") -Filter "flatmates-*" -ErrorAction SilentlyContinue)) {
    Write-Error "nenhum binario em skill\flatmates\bin. Baixe flatmates-skill.zip do release ou compile com cargo build --release."
}
$dest = Join-Path $Root ".claude\skills\flatmates"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item -Recurse -Force (Join-Path $source "*") $dest
Write-Output "skill instalada em $dest"
