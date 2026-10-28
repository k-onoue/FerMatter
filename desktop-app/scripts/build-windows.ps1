[CmdletBinding()]
param([switch]$DebugBuild, [string]$OutputDirectory, [switch]$SkipPackage)
. "$PSScriptRoot\environment-windows.ps1"
if ($OutputDirectory) { $OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory) }
Push-Location $DesktopRoot
try {
    if (-not (Test-Path -LiteralPath 'node_modules')) {
        Invoke-Checked npm.cmd @('ci')
    }
    $arguments = @('run', 'tauri', 'build', '--', '--bundles', 'nsis')
    if ($DebugBuild) { $arguments += '--debug' }
    Invoke-Checked npm.cmd $arguments
    if (-not $DebugBuild -and -not $SkipPackage) {
        & "$PSScriptRoot\package-windows.ps1" -OutputDirectory $OutputDirectory
    }
} finally {
    Pop-Location
}
