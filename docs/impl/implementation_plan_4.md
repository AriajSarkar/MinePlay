# Phase 4 - Minecraft Tuning / QA / Release

## Profile
- Ship a dedicated Bedrock/Pocket profile only.
- Zero acceleration by default.
- Sensitivity curve tunable per device and DPI.
- Distinct profiles for shell-injected native mode and touch fallback.

## Video
- 60 FPS target.
- Prefer native 16:9 or 1080p.
- Bitrate ladder based on RTT and encode queue depth.
- No B-frames, short GOP, low-latency encode.
- Atomic frame replacement only; never present partial frames.

## QA
- Walk, sprint, jump, sneak, mine, place, inventory, chat.
- Fast flick camera at multiple DPI values.
- Pause menu focus loss and restore.
- Wi-Fi drop and reconnect.
- Android rotation, app background and foreground, screen lock.
- 30 minute soak.

## Instrumentation
- Timestamp capture, encode, send, decode, and present.
- Android traces via perfetto and logcat.
- Desktop traces via native tracing and frame timing.
- Compare against the scrcpy baseline.

## Release
- Ship desktop binaries and a signed Android APK.
- Version the protocol and config schema.
- Store the supported-device matrix.
- Explicitly document: non-root only; if shell injection is blocked, native mouse mode is unsupported on that device.
