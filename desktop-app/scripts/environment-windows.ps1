# Dot-source from Windows build/check scripts. No system/user settings are changed.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$DesktopRoot = Split-Path -Parent $PSScriptRoot
$RepositoryRoot = Split-Path -Parent $DesktopRoot
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Run this script on Windows.'
}
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path -LiteralPath $cargoBin) {
    $env:PATH = "$cargoBin;$env:PATH"
}
$localRustup = Join-Path $DesktopRoot 'build\rustup'
if (Test-Path -LiteralPath (Join-Path $localRustup 'toolchains')) {
    $env:RUSTUP_HOME = $localRustup
}
foreach ($command in @('node', 'npm.cmd', 'cargo')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Missing $command. Install Node.js, Rust MSVC and Visual Studio C++ Build Tools first."
    }
}

function Invoke-Checked {
    param([Parameter(Mandatory)][string]$Command, [string[]]$Arguments = @())
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command exited with code $LASTEXITCODE."
    }
}
