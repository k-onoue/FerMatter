[CmdletBinding()]
param([string]$OutputDirectory)
. "$PSScriptRoot\environment-windows.ps1"
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $DesktopRoot 'build\windows-audit' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$auditorVersion = '0.22.2'
$toolRoot = Join-Path $DesktopRoot 'build\audit-tools'
$archive = Join-Path $toolRoot "cargo-audit-$auditorVersion.zip"
$executable = Join-Path $toolRoot "cargo-audit-x86_64-pc-windows-msvc-v$auditorVersion\cargo-audit.exe"
if (-not (Test-Path -LiteralPath $executable)) {
    New-Item -ItemType Directory -Path $toolRoot -Force | Out-Null
    Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/rustsec/rustsec/releases/download/cargo-audit/v$auditorVersion/cargo-audit-x86_64-pc-windows-msvc-v$auditorVersion.zip" -OutFile $archive
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne '0a7316540862c13d954f648917ceacca593747baed6eec180fafa590be2710ab') {
        throw 'cargo-audit download checksum mismatch.'
    }
    Expand-Archive -LiteralPath $archive -DestinationPath $toolRoot
}
$utf8 = New-Object Text.UTF8Encoding($false)
Push-Location $DesktopRoot
try {
    $npmOutput = & npm.cmd audit --omit=dev --json
    $npmExit = $LASTEXITCODE
    [IO.File]::WriteAllLines((Join-Path $OutputDirectory 'npm.json'), [string[]]$npmOutput, $utf8)
    $cargoOutput = & $executable audit --file src-tauri/Cargo.lock --json
    $cargoExit = $LASTEXITCODE
    [IO.File]::WriteAllLines((Join-Path $OutputDirectory 'rust.json'), [string[]]$cargoOutput, $utf8)
    if ($npmExit -ne 0 -or $cargoExit -ne 0) {
        throw "Dependency audit failed (npm=$npmExit, cargo=$cargoExit). Reports: $OutputDirectory"
    }
    Write-Output "PASS: npm production and Cargo vulnerability audits. Informational warnings remain in $OutputDirectory\rust.json."
} finally {
    Pop-Location
}
