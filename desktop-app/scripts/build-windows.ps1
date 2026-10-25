[CmdletBinding()]
param([switch]$Debug, [string]$OutputDirectory)
. "$PSScriptRoot\environment-windows.ps1"
if ($OutputDirectory) { $OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory) }
Push-Location $DesktopRoot
try {
    if (-not (Test-Path -LiteralPath 'node_modules')) {
        Invoke-Checked npm.cmd @('ci')
    }
    $arguments = @('run', 'tauri', 'build', '--', '--bundles', 'nsis')
    if ($Debug) { $arguments += '--debug' }
    Invoke-Checked npm.cmd $arguments
    if (-not $Debug) {
        & "$PSScriptRoot\package-windows.ps1" -OutputDirectory $OutputDirectory
    }
} finally {
    Pop-Location
}
