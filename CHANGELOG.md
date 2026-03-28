# Changelog

All notable changes to MinePlay are documented in this file.

The format follows Keep a Changelog and uses semantic-style release headings.

## [0.1.0] - 2026-03-28

### Added

- Rust workspace with dedicated crates for desktop orchestration, config, ADB control, protocol, shared core logic, and `scrcpy` backend execution
- Android app scaffold for future native non-root streaming and fallback paths
- Wireless ADB pairing, connection, device selection, and APK installation flow
- Local `scrcpy` installation and playable fullscreen launch path
- Automatic Android display override for `16:9` monitor-friendly Minecraft play
- Automatic display recovery and manual `reset-display` command for stale aspect-ratio state
- PowerShell scripts for tool bootstrap, Android build, pairing, and gameplay launch
- Operator documentation, command reference, troubleshooting guide, and phased implementation plans
