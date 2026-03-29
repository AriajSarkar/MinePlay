use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CONFIG_FILE_NAME: &str = "mineplay.toml";
pub const PROFILE_FILE_NAME: &str = "bedrock.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub android: AndroidConfig,
    pub network: NetworkConfig,
    pub video: VideoConfig,
    pub fallback: FallbackConfig,
    pub playback: PlaybackConfig,
    pub diagnostics: DiagnosticsConfig,
    pub scrcpy: ScrcpyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            android: AndroidConfig {
                package_name: "dev.mineplay.agent".to_string(),
                minecraft_package_name: "com.mojang.minecraftpe".to_string(),
                min_sdk: 29,
                prefer_wireless_debugging: true,
                paired_device: None,
            },
            network: NetworkConfig {
                transport: "quic".to_string(),
                listen_port: 47651,
                pairing_timeout_seconds: 60,
            },
            video: VideoConfig {
                preferred_width: 1920,
                preferred_height: 1080,
                target_fps: 60,
                target_bitrate_kbps: 20_000,
            },
            fallback: FallbackConfig {
                allow_accessibility_fallback: true,
                allow_shell_mode: true,
            },
            playback: PlaybackConfig::default(),
            diagnostics: DiagnosticsConfig::default(),
            scrcpy: ScrcpyConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AndroidConfig {
    pub package_name: String,
    pub minecraft_package_name: String,
    pub min_sdk: u32,
    pub prefer_wireless_debugging: bool,
    pub paired_device: Option<String>,
}

