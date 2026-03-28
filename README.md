# MinePlay

> Play Minecraft Bedrock on your PC monitor from your Android phone without an emulator.

MinePlay is a non-root Android-to-PC play stack for Minecraft Bedrock. It gives you a fullscreen wireless session on your laptop or desktop, uses your PC keyboard and mouse, removes black-bar aspect issues automatically, and restores the phone display when the session ends.

Today, the stable runtime uses a Rust desktop CLI with a `scrcpy` backend for real gameplay. The repository also contains the in-progress native Rust/Android transport scaffold that will replace the `scrcpy` path later.

## What It Does

- Uses wireless ADB to pair and connect to the Android device
- Launches a fullscreen borderless PC window for Minecraft play
- Uses keyboard and mouse forwarding through `scrcpy` UHID mode
- Applies a temporary `16:9` Android display override to avoid black bars
- Restores the original phone display ratio when the session closes
- Recovers stale display overrides from interrupted sessions
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

## How It Works

1. Pair the Android device over Wireless Debugging.
2. Start `mineplay-desktop play`.
3. MinePlay selects the live ADB device, ensures `scrcpy` is installed, applies a temporary display override, starts Minecraft, and opens the mirrored session on the PC.
4. Closing the session restores the original Android display size.

## Command Surface

| Command | Purpose |
| --- | --- |
| `init` | Create default config, profile, and workspace directories |
| `doctor` | Check toolchain, config, Android wrapper, `adb`, and `scrcpy` |
| `devices` | Show visible ADB devices |
| `pair` | Pair with Android Wireless Debugging |
| `connect` | Connect to the wireless debug endpoint |
| `install-scrcpy` | Download `scrcpy` into local repo tools |
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

## Current Status

Working today:

- Rust workspace and CLI
- Wireless ADB pairing and connection flow
- Local `scrcpy` installation and launch
- Fullscreen play path with automatic aspect correction
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

- `playback.fill_mode = "auto"`: default, temporary Android display override for monitor-friendly play
- `playback.fill_mode = "fit"`: keep the full phone frame, allow letterboxing
- `playback.fill_mode = "crop"`: crop after render; available, but not recommended for Minecraft menus

## Platform Notes

- Current scripts target Windows PowerShell.
- Android play path is non-root only.
- Phone and PC must be on the same Wi-Fi/LAN for wireless play.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT License

See `LICENSE-APACHE` and `LICENSE-MIT`.
