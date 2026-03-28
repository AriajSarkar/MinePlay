# Setup

## Goal

Prepare the Windows host, Rust workspace, Android build, and local play backend required to run MinePlay.

## Requirements

- Windows with PowerShell
- Rust `1.91+`
- Java `17`
- Android SDK
- Android phone with Developer Options and Wireless Debugging
- Same Wi-Fi/LAN for phone and PC

## 1. Refresh Tool Environment

If `adb` or Gradle was installed through Chocolatey:

```powershell
Import-Module $env:ChocolateyInstall\helpers\chocolateyProfile.psm1
refreshenv
```

## 2. Initialize Workspace Defaults

```powershell
cargo run -p mineplay-desktop -- init
```

Creates:

- `config/mineplay.toml`
- `profiles/bedrock.toml`
- workspace support directories

## 3. Install Local `scrcpy`

```powershell
cargo run -p mineplay-desktop -- install-scrcpy
```

Downloads `scrcpy` into:

- `tools/scrcpy/`

Use this even if `scrcpy` is not installed globally. MinePlay prefers its local copy.

## 4. Install `adb` If Needed

If `doctor` reports missing `adb`:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/bootstrap-tools.ps1
```

Downloads Android platform-tools into:

- `tools/platform-tools/`

## 5. Verify the Host

```powershell
cargo run -p mineplay-desktop -- doctor
```

Checks:

- config and profile files
- Android Gradle wrapper presence
- docs presence
- `cargo`
- `rustc`
- `java`
- `adb`
- `scrcpy`

## 6. Build the Android Agent

The Android app is scaffolded for the future native transport path. Build it if you want the APK ready:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build-android.ps1
```

APK output:

- `android/app/build/outputs/apk/debug/app-debug.apk`

## 7. Optional Session Dry Run

```powershell
cargo run -p mineplay-desktop -- play --dry-run
```

Shows:

- selected `scrcpy` binary
- selected Android serial
- display mode
- dynamic target resolution
- virtual display or logical override plan
- final launch arguments

## 8. Optional Wireless Probe

```powershell
cargo run -p mineplay-desktop -- perf-probe --seconds 5 --interval-ms 750
```

Use this before tuning so you can separate:

- ADB control-path overhead
- raw Wi-Fi link latency

## Runtime Display Behavior

Default config:

```toml
[playback]
fill_mode = "auto"
dynamic_display = true
prefer_virtual_display = true
```

Behavior:

- `auto`: detect the current PC monitor size, prefer `scrcpy --new-display=<width>x<height>` on supported devices, and fall back to temporary `adb shell wm size` override only if needed
- `fit`: keep full phone frame and allow black bars
- `crop`: crop after render; available, but not the preferred Minecraft mode

Frame-rate behavior:

- `video.target_fps = 60`: current default
- `video.target_fps = 0`: uncapped; omit `--max-fps` and let the device/backend drive as fast as possible

If a session was interrupted, MinePlay restores stale display state on the next `play` launch.

Manual recovery:

```powershell
cargo run -p mineplay-desktop -- reset-display
```

## Primary Files

- `config/mineplay.toml`: runtime configuration
- `profiles/bedrock.toml`: Minecraft binding profile
- `scripts/play.ps1`: PowerShell wrapper for `play`
- `scripts/pair-device.ps1`: PowerShell wrapper for pairing
- `scripts/build-android.ps1`: Android debug APK build
- `scripts/capture-perfetto.ps1`: Android trace capture for deeper analysis
