param(
    [Parameter(Mandatory = $true)]
    [string]$HostPort,
    [Parameter(Mandatory = $true)]
    [string]$Code,
    [string]$Serial
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")

cargo run -p mineplay-desktop -- --workspace-root $root pair $HostPort $Code

if ($Serial) {
    cargo run -p mineplay-desktop -- --workspace-root $root connect $Serial
}
