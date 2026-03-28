# Setup

## Scope
- Current repo state initializes the workspace, config, pair/connect tooling, Android app scaffold, and an immediate playable `scrcpy` backend.
- The custom Rust-native transport and Android capture stack are still under implementation.

## Prerequisites
- Rust 1.91+
- Java 17
- Android SDK with platform `android-35`
- `adb` available on `PATH` or installable through `scripts/bootstrap-tools.ps1`
- Same Wi-Fi/LAN for laptop and phone

## Bootstrap
1. If `adb` or `gradle` was installed through Chocolatey, run `Import-Module $env:ChocolateyInstall\helpers\chocolateyProfile.psm1; refreshenv`.
2. Run `cargo run -p mineplay-desktop -- init`.
3. Run `cargo run -p mineplay-desktop -- install-scrcpy`.
4. If `adb` is still missing, run `powershell -ExecutionPolicy Bypass -File scripts/bootstrap-tools.ps1`.
5. Set `ANDROID_SDK_ROOT` if Android Studio did not already set it.
6. Build the Android APK with `powershell -ExecutionPolicy Bypass -File scripts/build-android.ps1`.

## Verify
- `cargo run -p mineplay-desktop -- doctor`
- `cargo test`
- `cargo run -p mineplay-desktop -- play --dry-run`
- `cd android && .\gradlew.bat assembleDebug`

## Output Paths
- Desktop config: `config/mineplay.toml`
- Minecraft profile: `profiles/bedrock.toml`
- Android APK: `android/app/build/outputs/apk/debug/app-debug.apk`

## Display Behavior
- Default `config/mineplay.toml` uses `playback.fill_mode = "auto"`.
- `auto` applies a temporary `adb shell wm size` override to match the configured target aspect ratio, currently `16:9`.
- The override is restored automatically when `mineplay-desktop play` exits.
- If a previous session was interrupted, `play` will recover the stale override on the next launch.
- Manual recovery command: `cargo run -p mineplay-desktop -- reset-display`
