param(
    [ValidateSet("DryRun", "Apply")]
    [string]$Mode = "DryRun",
    [switch]$SkipBuild,
    [switch]$CleanTmp,
    [string]$JsonReportPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$moduleRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$repoRoot = (Resolve-Path (Join-Path $moduleRoot "..")).Path
$workspaceSdkgen = Join-Path $repoRoot "sdks\workspace-agent-sdkgen.mjs"
$workspaceReport = Join-Path $repoRoot "sdks\.sdkgen-agent-workspace-report.json"

if (-not (Test-Path $workspaceSdkgen)) {
    throw "workspace SDK generator script not found: $workspaceSdkgen"
}

$nodeMode = if ($Mode -eq "Apply") { "apply" } else { "dry-run" }
& node $workspaceSdkgen --mode $nodeMode
if ($LASTEXITCODE -ne 0) {
    throw "agent SDK workspace generation failed"
}

if (-not [string]::IsNullOrWhiteSpace($JsonReportPath)) {
    $resolvedReportPath = if ([System.IO.Path]::IsPathRooted($JsonReportPath)) {
        $JsonReportPath
    }
    else {
        Join-Path $moduleRoot $JsonReportPath
    }

    $reportDirectory = Split-Path -Parent $resolvedReportPath
    if (-not [string]::IsNullOrWhiteSpace($reportDirectory) -and -not (Test-Path $reportDirectory)) {
        New-Item -ItemType Directory -Path $reportDirectory -Force | Out-Null
    }

    Copy-Item -LiteralPath $workspaceReport -Destination $resolvedReportPath -Force
    Write-Host ("JSON report written: {0}" -f $resolvedReportPath)
}

if ($SkipBuild) {
    Write-Host "SkipBuild requested; generated package build verification is not run by this wrapper."
}

if ($CleanTmp) {
    Write-Host "CleanTmp requested; no module-local temporary SDK output is created by this wrapper."
}
