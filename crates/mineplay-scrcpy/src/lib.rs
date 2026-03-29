use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io::{self, Cursor},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
};

use anyhow::{Context, Result, bail};
use mineplay_android_shell::DisplaySize;
use mineplay_config::AppConfig;
use mineplay_core::{ProjectLayout, ToolStatus};
use serde::Deserialize;
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrcpyLocation {
    pub path: PathBuf,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrcpyLaunchOptions {
    pub serial: String,
    pub max_size: u32,
    pub max_fps: Option<u16>,
    pub bitrate_kbps: u32,
    pub fullscreen: bool,
    pub borderless: bool,
    pub no_audio: bool,
    pub stay_awake: bool,
    pub prefer_hid_keyboard: bool,
    pub prefer_hid_mouse: bool,
    pub start_app: Option<String>,
    pub video_codec: String,
    pub video_encoder: Option<String>,
    pub video_codec_options: Option<String>,
    pub video_buffer_ms: u32,
    pub render_driver: Option<String>,
    pub disable_mipmaps: bool,
    pub disable_clipboard_autosync: bool,
    pub print_fps: bool,
    pub verbosity: String,
    pub turn_screen_off: bool,
    pub window_title: String,
    pub adb_path: Option<PathBuf>,
    pub crop: Option<String>,
    pub new_display: Option<NewDisplaySpec>,
    pub no_vd_system_decorations: bool,
    pub no_vd_destroy_content: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewDisplaySpec {
    pub width: u32,
    pub height: u32,
    pub dpi: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoEncoderInfo {
    pub codec: String,
    pub name: String,
    pub hardware: bool,
    pub vendor: bool,
    pub alias: bool,
}

impl ScrcpyLaunchOptions {
    #[must_use]
    pub fn from_config(serial: String, config: &AppConfig) -> Self {
        let window_title = format!("Mineplay [{}]", serial);

        Self {
            serial,
            max_size: config.scrcpy.max_size.min(config.video.preferred_width),
            max_fps: (config.video.target_fps > 0).then_some(config.video.target_fps),
            bitrate_kbps: config.video.target_bitrate_kbps,
            fullscreen: config.playback.fullscreen,
            borderless: config.playback.borderless,
            no_audio: config.playback.no_audio,
            stay_awake: config.playback.stay_awake,
            prefer_hid_keyboard: config.playback.prefer_hid_keyboard,
            prefer_hid_mouse: config.playback.prefer_hid_mouse,
            start_app: config
                .playback
                .auto_launch_minecraft
                .then(|| config.android.minecraft_package_name.clone()),
            video_codec: config.scrcpy.video_codec.clone(),
            video_encoder: config.scrcpy.video_encoder.clone(),
            video_codec_options: config
                .scrcpy
                .video_codec_options
                .as_ref()
                .and_then(|value| (!value.trim().is_empty()).then(|| value.clone())),
            video_buffer_ms: config.scrcpy.video_buffer_ms,
            render_driver: config.scrcpy.render_driver.clone(),
            disable_mipmaps: config.scrcpy.disable_mipmaps,
            disable_clipboard_autosync: config.scrcpy.disable_clipboard_autosync,
            print_fps: config.diagnostics.enable_scrcpy_fps_counter,
            verbosity: config.scrcpy.verbosity.clone(),
            turn_screen_off: config.scrcpy.turn_screen_off,
            window_title,
            adb_path: None,
            crop: None,
            new_display: None,
            no_vd_system_decorations: config.playback.virtual_display_hide_system_decorations,
            no_vd_destroy_content: config.playback.virtual_display_preserve_content,
        }
    }

    #[must_use]
    pub fn args(&self) -> Vec<OsString> {
        let mut args = vec![
            OsString::from("--serial"),
            OsString::from(&self.serial),
            OsString::from(format!("--video-codec={}", self.video_codec)),
            OsString::from(format!("--max-size={}", self.max_size)),
            OsString::from(format!(
                "--video-bit-rate={}",
                format_bitrate(self.bitrate_kbps)
            )),
            OsString::from(format!("--window-title={}", self.window_title)),
            OsString::from(format!("--video-buffer={}", self.video_buffer_ms)),
        ];

        if let Some(max_fps) = self.max_fps {
            args.push(OsString::from(format!("--max-fps={max_fps}")));
        }

        if !self.verbosity.trim().is_empty() {
            args.push(OsString::from(format!("--verbosity={}", self.verbosity)));
        }

        if self.fullscreen {
            args.push(OsString::from("--fullscreen"));
        }

        if self.borderless {
            args.push(OsString::from("--window-borderless"));
        }

        if self.no_audio {
            args.push(OsString::from("--no-audio"));
        }

        if self.stay_awake {
            args.push(OsString::from("--stay-awake"));
        }

        if self.turn_screen_off {
            args.push(OsString::from("--turn-screen-off"));
        }

        if self.prefer_hid_keyboard {
            args.push(OsString::from("--keyboard=uhid"));
        }

        if self.prefer_hid_mouse {
            args.push(OsString::from("--mouse=uhid"));
        }

        if let Some(video_encoder) = &self.video_encoder {
            args.push(OsString::from(format!("--video-encoder={video_encoder}")));
        }

        if let Some(video_codec_options) = &self.video_codec_options {
            args.push(OsString::from(format!(
                "--video-codec-options={video_codec_options}"
            )));
        }

        if let Some(render_driver) = &self.render_driver {
            args.push(OsString::from(format!("--render-driver={render_driver}")));
        }

        if self.disable_mipmaps {
            args.push(OsString::from("--no-mipmaps"));
        }

        if self.disable_clipboard_autosync {
            args.push(OsString::from("--no-clipboard-autosync"));
        }

        if self.print_fps {
            args.push(OsString::from("--print-fps"));
        }

        if let Some(crop) = &self.crop {
            args.push(OsString::from(format!("--crop={crop}")));
        }

        if let Some(new_display) = self.new_display {
            let mut value = format!("{}x{}", new_display.width, new_display.height);
            if let Some(dpi) = new_display.dpi {
                value.push('/');
                value.push_str(&dpi.to_string());
            }
            args.push(OsString::from(format!("--new-display={value}")));
            if self.no_vd_system_decorations {
                args.push(OsString::from("--no-vd-system-decorations"));
            }
            if self.no_vd_destroy_content {
                args.push(OsString::from("--no-vd-destroy-content"));
            }
        }

        if let Some(package_name) = &self.start_app {
            args.push(OsString::from(format!("--start-app={package_name}")));
        }

        args
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallResult {
    pub version: String,
    pub binary_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum ScrcpyError {
    #[error("unsupported auto-install platform: {0}")]
    UnsupportedPlatform(String),
    #[error("scrcpy release asset not found for {target}")]
    AssetNotFound { target: String },
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub fn locate_scrcpy(layout: &ProjectLayout) -> Option<ScrcpyLocation> {
    candidate_paths(layout)
        .into_iter()
        .find(|(path, _)| path.exists())
        .map(|(path, source)| ScrcpyLocation {
            path,
            source: source.to_string(),
        })
        .or_else(|| locate_in_repo_tools(layout))
}

pub fn scrcpy_status(layout: &ProjectLayout) -> ToolStatus {
    match locate_scrcpy(layout) {
        Some(location) => ToolStatus {
            name: "scrcpy",
            available: true,
            detail: format!("found via {}", location.source),
        },
        None => ToolStatus {
            name: "scrcpy",
            available: false,
            detail: "not found; run `cargo run -p mineplay-desktop -- install-scrcpy` or `play --install-if-missing`".to_string(),
        },
    }
}

pub fn resolve_scrcpy_path(
    layout: &ProjectLayout,
    explicit: Option<&Path>,
    install_if_missing: bool,
) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok(path.to_path_buf());
        }

        bail!("explicit scrcpy path does not exist: {}", path.display());
    }

    if let Some(location) = locate_scrcpy(layout) {
        return Ok(location.path);
    }

    if !install_if_missing {
        bail!("scrcpy not found in repo tools or PATH");
    }

    Ok(install_latest_scrcpy(layout, None)?.binary_path)
}

pub fn install_latest_scrcpy(
    layout: &ProjectLayout,
    version_override: Option<&str>,
) -> Result<InstallResult> {
    if let Some(location) = locate_in_repo_tools(layout) {
        return Ok(InstallResult {
            version: version_override.unwrap_or("existing").to_string(),
            binary_path: location.path,
        });
    }

    let target = download_target()?;
    let release = fetch_release_metadata(version_override)?;
    let asset = select_asset(&release.assets, target)?;
    let tools_dir = layout.root.join("tools");
    let downloads_dir = tools_dir.join("_downloads");
    let scrcpy_dir = tools_dir.join("scrcpy");
    fs::create_dir_all(&downloads_dir)?;
    fs::create_dir_all(&scrcpy_dir)?;

    let archive_path = downloads_dir.join(&asset.name);
    let bytes = download_asset(&asset.browser_download_url)?;
    fs::write(&archive_path, &bytes)
        .with_context(|| format!("failed to write {}", archive_path.display()))?;
    extract_zip(&bytes, &scrcpy_dir)?;

    let binary_path = find_binary_recursive(&scrcpy_dir, scrcpy_binary_name())
        .context("scrcpy archive extracted, but scrcpy executable was not found")?;

    Ok(InstallResult {
        version: release.tag_name,
        binary_path,
    })
}

pub fn launch_scrcpy(binary_path: &Path, options: &ScrcpyLaunchOptions) -> Result<ExitStatus> {
    spawn_scrcpy(binary_path, options, false)?
        .wait()
        .with_context(|| {
            format!(
                "failed while waiting for scrcpy at {}",
                binary_path.display()
            )
        })
}

pub fn spawn_scrcpy(
    binary_path: &Path,
    options: &ScrcpyLaunchOptions,
    capture_output: bool,
) -> Result<Child> {
    let mut command = Command::new(binary_path);
    command.args(options.args());

    if let Some(adb_path) = &options.adb_path {
        prepend_path(&mut command, adb_path.parent());
    }

    if capture_output {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
    }

    command
        .spawn()
        .with_context(|| format!("failed to launch scrcpy at {}", binary_path.display()))
}

pub fn supports_option(binary_path: &Path, option: &str) -> Result<bool> {
    let output = Command::new(binary_path)
        .arg("--help")
        .output()
        .with_context(|| format!("failed to query scrcpy help at {}", binary_path.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains(option))
}

pub fn list_video_encoders(
    binary_path: &Path,
    serial: &str,
    adb_path: Option<&Path>,
) -> Result<Vec<VideoEncoderInfo>> {
    let mut command = Command::new(binary_path);
    command.args(["--serial", serial, "--list-encoders"]);

    if let Some(adb_path) = adb_path {
        prepend_path(&mut command, adb_path.parent());
    }

    let output = command
        .output()
        .with_context(|| format!("failed to list encoders via {}", binary_path.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_video_encoders(&stdout))
}

#[must_use]
pub fn choose_preferred_h264_encoder(encoders: &[VideoEncoderInfo]) -> Option<String> {
    encoders
        .iter()
        .filter(|encoder| encoder.codec.eq_ignore_ascii_case("h264"))
        .max_by_key(|encoder| {
            let mut score = 0_i32;
            if encoder.hardware {
                score += 100;
            }
            if encoder.vendor {
                score += 25;
            }
            if !encoder.alias {
                score += 20;
            }
            if encoder.name.starts_with("c2.") {
                score += 10;
            }
            if !encoder.name.contains(".wfd.") {
                score += 5;
            }
            if encoder.name.contains("google") || encoder.name.contains("android") {
                score -= 50;
            }
            score
        })
        .map(|encoder| encoder.name.clone())
}

fn fetch_release_metadata(version_override: Option<&str>) -> Result<GitHubRelease> {
    let url = match version_override {
        Some(version) => {
            format!("https://api.github.com/repos/Genymobile/scrcpy/releases/tags/{version}")
        }
        None => "https://api.github.com/repos/Genymobile/scrcpy/releases/latest".to_string(),
    };

    let client = reqwest::blocking::Client::builder()
        .build()
        .context("failed to create HTTP client")?;

    let response = client
        .get(url)
        .header("User-Agent", "mineplay")
        .send()
        .context("failed to query scrcpy release metadata")?
        .error_for_status()
        .context("scrcpy release metadata request failed")?;

    response
        .json::<GitHubRelease>()
        .context("failed to parse scrcpy release metadata")
}

fn select_asset<'a>(assets: &'a [GitHubAsset], target: &str) -> Result<&'a GitHubAsset> {
    assets
        .iter()
        .find(|asset| asset.name.contains(target))
        .ok_or_else(|| {
            ScrcpyError::AssetNotFound {
                target: target.to_string(),
            }
            .into()
        })
}

fn download_target() -> Result<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("windows", "x86_64") => Ok("scrcpy-win64-"),
        ("windows", "x86") => Ok("scrcpy-win32-"),
        other => Err(ScrcpyError::UnsupportedPlatform(format!("{}-{}", other.0, other.1)).into()),
    }
}

fn download_asset(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .build()
        .context("failed to create HTTP client")?;

    let response = client
        .get(url)
        .header("User-Agent", "mineplay")
        .send()
        .with_context(|| format!("failed to download {url}"))?
        .error_for_status()
        .with_context(|| format!("download failed for {url}"))?;

    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .context("failed to read scrcpy archive bytes")
}

fn extract_zip(bytes: &[u8], destination: &Path) -> Result<()> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).context("failed to open scrcpy zip archive")?;

    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .context("failed to read scrcpy zip entry")?;
        let Some(rel_path) = file.enclosed_name().map(|path| path.to_path_buf()) else {
            continue;
        };

        let out_path = destination.join(rel_path);
        if file.name().ends_with('/') {
            fs::create_dir_all(&out_path)?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out_file = File::create(&out_path)?;
        io::copy(&mut file, &mut out_file)?;
    }

    Ok(())
}

