[CmdletBinding()]
param([switch]$SkipCompilers)
. "$PSScriptRoot\environment-windows.ps1"
Push-Location $DesktopRoot
try {
    if (-not (Test-Path -LiteralPath 'node_modules')) {
        Invoke-Checked npm.cmd @('ci')
    }
    Invoke-Checked npm.cmd @('run', 'build')
    Invoke-Checked npm.cmd @('test', '--', '--maxWorkers=2')
    $checks = @(Get-ChildItem -LiteralPath scripts -Filter '*.check.mjs' | ForEach-Object FullName)
    Invoke-Checked node (@('--test') + $checks)
    Invoke-Checked cargo @('test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--lib')
    Invoke-Checked cargo @('clippy', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--lib', '--', '-D', 'warnings')
    if ($SkipCompilers -or $env:MATHLIDELUX_COMPILER_TESTS -eq '0') {
        Write-Warning 'SKIPPED: real LaTeX, Typst, TikZ and CeTZ compiler checks. This is not a release pass.'
    } else {
        Invoke-Checked cargo @('run', '--locked', '--manifest-path', 'src-tauri/Cargo.toml', '--example', 'verify', '--', $RepositoryRoot, (Join-Path $DesktopRoot 'build\windows-qa'))
    }
} finally {
    Pop-Location
}
