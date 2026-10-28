[CmdletBinding()]
param([string]$OutputDirectory, [ValidateSet('prototype', 'preview', 'stable-candidate')][string]$Channel = 'prototype', [string]$Validation)
. "$PSScriptRoot\environment-windows.ps1"
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $DesktopRoot 'build\windows' }
foreach ($file in @((Join-Path $DesktopRoot 'src-tauri\target\release\FerMatter.exe')) + @(Get-ChildItem (Join-Path $DesktopRoot 'src-tauri\target\release\bundle\nsis\*-setup.exe') | ForEach-Object FullName)) {
    if ((Get-AuthenticodeSignature -LiteralPath $file).Status -ne 'NotSigned') { throw "Expected an unsigned Windows payload: $file" }
}
$arguments = @((Join-Path $PSScriptRoot 'package-windows.mjs'), $OutputDirectory, $Channel)
if ($Validation) { $arguments += $Validation }
Invoke-Checked node $arguments