fn candidate_paths(layout: &ProjectLayout) -> Vec<(PathBuf, &'static str)> {
    let mut candidates = Vec::new();
    let binary_name = scrcpy_binary_name();

    for (key, source) in [("MINEPLAY_SCRCPY", "MINEPLAY_SCRCPY"), ("SCRCPY", "SCRCPY")] {
        if let Some(path) = env::var_os(key) {
            candidates.push((PathBuf::from(path), source));
        }
    }

    candidates.push((
        layout.root.join("tools").join("scrcpy").join(binary_name),
        "repo tools/scrcpy",
    ));
    candidates.extend(path_candidates(binary_name));
    candidates
}

fn locate_in_repo_tools(layout: &ProjectLayout) -> Option<ScrcpyLocation> {
    find_binary_recursive(
        &layout.root.join("tools").join("scrcpy"),
        scrcpy_binary_name(),
    )
    .map(|path| ScrcpyLocation {
        path,
        source: "repo tools/scrcpy".to_string(),
    })
}

fn find_binary_recursive(root: &Path, target_name: &str) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        let entries = fs::read_dir(&path).ok()?;
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }

            if entry_path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(target_name))
            {
                return Some(entry_path);
            }
        }
    }

    None
}

fn path_candidates(binary_name: &str) -> Vec<(PathBuf, &'static str)> {
    env::var_os("PATH")
        .map(|paths| {
            env::split_paths(&paths)
                .map(|path| (path.join(binary_name), "PATH"))
                .collect()
        })
        .unwrap_or_default()
}

