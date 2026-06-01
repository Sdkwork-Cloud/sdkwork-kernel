param(
    [switch]$SkipCargoTest,
    [switch]$SkipSdkgenDryRun,
    [string]$JsonReportPath = "specs/sdkgen/verification-ci.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$moduleRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$manifestPath = Join-Path $moduleRoot "Cargo.toml"
$sdkgenVerifyScript = Join-Path $moduleRoot "scripts\verify-sdkgen.ps1"

if (-not (Test-Path $manifestPath)) {
    throw "cargo manifest not found: $manifestPath"
}
if (-not (Test-Path $sdkgenVerifyScript)) {
    throw "sdkgen verification script not found: $sdkgenVerifyScript"
}

if (-not $SkipCargoTest) {
    Write-Host "Running cargo test (default features)..."
    & cargo test --manifest-path $manifestPath
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test failed"
    }
}

if (-not $SkipSdkgenDryRun) {
    Write-Host "Running SDK dry-run verification..."
    if ([string]::IsNullOrWhiteSpace($JsonReportPath)) {
        & $sdkgenVerifyScript -Mode DryRun -SkipBuild
    }
    else {
        & $sdkgenVerifyScript -Mode DryRun -SkipBuild -JsonReportPath $JsonReportPath
    }
}

Write-Host "CI verification finished successfully."
