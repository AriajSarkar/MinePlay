param(
    [string]$Serial,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$arguments = @("run", "-p", "mineplay-desktop", "--", "play")

if ($Serial) {
    $arguments += "--serial"
    $arguments += $Serial
}

if ($DryRun) {
    $arguments += "--dry-run"
}

Push-Location $root
try {
    cargo @arguments
} finally {
    Pop-Location
}