fn scrcpy_binary_name() -> &'static str {
    if cfg!(windows) {
        "scrcpy.exe"
    } else {
        "scrcpy"
    }
}

fn prepend_path(command: &mut Command, maybe_dir: Option<&Path>) {
    let Some(dir) = maybe_dir else {
        return;
    };

    let existing = env::var_os("PATH").unwrap_or_default();
    let mut parts = vec![dir.to_path_buf()];
    parts.extend(env::split_paths(&existing));
    if let Ok(path) = env::join_paths(parts) {
        command.env("PATH", path);
    }
}

fn format_bitrate(kbps: u32) -> String {
    if kbps.is_multiple_of(1_000) {
        format!("{}M", kbps / 1_000)
    } else {
        format!("{kbps}K")
    }
}

fn parse_video_encoders(stdout: &str) -> Vec<VideoEncoderInfo> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("--video-codec="))
        .filter_map(|line| {
            let codec = parse_flag_value(line, "--video-codec=")?;
            let name = parse_flag_value(line, "--video-encoder=")?;
            Some(VideoEncoderInfo {
                codec,
                name: name.clone(),
                hardware: line.contains("(hw)"),
                vendor: line.contains("[vendor]"),
                alias: line.contains("(alias for"),
            })
        })
        .collect()
}

