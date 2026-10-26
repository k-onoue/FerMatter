[CmdletBinding()]
param([string]$OutputDirectory)
. "$PSScriptRoot\environment-windows.ps1"
$arguments = @((Join-Path $PSScriptRoot 'package-windows.mjs'))
if ($OutputDirectory) { $arguments += $OutputDirectory }
Invoke-Checked node $arguments
