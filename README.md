# MinePlay

> Play Minecraft Bedrock on your PC monitor from your Android phone without an emulator.

MinePlay is a non-root Android-to-PC play stack for Minecraft Bedrock. It gives you a fullscreen wireless session on your laptop or desktop, uses your PC keyboard and mouse, sizes the game to the actual monitor automatically, and keeps the phone display clean when the session ends.

Today, the stable runtime uses a Rust desktop CLI with a `scrcpy` backend for real gameplay. The repository also contains the in-progress native Rust/Android transport scaffold that will replace the `scrcpy` path later.

## What It Does

- Uses wireless ADB to pair and connect to the Android device
- Launches a fullscreen borderless PC window for Minecraft play
- Uses keyboard and mouse forwarding through `scrcpy` UHID mode
- Prefers a monitor-sized Android virtual display on supported devices to avoid black bars
- Falls back to temporary logical display override only when virtual display is not available
- Restores the original phone display ratio when the session closes
- Recovers stale display overrides from interrupted sessions
- Auto-selects the best available H.264 hardware encoder when possible
- Compares new perf probes against the previous recorded probe automatically
- Supports capped or uncapped capture rate with `video.target_fps` (`0` = uncapped)
- Adds optional perf logging and RTT probes for wireless tuning
- Includes an Android agent scaffold for future native streaming and input transport

## Quick Start

```powershell
Import-Module $env:ChocolateyInstall\helpers\chocolateyProfile.psm1
refreshenv
cargo run -p mineplay-desktop -- init
cargo run -p mineplay-desktop -- install-scrcpy
cargo run -p mineplay-desktop -- doctor
cargo run -p mineplay-desktop -- play
```

If the phone was not paired before:

```powershell
cargo run -p mineplay-desktop -- pair <host:pairing-port> <pairing-code>
cargo run -p mineplay-desktop -- connect <device-ip:debug-port>
```

If the phone ever stays in the wrong aspect ratio after an interrupted session:

```powershell
cargo run -p mineplay-desktop -- reset-display
```

If you want live wireless diagnostics before tuning:

```powershell
cargo run -p mineplay-desktop -- perf-probe --seconds 5 --interval-ms 750
```

## How It Works

1. Pair the Android device over Wireless Debugging.
2. Start `mineplay-desktop play`.
3. MinePlay selects the live ADB device, ensures `scrcpy` is installed, detects the PC monitor size, chooses a low-latency launch profile, starts Minecraft on a virtual display when supported, and opens the mirrored session on the PC.
4. Closing the session tears down the play session and preserves the phone's normal display state.

## Command Surface

| Command | Purpose |
| --- | --- |
| `init` | Create default config, profile, and workspace directories |
| `doctor` | Check toolchain, config, Android wrapper, `adb`, and `scrcpy` |
| `devices` | Show visible ADB devices |
| `pair` | Pair with Android Wireless Debugging |
| `connect` | Connect to the wireless debug endpoint |
| `install-scrcpy` | Download `scrcpy` into local repo tools |
| `perf-probe` | Measure ADB control RTT and raw Wi-Fi ping to the device |
| `play` | Launch the playable fullscreen Minecraft session |
| `reset-display` | Restore the phone display if an override got stuck |

Full command reference: [docs/commands.md](docs/commands.md)

## Repo Layout

- `crates/mineplay-desktop`: desktop CLI and session orchestration
- `crates/mineplay-android-shell`: ADB device, pair, connect, install, and display control
- `crates/mineplay-scrcpy`: `scrcpy` discovery, installation, and launch backend
- `crates/mineplay-config`: config and Bedrock profile defaults
- `android/app`: Android agent scaffold
- `docs/impl`: phased implementation plans

Detailed module map: [docs/module-layout.md](docs/module-layout.md)

## Documentation

- [docs/setup.md](docs/setup.md): environment bootstrap and build steps
- [docs/start-playing.md](docs/start-playing.md): pairing, launch flow, and gameplay session rules
- [docs/commands.md](docs/commands.md): CLI and PowerShell wrappers
- [docs/troubleshooting.md](docs/troubleshooting.md): fixes for ADB, aspect ratio, and launch failures
- [docs/performance.md](docs/performance.md): latency limits, tuning strategy, and diagnostics
- [CHANGELOG.md](CHANGELOG.md): release history and notable changes

## Current Status

Working today:

- Rust workspace and CLI
- Wireless ADB pairing and connection flow
- Local `scrcpy` installation and launch
- Fullscreen play path with dynamic monitor-sized virtual display
- Automatic H.264 encoder selection for supported devices
- Automatic perf probe comparison against prior logs
- Manual and automatic display recovery

Still scaffolded, not production-ready:

- Native Rust transport instead of `scrcpy`
- Android `MediaProjection` capture pipeline
- Native desktop renderer and decoder
- Custom low-level input transport

## Configuration

Main files:

- `config/mineplay.toml`
- `profiles/bedrock.toml`

Important runtime setting:

- `playback.fill_mode = "auto"`: default, prefer virtual display sized to the current PC monitor and fall back to logical override if needed
- `playback.fill_mode = "fit"`: keep the full phone frame, allow letterboxing
- `playback.fill_mode = "crop"`: crop after render; available, but not recommended for Minecraft menus
- `video.target_fps = 60`: current default play cap
- `video.target_fps = 0`: uncapped capture if the device and backend can supply more

## Platform Notes

- Current scripts target Windows PowerShell.
- Android play path is non-root only.
- Phone and PC must be on the same Wi-Fi/LAN for wireless play.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT License

See `LICENSE-APACHE` and `LICENSE-MIT`.
