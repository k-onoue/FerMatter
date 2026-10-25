# Exercises the installer with synthetic storage. Refuses to replace an existing installation.
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Installer)
. "$PSScriptRoot\environment-windows.ps1"
$Installer = (Resolve-Path -LiteralPath $Installer).Path
$manifestPath = Join-Path (Split-Path -Parent $Installer) 'build-info.json'
if (-not (Test-Path -LiteralPath $manifestPath)) { throw 'Missing build-info.json beside the installer.' }
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$installerHash = (Get-FileHash -LiteralPath $Installer -Algorithm SHA256).Hash.ToLowerInvariant()
if ($manifest.file -ne (Split-Path -Leaf $Installer) -or $manifest.sha256 -ne $installerHash) {
    throw 'The installer does not match its build manifest.'
}
$uninstallRoots = @(
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
)
foreach ($registryRoot in $uninstallRoots) {
    if (-not (Test-Path -LiteralPath $registryRoot)) { continue }
    foreach ($key in Get-ChildItem -LiteralPath $registryRoot) {
        $entry = Get-ItemProperty -LiteralPath $key.PSPath
        if ($entry.PSObject.Properties.Name -contains 'DisplayName' -and $entry.DisplayName -eq 'FerMatter') {
            throw 'An existing FerMatter installation was found. Run installer verification on a disposable Windows account or CI machine.'
        }
    }
}
$qaRoot = Join-Path $DesktopRoot ('build\windows-install-' + [Guid]::NewGuid().ToString('N'))
$installDirectory = Join-Path $qaRoot 'application'
$dataDirectory = Join-Path $qaRoot 'data'
$projectDirectory = Join-Path $qaRoot 'Synthetic project'
New-Item -ItemType Directory -Path $dataDirectory, $projectDirectory -Force | Out-Null
$sentinel = Join-Path $dataDirectory 'preserved-draft.txt'
$source = Join-Path $projectDirectory 'main.typ'
Set-Content -LiteralPath $sentinel -Value 'Synthetic draft: preserve through installation and removal.'
Set-Content -LiteralPath $source -Value '$ x + 1 $'
$sentinelHash = (Get-FileHash -LiteralPath $sentinel -Algorithm SHA256).Hash
$sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
$previousDataDirectory = $env:MATHLIDELUX_DATA_DIR
$app = $null
try {
    $env:MATHLIDELUX_DATA_DIR = $dataDirectory
    for ($round = 1; $round -le 2; $round++) {
        $setup = Start-Process -FilePath $Installer -ArgumentList @('/S', "/D=$installDirectory") -PassThru -Wait -WindowStyle Hidden
        if ($setup.ExitCode -ne 0) { throw "Installer failed with code $($setup.ExitCode)." }
        foreach ($relative in @('FerMatter.exe', 'LICENSE', 'NOTICE', 'licenses\JAVASCRIPT-LICENSES.txt', 'licenses\RUST-LICENSES.txt', 'WINDOWS-PREVIEW-README.txt')) {
            if (-not (Test-Path -LiteralPath (Join-Path $installDirectory $relative) -PathType Leaf)) {
                throw "Missing installed payload: $relative"
            }
        }
        if ((Get-FileHash -LiteralPath (Join-Path $installDirectory 'FerMatter.exe') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.applicationSha256) {
            throw 'The installed executable does not match the tested build.'
        }
        if ((Get-FileHash -LiteralPath $sentinel).Hash -ne $sentinelHash -or (Get-FileHash -LiteralPath $source).Hash -ne $sourceHash) {
            throw 'Installation changed synthetic user data.'
        }
    }
    $app = Start-Process -FilePath (Join-Path $installDirectory 'FerMatter.exe') -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 8
    $app.Refresh()
    if ($app.HasExited) { throw "Installed application exited during startup (code $($app.ExitCode))." }
    & taskkill.exe /PID $app.Id /T /F | Out-Null
    $app.WaitForExit()
    $app = $null
    $uninstaller = Join-Path $installDirectory 'uninstall.exe'
    if (-not (Test-Path -LiteralPath $uninstaller)) { throw 'Missing uninstaller.' }
    $removal = Start-Process -FilePath $uninstaller -ArgumentList '/S' -PassThru -Wait -WindowStyle Hidden
    if ($removal.ExitCode -ne 0) { throw "Uninstaller failed with code $($removal.ExitCode)." }
    for ($attempt = 0; $attempt -lt 20 -and (Test-Path -LiteralPath (Join-Path $installDirectory 'FerMatter.exe')); $attempt++) {
        Start-Sleep -Milliseconds 500
    }
    if (Test-Path -LiteralPath (Join-Path $installDirectory 'FerMatter.exe')) { throw 'Application payload survived uninstall.' }
    if ((Get-FileHash -LiteralPath $sentinel).Hash -ne $sentinelHash -or (Get-FileHash -LiteralPath $source).Hash -ne $sourceHash) {
        throw 'Uninstall changed synthetic user data.'
    }
    $report = [ordered]@{
        installer = Split-Path -Leaf $Installer
        sha256 = $installerHash
        installation = $true
        replacement = $true
        installedStartup = $true
        removal = $true
        userDataPreserved = $true
        uiInteractions = 'Not verified by this smoke check.'
        os = [Environment]::OSVersion.VersionString
    }
    $report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $qaRoot 'verification.json') -Encoding utf8
    Write-Output "PASS: installer, replacement, bundled notices, installed startup, uninstall and synthetic data preservation. Evidence: $qaRoot"
} finally {
    if ($app -and -not $app.HasExited) {
        & taskkill.exe /PID $app.Id /T /F | Out-Null
    }
    $env:MATHLIDELUX_DATA_DIR = $previousDataDirectory
}
