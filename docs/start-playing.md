# Start Playing

## Objective

Launch Minecraft Bedrock from an Android phone onto the PC monitor, drive it with PC keyboard and mouse, and exit without leaving the phone stuck in the wrong aspect ratio.

## Fast Path

If the phone is already paired and visible in `adb devices`:

```powershell
cargo run -p mineplay-desktop -- play
```

PowerShell wrapper:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/play.ps1
```

What `play` does:

1. Selects the active Android device
2. Recovers any stale display override from a previous interrupted session
3. Resolves the local `scrcpy` binary
4. Applies a temporary `16:9` Android display override when `fill_mode = "auto"`
5. Starts Minecraft with fullscreen borderless mirror output
6. Restores the original Android display on exit

## First-Time Wireless Pairing

On the phone:

1. Enable Developer Options
2. Enable Wireless Debugging
3. Open the pairing screen and copy the pairing address and code

On the PC:

```powershell
cargo run -p mineplay-desktop -- pair <host:pairing-port> <pairing-code>
cargo run -p mineplay-desktop -- connect <device-ip:debug-port>
```

Wrapper script:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/pair-device.ps1 -HostPort <host:pairing-port> -Code <pairing-code> -Serial <device-ip:debug-port>
```

Verify:

```powershell
cargo run -p mineplay-desktop -- devices
```

## Minecraft In-Game Checks

Inside Minecraft Bedrock:

- open `Settings`
- open the `Keyboard & Mouse` page
- tune mouse sensitivity and invert-Y to taste
- confirm the game is reacting to keyboard and mouse input from the PC session

MinePlay does not patch Minecraft settings files directly. It provides the transport and display path; in-game sensitivity stays under Minecraft control.

## Fill Modes

Configured in `config/mineplay.toml`:

```toml
[playback]
fill_mode = "auto"
```

Modes:

- `auto`: recommended; changes Android logical display size before launch and restores it on exit
- `fit`: preserves the full phone frame and may show black bars
- `crop`: crops the mirrored frame after render; usable only as a fallback

## If the Phone Ratio Gets Stuck

Run:

```powershell
cargo run -p mineplay-desktop -- reset-display
```

This forces:

- `adb shell wm size reset`

MinePlay also performs stale-override recovery automatically on the next `play` command.

## Stop Playing

- Close the `scrcpy` window
- or stop the `mineplay-desktop play` process cleanly

Expected cleanup:

- `scrcpy` exits
- the temporary Android display override is removed
- the phone returns to its physical display size

## Current Backend Status

Working today:

- wireless ADB orchestration
- local `scrcpy` runtime
- fullscreen mirror window
- UHID keyboard and mouse forwarding
- automatic display override and recovery

Still in scaffold state:

- native Rust video transport
- Android `MediaProjection` streaming pipeline
- native desktop renderer/decoder
- custom input protocol
