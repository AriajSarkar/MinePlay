# Commands

## Desktop CLI

All commands run from the repository root.

Base pattern:

```powershell
cargo run -p mineplay-desktop -- <command>
```

## `init`

Create default runtime files and directories.

```powershell
cargo run -p mineplay-desktop -- init
```

Creates:

- `config/mineplay.toml`
- `profiles/bedrock.toml`

## `doctor`

Check whether the host is ready.

```powershell
cargo run -p mineplay-desktop -- doctor
```

Reports:

- config and profile presence
- Android wrapper presence
- docs presence
- `cargo`, `rustc`, `java`, `adb`, and `scrcpy` availability

## `session-plan`

Show the bootstrap plan derived from the current config.

```powershell
cargo run -p mineplay-desktop -- session-plan
```

## `devices`

List ADB-visible devices.

```powershell
cargo run -p mineplay-desktop -- devices
```

## `pair`

Pair with Android Wireless Debugging.

```powershell
cargo run -p mineplay-desktop -- pair <host:pairing-port> <pairing-code>
```

Example:

```powershell
cargo run -p mineplay-desktop -- pair 192.168.1.10:39017 123456
```

## `connect`

Connect to the wireless debugging endpoint after pairing.

```powershell
cargo run -p mineplay-desktop -- connect <device-ip:debug-port>
```

Example:

```powershell
cargo run -p mineplay-desktop -- connect 192.168.1.10:37111
```

## `install-agent`

Install the Android debug APK.

```powershell
cargo run -p mineplay-desktop -- install-agent <serial> android/app/build/outputs/apk/debug/app-debug.apk
```

Purpose:

- preload the Android scaffold app for future native transport work

## `install-scrcpy`

Download local `scrcpy` into repo tools.

```powershell
cargo run -p mineplay-desktop -- install-scrcpy
```

Optional version pin:

```powershell
cargo run -p mineplay-desktop -- install-scrcpy --version v3.3.4
```

## `play`

Launch the playable fullscreen Minecraft session.

```powershell
cargo run -p mineplay-desktop -- play
```

Optional serial selection:

```powershell
cargo run -p mineplay-desktop -- play --serial <adb-serial>
```

Dry run:

```powershell
cargo run -p mineplay-desktop -- play --dry-run
```

Enable perf log capture:

```powershell
cargo run -p mineplay-desktop -- play --perf-log
```

What it does:

- selects the device
- resolves `adb` and `scrcpy`
- detects the current monitor resolution
- prefers a monitor-sized virtual display when supported
- falls back to logical display override when required
- auto-selects a preferred H.264 encoder when available
- launches Minecraft in the mirror session
- restores display state on exit

With `--perf-log`, it also:

- enables `scrcpy` FPS output
- writes session JSONL logs under `logs/perf/`
- samples ADB control RTT during the session

## `perf-probe`

Measure the wireless path before tuning.

```powershell
cargo run -p mineplay-desktop -- perf-probe --seconds 15 --interval-ms 1000
```

Reports:

- ADB control round-trip time
- raw Wi-Fi ping time to the device IP
- deltas against the previous probe when a previous log exists
- JSONL log output path

Config note:

- `video.target_fps = 0` means uncapped and omits `--max-fps`

## `reset-display`

Force-reset Android display size if a session ended badly.

```powershell
cargo run -p mineplay-desktop -- reset-display
```

Optional serial:

```powershell
cargo run -p mineplay-desktop -- reset-display --serial <adb-serial>
```

## PowerShell Wrappers

### `scripts/play.ps1`

Wrapper around `play`.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/play.ps1
```

Dry run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/play.ps1 -DryRun
```

### `scripts/pair-device.ps1`

Wrapper around `pair` and optional `connect`.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/pair-device.ps1 -HostPort <host:pairing-port> -Code <pairing-code> -Serial <device-ip:debug-port>
```

### `scripts/build-android.ps1`

Build the Android debug APK.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build-android.ps1
```

### `scripts/bootstrap-tools.ps1`

Download local Android platform-tools if `adb` is missing.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/bootstrap-tools.ps1
```

### `scripts/capture-perfetto.ps1`

Capture an Android Perfetto trace over ADB for deeper frame/input analysis.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-perfetto.ps1 -Seconds 15
```
