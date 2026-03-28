# Phase 3 - Desktop Client

## Runtime
- Rust workspace.
- `winit` event loop and raw input.
- `wgpu` or platform hardware decode output.
- Separate threads for input capture, network I/O, decode, and render.

## Windowing
- Borderless fullscreen on connect.
- `CursorGrabMode::Locked` and invisible cursor in gameplay mode.
- Toggle to release cursor for menus and settings.
- No window chrome during play.

## Input Capture
- Capture raw `DeviceEvent::MouseMotion` and key press/release on the laptop only.
- Do not use window-relative cursor reads for gameplay.
- Disable mouse acceleration in software.
- Per-game profile maps raw deltas to Minecraft look curve and key bindings.

## Decode / Present
- Hardware decode first.
- Software fallback if unavailable.
- Drop stale frames before decode.
- Present newest complete frame only.
- VSync on; renderer must never block input processing.

## Network
- QUIC streams: video, control, telemetry.
- Track RTT, jitter, packet loss, and encode latency.
- Reconnect states: disconnected, pairing, streaming, recovering.
- Hot reload only if session id and codec config match.

## Minecraft Bindings
- WASD movement.
- Space jump.
- Shift sneak.
- Ctrl sprint.
- Hotbar number row.
- Mouse wheel hotbar scroll.
- LMB attack and break.
- RMB use and place.
- Esc menu and release cursor.
