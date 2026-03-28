$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$androidDir = Join-Path $root "android"
$gradle = Join-Path $androidDir "gradlew.bat"

if (-not (Test-Path $gradle)) {
    throw "gradle wrapper missing at $gradle"
}

Push-Location $androidDir
try {
    & $gradle assembleDebug
} finally {
    Pop-Location
}
