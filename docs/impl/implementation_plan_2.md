# Phase 2 - Android Capture / Non-Root Injector

## Capture
- APK runs a foreground `MediaProjection` service.
- Request `MediaProjection` consent once per session.
- Start capture only after the service is foreground with `FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION`.
- Create exactly one `VirtualDisplay` per `MediaProjection` token.
- Register `MediaProjection.Callback` before `createVirtualDisplay()`.
- On `onStop()`, release `VirtualDisplay`, `Surface`, codec, sockets, and session state immediately.

## Encoder
- `MediaCodec` with `video/avc`, surface input only.
- Low-latency profile, no B-frames, no reordering.
- Set bitrate, frame rate, and I-frame interval from desktop telemetry.
- Prefer 1080p or device-native; downscale only on sustained encode pressure.
- Expose SPS/PPS or codec config in the handshake.

## Shell Injector
- Start a shell-launched helper through wireless ADB.
- Helper runs under shell UID, not root.
- Helper consumes control packets from the desktop and injects them using the shell permission envelope available on supported devices.
- If device or OEM blocks shell injection, only the accessibility-touch fallback is enabled.
- Do not add any root or `uinput` branch.

## Accessibility Fallback
- `AccessibilityService` with `dispatchGesture()` for touch emulation.
- Use overlay regions for virtual stick, camera drag, and buttons.
- This fallback is touch-accurate only, not true raw mouse.

## Runtime
- Foreground service plus wakelock plus keep-screen-on.
- Recreate projection token and encoder after stop, lock, or reconnect.
- Android 14+: never call `createVirtualDisplay()` twice on one token.
