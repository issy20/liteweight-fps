# Build and run liteweight-fps from source (Windows / PowerShell).
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error @"
Rust toolchain not found. Install Rust stable from https://rustup.rs and the
MSVC Build Tools (Visual Studio "Desktop development with C++"), then re-run.
"@
    exit 1
}

cargo run --release --locked -- @args
exit $LASTEXITCODE
