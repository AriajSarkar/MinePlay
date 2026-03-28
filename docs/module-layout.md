# Module Layout

## Rust
- `crates/mineplay-protocol`: wire format, session IDs, control/video envelopes
- `crates/mineplay-config`: config/profile files and defaults
- `crates/mineplay-core`: non-root runtime model and workspace layout
- `crates/mineplay-android-shell`: ADB discovery and pair/connect/install execution
- `crates/mineplay-scrcpy`: `scrcpy` discovery, download, and launch backend
- `crates/mineplay-desktop`: operator CLI

## Android
- `android/app`: installed APK scaffold
- `android/app/src/main/java/dev/mineplay/agent/projection`: projection service entry point
- `android/app/src/main/java/dev/mineplay/agent/accessibility`: accessibility fallback skeleton
- `android/app/src/main/java/dev/mineplay/agent/session`: Android-side session wiring placeholder

## Docs
- `docs/impl`: original implementation phases
- `docs/setup.md`: toolchain and bootstrap
- `docs/start-playing.md`: operator flow and current blockers