fn parse_flag_value(line: &str, prefix: &str) -> Option<String> {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(prefix))
        .map(str::to_string)
}

pub fn compute_crop(
    device_width: u32,
    device_height: u32,
    target_aspect_width: u32,
    target_aspect_height: u32,
) -> Option<String> {
    if device_width == 0
        || device_height == 0
        || target_aspect_width == 0
        || target_aspect_height == 0
    {
        return None;
    }

    let (screen_width, screen_height) = (device_width, device_height);
    let device_is_portrait = screen_width < screen_height;
    let target_is_portrait = target_aspect_width < target_aspect_height;
    let mut target_ratio = target_aspect_width as f64 / target_aspect_height as f64;
    if device_is_portrait != target_is_portrait {
        target_ratio = 1.0 / target_ratio;
    }
    let screen_ratio = screen_width as f64 / screen_height as f64;

    if (screen_ratio - target_ratio).abs() < 0.001 {
        return None;
    }

    let (crop_width, crop_height) = if screen_ratio > target_ratio {
        (
            ((screen_height as f64 * target_ratio).round() as u32).min(screen_width),
            screen_height,
        )
    } else {
        (
            screen_width,
            ((screen_width as f64 / target_ratio).round() as u32).min(screen_height),
        )
    };

    let x = (screen_width - crop_width) / 2;
    let y = (screen_height - crop_height) / 2;
    Some(format!("{crop_width}:{crop_height}:{x}:{y}"))
}

