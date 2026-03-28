# AGENTS

## Repo Intent

MinePlay is a non-root Android-to-PC Minecraft Bedrock play stack. The current stable path is a Rust desktop CLI plus `scrcpy`. The Android native transport path exists only as scaffold code.

## Branch Workflow

- Default working branch for active development: `dev`
- Do not commit or push without explicit user approval
- Keep `main` as publish-ready history only

## Required Validation

For code changes:

- `cargo fmt`
- `cargo test --workspace`

For desktop play-path changes:

- `cargo run -p mineplay-desktop -- play --dry-run`
- `cargo run -p mineplay-desktop -- perf-probe --seconds 5 --interval-ms 750`

If a device is available and the change touches launch/runtime behavior:

- run a short live `play` session
- verify Android display override cleanup on exit

## Performance Priorities

Optimize for lower end-to-end latency before maximizing image quality.

Current latency-first defaults:

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

Do not claim true zero-medium native latency over wireless. Wireless ADB plus screen encoding/decoding always adds overhead. The goal is minimum practical latency and stable frame pacing.

## Diagnostics

Use these tools before changing tuning:

- `perf-probe` for ADB control RTT and raw Wi-Fi ping split, plus automatic comparison to the previous probe
- `play --perf-log` for session JSONL logs and `scrcpy` FPS output
- `scripts/capture-perfetto.ps1` for deeper Android trace capture

Write perf artifacts under:

- `logs/perf/`

## Config Rules

- Keep new config fields backward-compatible with existing `mineplay.toml`
- Prefer explicit toggles over hidden behavior changes
- Do not silently enable heavy diagnostics by default if they may affect latency

## Android Rules

- Non-root only
- Do not add root-only or OTG-only paths as the primary experience
- Preserve the `reset-display` recovery path whenever changing display handling

## Docs Rules

If commands or runtime defaults change, update:

- `README.md`
- `docs/setup.md`
- `docs/start-playing.md`
- `docs/commands.md`
- `docs/troubleshooting.md`
