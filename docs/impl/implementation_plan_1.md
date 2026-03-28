# Phase 1 - Non-Root Architecture Lock

## Target
- Rust desktop client on Windows, Linux, and macOS.
- Android 11+ wireless debugging only; Android 10 and lower require one USB bootstrap for pairing.
- Non-root only: no root, no `uinput`, no privileged system app, no custom kernel.
- Same-LAN transport only.

## Proven Envelope
- Use scrcpy as the baseline for capability and latency targets.
- Rust-only implementation is required for desktop and all new protocol code.
- Native Minecraft Bedrock keyboard/mouse mode is only valid when the shell-context injector path is available.

## Package Layout
- `crates/mineplay-core`: protocol, config, session state, timing, mappings.
- `crates/mineplay-desktop`: input capture, decode, render, launcher.
- `crates/mineplay-android-agent`: APK for MediaProjection session and lifecycle.
- `crates/mineplay-android-shell`: shell-launched injector/helper, started through wireless ADB.

## Transport
- QUIC (`quinn`) for video, control, and telemetry over LAN.
- One-way video stream.
- Ordered control stream with sequence ids and timestamps.
- Heartbeat + RTT telemetry for bitrate and resend policy.

## Input Model
- Desktop captures raw keyboard and mouse from the laptop only.
- Android receives translated events from the desktop.
- Primary injector: shell-context helper started by `adb shell` / wireless debugging.
- Fallback injector: `AccessibilityService` gesture backend only.
- If shell injection is blocked on a device, native mouse-look mode is unsupported on that device.

## Acceptance Gates
- Pairing completes without root.
- No USB needed after pairing on Android 11+.
- Video reaches fullscreen desktop window.
- Control path is non-root and session-scoped.
- No root-only code paths remain in the design.
