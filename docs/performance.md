# Performance

## Goal

Make MinePlay feel as close as possible to native handheld play while staying wireless and non-root.

## Hard Limit

Wireless play cannot be truly identical to direct on-device play.

There is always extra work in the path:

1. Android captures frames
2. Android encodes video
3. ADB over Wi-Fi transports control and video
4. PC decodes and renders

The correct target is:

- reduce queueing
- reduce jitter
- keep input/control path stable
- keep frame pacing predictable

## Current Latency-First Defaults

MinePlay now biases the current `scrcpy` backend toward lower latency:

- `video.target_fps = 60`
- `video.target_bitrate_kbps = 30000`
- `scrcpy.video_codec = "h264"`
- `scrcpy.video_buffer_ms = 0`
- `scrcpy.render_driver = "direct3d"` on Windows
- `scrcpy.disable_mipmaps = true`
- `scrcpy.disable_clipboard_autosync = true`
- `playback.fill_mode = "auto"`
- `playback.dynamic_display = true`
- `playback.prefer_virtual_display = true`

Runtime behavior:

- `target_fps = 0` means uncapped
- MinePlay auto-picks a preferred H.264 hardware encoder when available
- MinePlay compares every new `perf-probe` run against the previous probe automatically
- MinePlay can reduce bitrate automatically when recent probe/session data shows worse jitter

These settings aim to reduce buffering and extra processing without dropping the current non-root wireless path.

## Diagnostics

### Quick Probe

```powershell
cargo run -p mineplay-desktop -- perf-probe --seconds 5 --interval-ms 750
```

This measures:

- `avg_ping_ms`: raw Wi-Fi path to the device IP
- `avg_rtt_ms`: ADB control-path round trip
- `delta_avg_rtt_ms`: change vs. previous probe
- `delta_p95_rtt_ms`: change vs. previous probe tail latency

Interpretation:

- low ping + high ADB RTT: overhead is mostly above raw Wi-Fi
- high ping + high ADB RTT: wireless link quality is a major factor

### Monitored Play Session

```powershell
cargo run -p mineplay-desktop -- play --perf-log
```

Outputs:

- `scrcpy` FPS counter
- session JSONL log file path
- periodic ADB RTT samples during play

Logs are written under:

- `logs/perf/`

### Deep Android Trace

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-perfetto.ps1 -Seconds 15
```

Use this when you need Android-side scheduling, input, WM, and graphics traces.

## What To Tune First

If gameplay still feels delayed:

1. verify the phone stays on strong 5 GHz or Wi-Fi 6 signal
2. run `perf-probe`
3. run `play --perf-log`
4. check whether RTT spikes or FPS drops line up with the sluggish feeling

## Current Measured Path

On the connected test device:

- raw Wi-Fi ping remained single-digit milliseconds
- ADB RTT remained much higher than raw Wi-Fi
- the new runtime path successfully started Minecraft on a `1920x1080` Android virtual display

That means the main remaining limit is not the radio alone. It is still the wireless ADB plus encode/decode/control path.

## Future Native Path

The current backend is still `scrcpy`.

For lower latency later, the repository already keeps room for:

- native Rust transport
- native Android capture/injection pipeline
- deeper frame/input telemetry

That path may outperform the current backend, but it is not the stable gameplay path yet.
