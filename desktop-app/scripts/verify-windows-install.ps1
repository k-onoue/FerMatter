# Exercises the installer with synthetic storage. Refuses to replace an existing installation.
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Installer, [switch]$CheckCompilers)
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
        if ($null -ne $entry -and $null -ne $entry.PSObject.Properties['DisplayName'] -and $entry.DisplayName -eq 'FerMatter') {
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
$compilerReportPath = $null
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
    if ($CheckCompilers) {
        $compilerOutput = Join-Path $qaRoot 'installed-compilers'
        New-Item -ItemType Directory -Path $compilerOutput | Out-Null
        $compilerRootPrefix = [IO.Path]::GetFullPath($compilerOutput).TrimEnd('\') + '\'
        function Assert-CompilerArtifact {
            param([Parameter(Mandatory)][string]$Path)
            if (-not [IO.Path]::IsPathRooted($Path)) { throw 'Compiler evidence must use absolute paths.' }
            $fullPath = [IO.Path]::GetFullPath($Path)
            if ($fullPath.StartsWith('\\?\UNC\', [StringComparison]::OrdinalIgnoreCase)) {
                $fullPath = '\\' + $fullPath.Substring(8)
            } elseif ($fullPath.StartsWith('\\?\', [StringComparison]::OrdinalIgnoreCase)) {
                $fullPath = $fullPath.Substring(4)
            }
            if (-not $fullPath.StartsWith($compilerRootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
                throw 'Compiler evidence leaves the isolated QA directory.'
            }
            $item = Get-Item -LiteralPath $fullPath
            if ($item.PSIsContainer -or $item.Length -le 0) { throw "Missing or empty compiler evidence: $fullPath" }
            return $fullPath
        }
        $previousPath = $env:PATH
        try {
            # Simulate Explorer retaining its pre-install PATH. The app must discover
            # the installed compilers from current registry paths and known folders.
            $env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"
            $verification = Start-Process -FilePath (Join-Path $installDirectory 'FerMatter.exe') -ArgumentList @('--fermatter-verify-instant', ('"' + $compilerOutput + '"')) -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $compilerOutput 'stdout.json') -RedirectStandardError (Join-Path $compilerOutput 'stderr.log')
            if (-not $verification.WaitForExit(420000)) {
                & "$env:SystemRoot\System32\taskkill.exe" /PID $verification.Id /T /F | Out-Null
                throw 'Installed Instant compiler verification timed out.'
            }
            $verification.WaitForExit()
            if ($verification.ExitCode -ne 0) {
                throw "Installed Instant compiler verification failed (code $($verification.ExitCode)). Evidence: $compilerOutput"
            }
        } finally {
            $env:PATH = $previousPath
        }
        $result = Get-Content -LiteralPath (Join-Path $compilerOutput 'stdout.json') -Raw | ConvertFrom-Json
        if ($result.success -ne $true) { throw 'Installed compiler verification did not report success.' }
        $compilerReportPath = Assert-CompilerArtifact $result.reportPath
        $compilerReport = Get-Content -LiteralPath $compilerReportPath -Raw | ConvertFrom-Json
        $expectedCases = @(@{id='latex-align';format='latex'}, @{id='latex-equation';format='latex'}, @{id='typst';format='typst'})
        if ($compilerReport.success -ne $true -or @($compilerReport.cases).Count -ne $expectedCases.Count) {
            throw 'Installed compiler report must contain all three successful Instant cases.'
        }
        foreach ($expected in $expectedCases) {
            $cases = @($compilerReport.cases | Where-Object { $_.id -eq $expected.id -and $_.format -eq $expected.format })
            if ($cases.Count -ne 1 -or $cases[0].status -ne 'ready' -or $cases[0].sourceUnchanged -ne $true) {
                throw "Installed compiler case failed: $($expected.id)"
            }
            foreach ($format in @('svg', 'png', 'pdf')) {
                Assert-CompilerArtifact $cases[0].artifacts.$format | Out-Null
            }
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
        compilerChecks = $(if ($CheckCompilers) { 'passed' } else { 'skipped (not requested)' })
        compilerReport = $compilerReportPath
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
