# Start Playing

## Current Status
- Pairing and environment setup are implemented.
- A playable fullscreen session path is implemented through `scrcpy`.
- The custom Rust-native transport and Android capture/injection stack are still pending.

## Play Now
1. Enable Developer Options and Wireless Debugging on the phone.
2. Run `cargo run -p mineplay-desktop -- doctor`.
3. Pair with `cargo run -p mineplay-desktop -- pair <host:pairing-port> <pairing-code>` if the device is not already listed.
4. Connect with `cargo run -p mineplay-desktop -- connect <device-ip:debug-port>` if the device is not already listed.
5. Run `cargo run -p mineplay-desktop -- play`.
6. The launcher auto-selects the active ADB device, auto-installs `scrcpy` if needed, launches Minecraft, temporarily applies a `wm size` override for `16:9` fullscreen, and opens a fullscreen borderless mirror window.
7. When the session exits, the launcher restores the phone display size automatically.
8. If a session is interrupted and the phone stays in the wrong ratio, run `cargo run -p mineplay-desktop -- reset-display`.

## Fill Modes
- `fill_mode = "auto"`: temporarily changes Android logical display size to the configured target aspect before launch. This is the default and avoids black bars without clipping the Minecraft UI.
- `fill_mode = "fit"`: keeps the full phone frame and may show black bars on the monitor.
- `fill_mode = "crop"`: crops the mirrored frame after render. Use only as a fallback; Minecraft menus can become unplayable.

## Stale Recovery
- `play` restores any stale display override state left by a previous interrupted Mineplay session before launching a new one.
- `reset-display` force-restores the current device display when no game session is running.

## Optional Android App Path
- Build the Android APK with `powershell -ExecutionPolicy Bypass -File scripts/build-android.ps1`.
- Install it with `cargo run -p mineplay-desktop -- install-agent <device-ip:debug-port> android/app/build/outputs/apk/debug/app-debug.apk`.
- This path is scaffolded for the future custom transport and is not the main gameplay path yet.

## Custom Transport Gap
- QUIC streaming session is not implemented.
- Desktop fullscreen renderer is not implemented.
- Android `MediaProjection` -> encoder -> transport path is not implemented.
- Shell-context injector server is not implemented.
