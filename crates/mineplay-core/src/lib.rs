use std::path::{Path, PathBuf};

use mineplay_config::{AppConfig, WorkspaceFiles};
use mineplay_protocol::ControlMode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeMode {
    ShellInjected,
    AccessibilityFallback,
}

impl From<RuntimeMode> for ControlMode {
    fn from(value: RuntimeMode) -> Self {
        match value {
            RuntimeMode::ShellInjected => ControlMode::ShellInjected,
            RuntimeMode::AccessibilityFallback => ControlMode::AccessibilityFallback,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionBootstrapPlan {
    pub game_profile: String,
    pub mode: RuntimeMode,
    pub steps: Vec<String>,
}

impl SessionBootstrapPlan {
    #[must_use]
    pub fn from_config(config: &AppConfig) -> Self {
        let mode = if config.fallback.allow_shell_mode {
            RuntimeMode::ShellInjected
        } else {
            RuntimeMode::AccessibilityFallback
        };

        Self {
            game_profile: "bedrock".to_string(),
            mode,
            steps: vec![
                "Pair Android device through wireless ADB or one-time USB bootstrap.".to_string(),
                "Start the Android foreground projection service.".to_string(),
                "Start the shell helper or accessibility fallback.".to_string(),
                "Open QUIC video and control streams.".to_string(),
                "Capture laptop keyboard and mouse.".to_string(),
                "Decode, render, and present fullscreen.".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLayout {
    pub root: PathBuf,
    pub files: WorkspaceFiles,
}

impl ProjectLayout {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let files = WorkspaceFiles::from_root(root.clone());
        Self { root, files }
    }

    #[must_use]
    pub fn android_dir(&self) -> PathBuf {
        self.root.join("android")
    }

    #[must_use]
    pub fn docs_exist(&self) -> bool {
        self.files.docs_dir.exists()
    }

    #[must_use]
    pub fn has_android_wrapper(&self) -> bool {
        self.android_dir().join("gradlew.bat").exists()
            || self.android_dir().join("gradlew").exists()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: &'static str,
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub config_present: bool,
    pub profile_present: bool,
    pub android_wrapper_present: bool,
    pub docs_present: bool,
    pub tool_statuses: Vec<ToolStatus>,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("missing workspace root: {0}")]
    MissingRoot(PathBuf),
}

pub fn ensure_root_exists(root: &Path) -> Result<(), CoreError> {
    if root.exists() {
        Ok(())
    } else {
        Err(CoreError::MissingRoot(root.to_path_buf()))
    }
}
