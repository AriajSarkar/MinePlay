# mineplay

Non-root Android Bedrock-on-PC streaming stack.

Current scope:
- Rust workspace for protocol, config, desktop bootstrap, ADB shell orchestration, and a usable `scrcpy` play backend
- Android app scaffold for `MediaProjection` and accessibility fallback
- Setup and play-session instructions in [`docs`](docs)

Entry points:
- Desktop CLI: `cargo run -p mineplay-desktop -- --help`
- Start play session now: `cargo run -p mineplay-desktop -- play`
- Android app: [`android/app`](android/app)
- Setup guide: [`docs/setup.md`](docs/setup.md)
- Start-playing guide: [`docs/start-playing.md`](docs/start-playing.md)
