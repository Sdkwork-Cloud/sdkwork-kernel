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
$repoRoot = (Resolve-Path (Join-Path $moduleRoot "..\..\..\..")).Path
$sdkgen = Join-Path $repoRoot "sdk\sdkwork-sdk-generator\bin\sdkgen.js"
$startedAt = Get-Date

if (-not (Test-Path $sdkgen)) {
    throw "sdkgen entrypoint not found: $sdkgen"
}

$tmpRoot = Join-Path $moduleRoot ".tmp"
$definitions = @(
    @{
        name = "app"
        input = Join-Path $moduleRoot "specs\openapi\agent-business-app-openapi-3.1.2.yaml"
        output = Join-Path $tmpRoot "agent-business-app-sdk-typescript"
        sdkName = "sdkwork-agent-business-app-sdk"
        sdkType = "app"
        apiPrefix = "/app/v3/api"
    },
    @{
        name = "backend"
        input = Join-Path $moduleRoot "specs\openapi\agent-business-backend-openapi-3.1.2.yaml"
        output = Join-Path $tmpRoot "agent-business-backend-sdk-typescript"
        sdkName = "sdkwork-agent-business-backend-sdk"
        sdkType = "backend"
        apiPrefix = "/backend/v3/api"
    }
)

function Invoke-SdkgenDryRunJson {
    param([hashtable]$definition)

    $args = @(
        $sdkgen,
        "generate",
        "-i", $definition.input,
        "-o", $definition.output,
        "-n", $definition.sdkName,
        "-t", $definition.sdkType,
        "-l", "typescript",
        "--base-url", "http://localhost:8080",
        "--api-prefix", $definition.apiPrefix,
        "--standard-profile", "sdkwork-v3",
        "--dry-run",
        "--json"
    )

    $raw = & node @args
    if ($LASTEXITCODE -ne 0) {
        throw "sdkgen dry-run failed for $($definition.name)"
    }
    return ($raw | Out-String | ConvertFrom-Json)
}

function Invoke-SdkgenApply {
    param(
        [hashtable]$definition,
        [string]$version,
        [string]$fingerprint
    )

    $args = @(
        $sdkgen,
        "generate",
        "-i", $definition.input,
        "-o", $definition.output,
        "-n", $definition.sdkName,
        "-t", $definition.sdkType,
        "-l", "typescript",
        "--base-url", "http://localhost:8080",
        "--api-prefix", $definition.apiPrefix,
        "--standard-profile", "sdkwork-v3",
        "--fixed-sdk-version", $version,
        "--expected-change-fingerprint", $fingerprint,
        "--license", "MIT"
    )

    & node @args
    if ($LASTEXITCODE -ne 0) {
        throw "sdkgen apply failed for $($definition.name)"
    }
}

function Invoke-SdkPackageVerification {
    param([hashtable]$definition)

    if ($SkipBuild) {
        return [pscustomobject]@{
            check = "skipped"
            build = "skipped"
        }
    }

    Push-Location $definition.output
    try {
        & node ".\bin\publish-core.mjs" "--language" "typescript" "--project-dir" "." "--action" "check" | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "publish check failed for $($definition.name)"
        }

        & node ".\bin\publish-core.mjs" "--language" "typescript" "--project-dir" "." "--action" "build" | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "publish build failed for $($definition.name)"
        }

        return [pscustomobject]@{
            check = "passed"
            build = "passed"
        }
    }
    finally {
        Pop-Location
    }
}

function Resolve-ReportPath {
    param([string]$path)

    if ([string]::IsNullOrWhiteSpace($path)) {
        return $null
    }
    if ([System.IO.Path]::IsPathRooted($path)) {
        return $path
    }
    return Join-Path $moduleRoot $path
}

function Write-JsonReport {
    param(
        [string]$reportPath,
        [object]$reportObject
    )

    if ([string]::IsNullOrWhiteSpace($reportPath)) {
        return
    }

    $reportDirectory = Split-Path -Parent $reportPath
    if (-not [string]::IsNullOrWhiteSpace($reportDirectory) -and -not (Test-Path $reportDirectory)) {
        New-Item -ItemType Directory -Path $reportDirectory -Force | Out-Null
    }

    $reportObject | ConvertTo-Json -Depth 30 | Set-Content -Path $reportPath -Encoding UTF8
    Write-Host ("JSON report written: {0}" -f $reportPath)
}

$report = [ordered]@{
    schemaVersion = 1
    module = "sdkwork-agent-business"
    mode = $Mode
    startedAt = $startedAt.ToString("o")
    skipBuild = [bool]$SkipBuild
    cleanTmp = [bool]$CleanTmp
    sdkgenPath = $sdkgen
    plans = @()
    applyResults = @()
}

$plans = @()
foreach ($definition in $definitions) {
    $dryRunPlan = Invoke-SdkgenDryRunJson -definition $definition
    $plans += [pscustomobject]@{
        definition = $definition
        sdkVersion = $dryRunPlan.sdk.version
        fingerprint = $dryRunPlan.changeFingerprint
        hasChanges = $dryRunPlan.hasChanges
        riskLevel = $dryRunPlan.executionDecision.riskLevel
    }
    $report.plans += [ordered]@{
        name = $definition.name
        input = $definition.input
        output = $definition.output
        sdkName = $definition.sdkName
        sdkType = $definition.sdkType
        apiPrefix = $definition.apiPrefix
        sdkVersion = $dryRunPlan.sdk.version
        fingerprint = $dryRunPlan.changeFingerprint
        hasChanges = $dryRunPlan.hasChanges
        riskLevel = $dryRunPlan.executionDecision.riskLevel
        hasDestructiveChanges = $dryRunPlan.hasDestructiveChanges
        impactAreas = @($dryRunPlan.changeImpact.areas)
    }
}

Write-Host "SDK dry-run summary:"
foreach ($plan in $plans) {
    Write-Host ("- {0}: version={1}, hasChanges={2}, fingerprint={3}, risk={4}" -f `
        $plan.definition.name, $plan.sdkVersion, $plan.hasChanges, $plan.fingerprint, $plan.riskLevel)
}

if ($Mode -eq "Apply") {
    foreach ($plan in $plans) {
        $applyResult = [ordered]@{
            name = $plan.definition.name
            output = $plan.definition.output
            sdkVersion = $plan.sdkVersion
            fingerprint = $plan.fingerprint
            generated = $false
            publishCheck = "not_run"
            publishBuild = "not_run"
        }

        Invoke-SdkgenApply -definition $plan.definition -version $plan.sdkVersion -fingerprint $plan.fingerprint
        $applyResult.generated = $true

        $publish = Invoke-SdkPackageVerification -definition $plan.definition
        $applyResult.publishCheck = $publish.check
        $applyResult.publishBuild = $publish.build
        $report.applyResults += $applyResult
    }
}

if ($CleanTmp -and (Test-Path $tmpRoot)) {
    Remove-Item -LiteralPath $tmpRoot -Recurse -Force
}

$finishedAt = Get-Date
$report.finishedAt = $finishedAt.ToString("o")
$report.durationSeconds = [Math]::Round(($finishedAt - $startedAt).TotalSeconds, 3)
$report.tmpRemoved = [bool]$CleanTmp
$report.tmpExistsAfterRun = Test-Path $tmpRoot

$resolvedReportPath = Resolve-ReportPath -path $JsonReportPath
Write-JsonReport -reportPath $resolvedReportPath -reportObject $report
