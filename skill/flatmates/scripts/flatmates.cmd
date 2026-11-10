@echo off
rem Escolhe o binario certo para esta maquina (Windows).
setlocal
set "HERE=%~dp0.."
set "T=x86_64-pc-windows-msvc"
if /I "%PROCESSOR_ARCHITECTURE%"=="ARM64" set "T=aarch64-pc-windows-msvc"
set "BIN=%HERE%\bin\flatmates-%T%.exe"
if not exist "%BIN%" (
  echo erro: falta %BIN%; baixe flatmates-skill.zip do release 1>&2
  exit /b 1
)
"%BIN%" %*
