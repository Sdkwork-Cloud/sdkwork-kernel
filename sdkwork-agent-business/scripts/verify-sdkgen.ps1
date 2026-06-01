param(
    [ValidateSet("DryRun", "Apply")]
    [string]$Mode = "DryRun",
    [switch]$SkipBuild,
    [switch]$CleanTmp
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$moduleRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$repoRoot = (Resolve-Path (Join-Path $moduleRoot "..\..\..\..")).Path
$sdkgen = Join-Path $repoRoot "sdk\sdkwork-sdk-generator\bin\sdkgen.js"

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
        return
    }

    Push-Location $definition.output
    try {
        & node ".\bin\publish-core.mjs" "--language" "typescript" "--project-dir" "." "--action" "check"
        if ($LASTEXITCODE -ne 0) {
            throw "publish check failed for $($definition.name)"
        }

        & node ".\bin\publish-core.mjs" "--language" "typescript" "--project-dir" "." "--action" "build"
        if ($LASTEXITCODE -ne 0) {
            throw "publish build failed for $($definition.name)"
        }
    }
    finally {
        Pop-Location
    }
}

$plans = @()
foreach ($definition in $definitions) {
    $plan = Invoke-SdkgenDryRunJson -definition $definition
    $plans += [pscustomobject]@{
        definition = $definition
        sdkVersion = $plan.sdk.version
        fingerprint = $plan.changeFingerprint
        hasChanges = $plan.hasChanges
        riskLevel = $plan.executionDecision.riskLevel
    }
}

Write-Host "SDK dry-run summary:"
foreach ($plan in $plans) {
    Write-Host ("- {0}: version={1}, hasChanges={2}, fingerprint={3}, risk={4}" -f `
        $plan.definition.name, $plan.sdkVersion, $plan.hasChanges, $plan.fingerprint, $plan.riskLevel)
}

if ($Mode -eq "Apply") {
    foreach ($plan in $plans) {
        Invoke-SdkgenApply -definition $plan.definition -version $plan.sdkVersion -fingerprint $plan.fingerprint
        Invoke-SdkPackageVerification -definition $plan.definition
    }
}

if ($CleanTmp -and (Test-Path $tmpRoot)) {
    Remove-Item -LiteralPath $tmpRoot -Recurse -Force
}