impl Default for AndroidConfig {
    fn default() -> Self {
        AppConfig::default().android
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct NetworkConfig {
    pub transport: String,
    pub listen_port: u16,
    pub pairing_timeout_seconds: u16,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        AppConfig::default().network
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct VideoConfig {
    pub preferred_width: u32,
    pub preferred_height: u32,
    pub target_fps: u16,
    pub target_bitrate_kbps: u32,
}

impl Default for VideoConfig {
    fn default() -> Self {
        AppConfig::default().video
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FallbackConfig {
    pub allow_accessibility_fallback: bool,
    pub allow_shell_mode: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        AppConfig::default().fallback
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct PlaybackConfig {
    pub backend: String,
    pub preferred_serial: Option<String>,
    pub fullscreen: bool,
    pub borderless: bool,
    pub fill_mode: String,
    pub dynamic_display: bool,
    pub prefer_virtual_display: bool,
    pub virtual_display_dpi: Option<u32>,
    pub virtual_display_hide_system_decorations: bool,
    pub virtual_display_preserve_content: bool,
    pub target_aspect_width: u32,
    pub target_aspect_height: u32,
    pub no_audio: bool,
    pub stay_awake: bool,
    pub prefer_hid_keyboard: bool,
    pub prefer_hid_mouse: bool,
    pub auto_launch_minecraft: bool,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            backend: "scrcpy".to_string(),
            preferred_serial: None,
            fullscreen: true,
            borderless: true,
            fill_mode: "auto".to_string(),
            dynamic_display: true,
            prefer_virtual_display: true,
            virtual_display_dpi: None,
            virtual_display_hide_system_decorations: true,
            virtual_display_preserve_content: true,
            target_aspect_width: 16,
            target_aspect_height: 9,
            no_audio: true,
            stay_awake: true,
            prefer_hid_keyboard: true,
            prefer_hid_mouse: true,
            auto_launch_minecraft: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DiagnosticsConfig {
    pub enable_session_perf_log: bool,
    pub enable_scrcpy_fps_counter: bool,
    pub enable_adb_rtt_probe: bool,
    pub compare_previous_perf: bool,
    pub adb_rtt_probe_interval_ms: u64,
    pub adb_rtt_probe_timeout_ms: u64,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            enable_session_perf_log: false,
            enable_scrcpy_fps_counter: false,
            enable_adb_rtt_probe: false,
            compare_previous_perf: true,
            adb_rtt_probe_interval_ms: 1_000,
            adb_rtt_probe_timeout_ms: 1_500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ScrcpyConfig {
    pub auto_install: bool,
    pub version: Option<String>,
    pub video_codec: String,
    pub video_encoder: Option<String>,
    pub video_codec_options: Option<String>,
    pub max_size: u32,
    pub video_buffer_ms: u32,
    pub render_driver: Option<String>,
    pub disable_mipmaps: bool,
    pub disable_clipboard_autosync: bool,
    pub verbosity: String,
    pub turn_screen_off: bool,
}

impl Default for ScrcpyConfig {
    fn default() -> Self {
        Self {
            auto_install: true,
            version: None,
            video_codec: "h264".to_string(),
            video_encoder: None,
            video_codec_options: None,
            max_size: 1920,
            video_buffer_ms: 30,
            render_driver: if cfg!(windows) {
                Some("direct3d".to_string())
            } else {
                None
            },
            disable_mipmaps: true,
            disable_clipboard_autosync: true,
            verbosity: "info".to_string(),
            turn_screen_off: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BedrockProfile {
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub gui_pointer_scale: f32,
    pub bindings: Vec<KeyBinding>,
}

impl Default for BedrockProfile {
    fn default() -> Self {
        Self {
            look_sensitivity: 1.0,
            invert_y: false,
            gui_pointer_scale: 1.0,
            bindings: vec![
                KeyBinding::new("move_forward", "KeyW"),
                KeyBinding::new("move_left", "KeyA"),
                KeyBinding::new("move_backward", "KeyS"),
                KeyBinding::new("move_right", "KeyD"),
                KeyBinding::new("jump", "Space"),
                KeyBinding::new("sneak", "ShiftLeft"),
                KeyBinding::new("sprint", "ControlLeft"),
                KeyBinding::new("inventory", "KeyE"),
                KeyBinding::new("attack_break", "MouseLeft"),
                KeyBinding::new("use_place", "MouseRight"),
                KeyBinding::new("hotbar_next", "WheelDown"),
                KeyBinding::new("hotbar_prev", "WheelUp"),
                KeyBinding::new("pause_release_cursor", "Escape"),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyBinding {
    pub action: String,
    pub input: String,
}

impl KeyBinding {
    #[must_use]
    pub fn new(action: &str, input: &str) -> Self {
        Self {
            action: action.to_string(),
            input: input.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFiles {
    pub root: PathBuf,
    pub docs_dir: PathBuf,
    pub config_dir: PathBuf,
    pub profiles_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub config_file: PathBuf,
    pub profile_file: PathBuf,
}

impl WorkspaceFiles {
    #[must_use]
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let docs_dir = root.join("docs");
        let config_dir = root.join("config");
        let profiles_dir = root.join("profiles");
        let logs_dir = root.join("logs");

        Self {
            config_file: config_dir.join(CONFIG_FILE_NAME),
            profile_file: profiles_dir.join(PROFILE_FILE_NAME),
            root,
            docs_dir,
            config_dir,
            profiles_dir,
            logs_dir,
        }
    }

    #[must_use]
    pub fn user_data_dirs() -> Option<ProjectDirs> {
        ProjectDirs::from("dev", "mineplay", "mineplay")
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

pub fn ensure_workspace_dirs(files: &WorkspaceFiles) -> Result<(), ConfigError> {
    fs::create_dir_all(&files.docs_dir)?;
    fs::create_dir_all(&files.config_dir)?;
    fs::create_dir_all(&files.profiles_dir)?;
    fs::create_dir_all(&files.logs_dir)?;
    Ok(())
}

pub fn write_default_files(files: &WorkspaceFiles) -> Result<(), ConfigError> {
    ensure_workspace_dirs(files)?;

    if !files.config_file.exists() {
        write_config(&files.config_file, &AppConfig::default())?;
    }

    if !files.profile_file.exists() {
        write_profile(&files.profile_file, &BedrockProfile::default())?;
    }

    Ok(())
}

pub fn write_config(path: &Path, config: &AppConfig) -> Result<(), ConfigError> {
    let text = toml::to_string_pretty(config)?;
    fs::write(path, text)?;
    Ok(())
}

pub fn write_profile(path: &Path, profile: &BedrockProfile) -> Result<(), ConfigError> {
    let text = toml::to_string_pretty(profile)?;
    fs::write(path, text)?;
    Ok(())
}

pub fn load_config(path: &Path) -> Result<AppConfig, ConfigError> {
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

pub fn load_profile(path: &Path) -> Result<BedrockProfile, ConfigError> {
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_include_minecraft_bindings() {
        let profile = BedrockProfile::default();
        assert!(
            profile
                .bindings
                .iter()
                .any(|binding| binding.action == "attack_break" && binding.input == "MouseLeft")
        );
    }
}
