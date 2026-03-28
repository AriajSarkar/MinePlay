$ErrorActionPreference = "Stop"

$chocoProfile = $null
if ($env:ChocolateyInstall) {
    $chocoProfile = Join-Path $env:ChocolateyInstall "helpers\chocolateyProfile.psm1"
}

if ($chocoProfile -and (Test-Path $chocoProfile)) {
    Import-Module $chocoProfile
    refreshenv
}

$adbCommand = Get-Command adb -ErrorAction SilentlyContinue
if ($adbCommand) {
    Write-Host "adb already available on PATH: $($adbCommand.Source)"
    return
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$toolsDir = Join-Path $root "tools"
$downloadsDir = Join-Path $toolsDir "_downloads"
$platformToolsDir = Join-Path $toolsDir "platform-tools"
$adbPath = Join-Path $platformToolsDir "adb.exe"

New-Item -ItemType Directory -Force $toolsDir, $downloadsDir | Out-Null

if (-not (Test-Path $adbPath)) {
    $zipPath = Join-Path $downloadsDir "platform-tools-latest-windows.zip"
    Invoke-WebRequest -Uri "https://dl.google.com/android/repository/platform-tools-latest-windows.zip" -OutFile $zipPath

    if (Test-Path $platformToolsDir) {
        Remove-Item $platformToolsDir -Recurse -Force
    }

    Expand-Archive -Path $zipPath -DestinationPath $toolsDir -Force
}

Write-Host "adb path: $adbPath"
Write-Host "run 'cargo run -p mineplay-desktop -- doctor' next"
