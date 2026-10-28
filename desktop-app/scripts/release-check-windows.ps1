[CmdletBinding()]
param([ValidateSet('preview', 'stable-candidate')][string]$Channel = 'stable-candidate', [string]$OutputDirectory)
. "$PSScriptRoot\environment-windows.ps1"
if ($env:MATHLIDELUX_COMPILER_TESTS -eq '0') { throw 'Release validation cannot skip real compilers.' }
Push-Location $RepositoryRoot
try {
    if (& git status --porcelain) { throw 'Commit the candidate before release validation.' }
    $commit = (& git rev-parse HEAD).Trim()
    $run = Join-Path $DesktopRoot ('build\release-validation\windows-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $run -Force | Out-Null
    $steps = Join-Path $run 'steps.tsv'
    $utf8 = New-Object Text.UTF8Encoding($false)
    [IO.File]::WriteAllText($steps, '', $utf8)
    $bundle = Join-Path $DesktopRoot 'src-tauri\target\release\bundle\nsis'
    $status = 'failed'
    function Step([string]$Name, [scriptblock]$Command) {
        # Windows PowerShell 5 turns redirected native stderr (even npm warnings)
        # into terminating NativeCommandError records under Stop. Capture the
        # host transcript instead; Invoke-Checked still enforces native exit codes.
        Start-Transcript -Path (Join-Path $run "$Name.log") | Out-Null
        try {
            & $Command
            [IO.File]::AppendAllText($steps, "$Name`tpassed`n", $utf8)
        } catch {
            [IO.File]::AppendAllText($steps, "$Name`tfailed`n", $utf8)
            throw
        } finally {
            Stop-Transcript | Out-Null
        }
    }
    try {
        # check-windows also includes Clippy; retain separate evidence for the gate.
        Step 'check' { & "$PSScriptRoot\check-windows.ps1" }
        [IO.File]::AppendAllText($steps, "clippy`tpassed`n", $utf8)
        Step 'audit' { & "$PSScriptRoot\audit-windows.ps1" }
        Step 'build' { & "$PSScriptRoot\build-windows.ps1" -SkipPackage }
        Step 'publication' {
            foreach ($project in @('scripts', 'site')) {
                if (-not (Test-Path "$project/node_modules")) { Invoke-Checked npm.cmd @('--prefix', $project, 'ci') }
            }
            Invoke-Checked node @('scripts/build-guides.mjs')
            Invoke-Checked node @('scripts/check-publication.mjs')
        }
        if ((& git rev-parse HEAD).Trim() -ne $commit -or (& git status --porcelain)) { throw 'Candidate changed during validation.' }
        $status = 'passed'
    } finally {
        $dirty = if (& git status --porcelain) { 'dirty' } else { 'clean' }
        Invoke-Checked node @("$PSScriptRoot/release-report.mjs", 'write', $run, $RepositoryRoot, $bundle, $status, $commit, $dirty)
        Write-Output "Release evidence: $run/report.json"
    }
    & "$PSScriptRoot\package-windows.ps1" -OutputDirectory $OutputDirectory -Channel $Channel -Validation (Join-Path $run 'report.json')
} finally {
    Pop-Location
}