pub fn compute_display_override(
    device_width: u32,
    device_height: u32,
    target_aspect_width: u32,
    target_aspect_height: u32,
) -> Option<DisplaySize> {
    if device_width == 0
        || device_height == 0
        || target_aspect_width == 0
        || target_aspect_height == 0
    {
        return None;
    }

    let device_is_portrait = device_width < device_height;
    let target_is_portrait = target_aspect_width < target_aspect_height;
    let (target_width, target_height) = if device_is_portrait == target_is_portrait {
        (target_aspect_width, target_aspect_height)
    } else {
        (target_aspect_height, target_aspect_width)
    };

    let device_ratio = device_width as f64 / device_height as f64;
    let target_ratio = target_width as f64 / target_height as f64;

    if (device_ratio - target_ratio).abs() < 0.001 {
        return None;
    }

    let (override_width, override_height) = if device_ratio > target_ratio {
        (
            ((device_height as f64 * target_ratio).round() as u32).min(device_width),
            device_height,
        )
    } else {
        (
            device_width,
            ((device_width as f64 / target_ratio).round() as u32).min(device_height),
        )
    };

    if override_width == 0
        || override_height == 0
        || (override_width == device_width && override_height == device_height)
    {
        return None;
    }

    Some(DisplaySize {
        width: override_width,
        height: override_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mineplay_config::AppConfig;

    #[test]
    fn formats_scrcpy_args() {
        let mut options =
            ScrcpyLaunchOptions::from_config("device-1".to_string(), &AppConfig::default());
        options.adb_path = Some(PathBuf::from("C:/platform-tools/adb.exe"));

        let args = options.args();
        assert!(args.iter().any(|arg| arg == "--fullscreen"));
        assert!(args.iter().any(|arg| arg == "--keyboard=uhid"));
        assert!(args.iter().any(|arg| arg == "--mouse=uhid"));
        assert!(args.iter().any(|arg| arg == "--max-fps=60"));
        assert!(args.iter().any(|arg| arg == "--video-buffer=30"));
        assert!(args.iter().any(|arg| arg == "--render-driver=direct3d"));
        assert!(args.iter().any(|arg| arg == "--no-mipmaps"));
        assert!(args.iter().any(|arg| arg == "--no-clipboard-autosync"));
        assert!(args.iter().any(|arg| arg == "--turn-screen-off"));
        assert!(
            args.iter()
                .any(|arg| arg == "--start-app=com.mojang.minecraftpe")
        );
    }

    #[test]
    fn bitrate_uses_megabit_suffix_when_possible() {
        assert_eq!(format_bitrate(20_000), "20M");
        assert_eq!(format_bitrate(12_500), "12500K");
    }

    #[test]
    fn computes_center_crop_for_wide_phone() {
        let crop = compute_crop(1080, 2340, 16, 9).expect("crop");
        assert_eq!(crop, "1080:1920:0:210");
    }

    #[test]
    fn computes_16_by_9_display_override_for_tall_phone() {
        let size = compute_display_override(1080, 2340, 16, 9).expect("override size");
        assert_eq!(
            size,
            DisplaySize {
                width: 1080,
                height: 1920
            }
        );
    }

    #[test]
    fn omits_max_fps_when_uncapped() {
        let mut config = AppConfig::default();
        config.video.target_fps = 0;
        let options = ScrcpyLaunchOptions::from_config("device-1".to_string(), &config);

        let args = options.args();
        assert!(
            !args
                .iter()
                .any(|arg| arg.to_string_lossy().starts_with("--max-fps="))
        );
    }

    #[test]
    fn formats_virtual_display_args() {
        let mut options =
            ScrcpyLaunchOptions::from_config("device-1".to_string(), &AppConfig::default());
        options.new_display = Some(NewDisplaySpec {
            width: 1920,
            height: 1080,
            dpi: Some(420),
        });

        let args = options.args();
        assert!(args.iter().any(|arg| arg == "--new-display=1920x1080/420"));
        assert!(args.iter().any(|arg| arg == "--no-vd-system-decorations"));
        assert!(args.iter().any(|arg| arg == "--no-vd-destroy-content"));
    }

    #[test]
    fn parses_video_encoders_from_scrcpy_output() {
        let encoders = parse_video_encoders(
            "[server] INFO: List of video encoders:\n    --video-codec=h264 --video-encoder=c2.exynos.h264.encoder         (hw) [vendor]\n    --video-codec=h264 --video-encoder=c2.android.avc.encoder         (sw)\n",
        );

        assert_eq!(encoders.len(), 2);
        assert_eq!(encoders[0].codec, "h264");
        assert!(encoders[0].hardware);
        assert!(encoders[0].vendor);
        assert_eq!(encoders[1].name, "c2.android.avc.encoder");
    }

    #[test]
    fn prefers_vendor_hardware_h264_encoder() {
        let encoders = vec![
            VideoEncoderInfo {
                codec: "h264".to_string(),
                name: "c2.android.avc.encoder".to_string(),
                hardware: false,
                vendor: false,
                alias: false,
            },
            VideoEncoderInfo {
                codec: "h264".to_string(),
                name: "c2.exynos.h264.encoder".to_string(),
                hardware: true,
                vendor: true,
                alias: false,
            },
        ];

        let encoder = choose_preferred_h264_encoder(&encoders).expect("encoder");
        assert_eq!(encoder, "c2.exynos.h264.encoder");
    }
}
