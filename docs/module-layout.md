# Module Layout

## Rust Workspace

### `crates/mineplay-desktop`

Top-level operator CLI.

Responsibilities:

- workspace bootstrap
- tool checks
- device selection
- play-session orchestration
- display override recovery

### `crates/mineplay-android-shell`

ADB-facing execution layer.

Responsibilities:

- locate `adb`
- pair and connect
- enumerate devices
- install APKs
- read and control Android display size

### `crates/mineplay-scrcpy`

Playable backend for the current runtime.

Responsibilities:

- locate local or global `scrcpy`
- install `scrcpy` release assets into repo tools
- format launch arguments
- compute crop and display override targets
- launch the mirror session

### `crates/mineplay-config`

Configuration and profile model.

Responsibilities:

- parse and write `config/mineplay.toml`
- parse and write `profiles/bedrock.toml`
- hold defaults for Android, video, fallback, and playback settings

### `crates/mineplay-core`

Shared workspace/runtime structures.

Responsibilities:

- project layout
- doctor report model
- session bootstrap plan

### `crates/mineplay-protocol`

Future native transport protocol crate.

Responsibilities:

- control envelope types
- session identifiers
- transport message model

## Android App

### `android/app`

Android agent scaffold for the future native non-`scrcpy` path.

### `android/app/src/main/java/dev/mineplay/agent/projection`

Planned `MediaProjection` capture entry point.

### `android/app/src/main/java/dev/mineplay/agent/accessibility`

Accessibility fallback skeleton.

### `android/app/src/main/java/dev/mineplay/agent/session`

Android-side session coordinator placeholder.

## Config and Runtime Data

- `config/mineplay.toml`: desktop runtime configuration
- `profiles/bedrock.toml`: Minecraft control profile
- `logs/`: transient local runtime state, including stale display recovery bookkeeping

## Scripts

- `scripts/bootstrap-tools.ps1`: install local platform-tools when `adb` is missing
- `scripts/build-android.ps1`: build the Android debug APK
- `scripts/pair-device.ps1`: pair and optionally connect to the phone
- `scripts/play.ps1`: wrapper around `mineplay-desktop play`

## Docs

- `docs/setup.md`: host and toolchain bootstrap
- `docs/start-playing.md`: gameplay launch flow
- `docs/commands.md`: command reference
- `docs/troubleshooting.md`: operational fixes
- `docs/impl/`: original phase-by-phase implementation planning
