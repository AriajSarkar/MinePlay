# Phase 0 - System Map and Data Sources

## Goal
- Non-root Android Bedrock/Pocket play on a PC monitor.
- Laptop provides keyboard/mouse.
- Android provides screen and touch-target runtime.
- All transport stays on local Wi-Fi/LAN after initial pairing.

## Runtime Topology
- PC app:
  - captures keyboard/mouse from the laptop
  - opens fullscreen borderless output
  - decodes video
  - sends control packets
- Android app:
  - requests `MediaProjection`
  - captures screen through `VirtualDisplay`
  - encodes video
  - receives control packets
  - injects input through shell-context helper or `AccessibilityService` fallback

## Connection Bootstrap
- Source of pairing info:
  - Android `Wireless debugging` screen
  - `adb pair` QR/pairing-code workflow
- Required state:
  - same Wi-Fi/LAN
  - developer options enabled
  - paired workstation entry stored on the device
- After pairing:
  - desktop discovers device endpoint
  - desktop opens QUIC session directly to the phone

## Data Sources
- Screen video:
  - Android `MediaProjection`
  - `MediaCodec` encoder
  - `VirtualDisplay` surface
- Device/codec capability:
  - `Build.VERSION`
  - `DisplayMetrics`
  - `MediaCodecList`
  - `MediaFormat`
- Accessibility fallback capability:
  - `AccessibilityServiceInfo`
  - `dispatchGesture()`
  - `FLAG_REQUEST_FILTER_KEY_EVENTS`
- Desktop input:
  - `winit::event::DeviceEvent::MouseMotion`
  - raw key press/release events
- Session/latency telemetry:
  - QUIC RTT and loss stats
  - encoder queue depth
  - frame timestamps

## Local Storage
- `config/mineplay.toml`:
  - paired device address
  - selected video mode
  - bitrate ladder
  - input profile
  - fallback mode flags
- `profiles/bedrock.toml`:
  - Minecraft keymap
  - mouse sensitivity curve
  - aim scaling
  - fullscreen behavior
- `logs/`:
  - pairing errors
  - encode/decode latency
  - reconnect events

## Execution Rules
- No root path.
- No `uinput` path.
- No cloud relay.
- No stale-frame rendering.
- No control packet replay after reconnect without a fresh session id.

## Build Order
- 1. Pair device over wireless ADB.
- 2. Start Android foreground projection service.
- 3. Start shell helper or accessibility fallback.
- 4. Open QUIC control/video streams.
- 5. Capture laptop keyboard/mouse.
- 6. Decode and present video fullscreen.
- 7. Tune Minecraft profile and latency ladder.

## Proof Sources For Implementation
- Wireless debugging and `adb pair`:
  - Android Developers ADB guide
- Screen capture:
  - Android `MediaProjection` docs
- Accessibility fallback:
  - Android `AccessibilityService` and `AccessibilityServiceInfo` docs
- Desktop raw input / fullscreen:
  - `winit` docs
- Transport:
  - `quinn` docs
- Baseline behavior and latency target:
  - scrcpy README
