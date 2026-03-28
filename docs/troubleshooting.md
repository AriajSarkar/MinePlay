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
target_aspect_width = 16
target_aspect_height = 9
```

Behavior:

- `auto` removes monitor letterboxing by changing Android logical display size before launch
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

## Keyboard Or Mouse Feels Wrong In Minecraft

Inside Minecraft:

- open `Settings`
- open `Keyboard & Mouse`
- adjust sensitivity
- adjust invert-Y if needed

MinePlay forwards the input path. Gameplay sensitivity stays under Minecraft settings.
