# Troubleshooting

## `adb` Is Not Found

Refresh the PowerShell environment first:

```powershell
Import-Module $env:ChocolateyInstall\helpers\chocolateyProfile.psm1
refreshenv
```

If `adb` is still missing:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/bootstrap-tools.ps1
```

Then verify:

```powershell
cargo run -p mineplay-desktop -- doctor
```

## Phone Does Not Appear In `devices`

Check:

- phone and PC are on the same Wi-Fi/LAN
- Developer Options are enabled
- Wireless Debugging is enabled
- the device was paired correctly

Run:

```powershell
cargo run -p mineplay-desktop -- devices
cargo run -p mineplay-desktop -- pair <host:pairing-port> <pairing-code>
cargo run -p mineplay-desktop -- connect <device-ip:debug-port>
```

## Black Bars On The PC Window

Use the default display mode:

```toml
[playback]
fill_mode = "auto"
dynamic_display = true
prefer_virtual_display = true
```

Behavior:

- `auto` removes monitor letterboxing by sizing the session to the current PC monitor and preferring an Android virtual display first
- `fit` keeps the full phone frame and can show black bars
- `crop` removes bars by cutting pixels and is not recommended for Minecraft menus

## Phone Stays In The Wrong Aspect Ratio After Exit

Run:

```powershell
cargo run -p mineplay-desktop -- reset-display
```

MinePlay also restores stale display state automatically on the next `play` launch.

## `scrcpy` Is Missing

Install the local copy:

```powershell
cargo run -p mineplay-desktop -- install-scrcpy
```

## `play` Fails Before Launch

Run:

```powershell
cargo run -p mineplay-desktop -- doctor
cargo run -p mineplay-desktop -- play --dry-run
```

Use the output to verify:

- selected Android serial
- resolved `scrcpy` path
- display mode
- final launch arguments

## Gameplay Feels Lazy Or Slightly Delayed

Reality first:

- Wireless play cannot be perfectly identical to holding the phone directly
- encode, transport, decode, and display scheduling always add overhead

Do this:

```powershell
cargo run -p mineplay-desktop -- perf-probe --seconds 5 --interval-ms 750
cargo run -p mineplay-desktop -- play --perf-log
```

Check:

- `avg_ping_ms`: raw Wi-Fi latency to the device IP
- `avg_rtt_ms`: ADB control-path round trip
- `delta_avg_rtt_ms`: how the current probe compares to the previous probe
- `logs/perf/*.jsonl`: session and probe logs

If the Wi-Fi ping is low but ADB RTT is high, the bottleneck is not the radio alone; it is the ADB/control stack or session buffering.

If the session only feels lazy every few seconds:

- keep `video.target_fps = 60` first
- run `perf-probe` again and compare `delta_*` fields
- let MinePlay keep the reduced adaptive bitrate when previous probes are worse than baseline

## Keyboard Or Mouse Feels Wrong In Minecraft

Inside Minecraft:

- open `Settings`
- open `Keyboard & Mouse`
- adjust sensitivity
- adjust invert-Y if needed

MinePlay forwards the input path. Gameplay sensitivity stays under Minecraft settings.
