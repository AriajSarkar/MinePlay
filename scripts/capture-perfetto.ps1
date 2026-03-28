param(
    [string]$Serial,
    [int]$Seconds = 15,
    [string]$OutputPath
)

$ErrorActionPreference = "Stop"

if (-not $OutputPath) {
    $root = Resolve-Path (Join-Path $PSScriptRoot "..")
    $perfDir = Join-Path $root "logs\perf"
    New-Item -ItemType Directory -Force $perfDir | Out-Null
    $OutputPath = Join-Path $perfDir ("perfetto-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".perfetto-trace")
}

$adbArgs = @()
if ($Serial) {
    $adbArgs += "-s"
    $adbArgs += $Serial
}

$deviceTrace = "/data/misc/perfetto-traces/mineplay-trace.perfetto-trace"
$duration = "${Seconds}s"

& adb @adbArgs shell perfetto -o $deviceTrace -t $duration sched freq idle am wm gfx view input
& adb @adbArgs pull $deviceTrace $OutputPath
& adb @adbArgs shell rm $deviceTrace

Write-Host "perfetto trace: $OutputPath"
