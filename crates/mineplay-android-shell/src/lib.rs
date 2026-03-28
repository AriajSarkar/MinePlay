use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use mineplay_core::{ProjectLayout, ToolStatus};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbLocation {
    pub path: PathBuf,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSummary {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntry {
    pub serial: String,
    pub state: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplaySize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WmSizeInfo {
    pub physical: DisplaySize,
    pub override_size: Option<DisplaySize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairRequest {
    pub host_port: String,
    pub code: String,
}

impl PairRequest {
    #[must_use]
    pub fn args(&self) -> Vec<OsString> {
        vec![
            OsString::from("pair"),
            OsString::from(&self.host_port),
            OsString::from(&self.code),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectRequest {
    pub serial: String,
}

impl ConnectRequest {
    #[must_use]
    pub fn args(&self) -> Vec<OsString> {
        vec![OsString::from("connect"), OsString::from(&self.serial)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRequest {
    pub serial: String,
    pub apk_path: PathBuf,
}

impl InstallRequest {
    #[must_use]
    pub fn args(&self) -> Vec<OsString> {
        vec![
            OsString::from("-s"),
            OsString::from(&self.serial),
            OsString::from("install"),
            OsString::from("-r"),
            self.apk_path.as_os_str().to_owned(),
        ]
    }
}

#[derive(Debug, Error)]
pub enum AdbError {
    #[error("adb command failed with status {status_code:?}: {stderr}")]
    CommandFailed {
        status_code: Option<i32>,
        stderr: String,
    },
}

#[derive(Debug, Clone)]
pub struct AdbRunner {
    adb_path: PathBuf,
}

impl AdbRunner {
    #[must_use]
    pub fn new(adb_path: PathBuf) -> Self {
        Self { adb_path }
    }

    pub fn pair(&self, request: &PairRequest) -> Result<OutputSummary> {
        self.run_args(&request.args())
    }

    pub fn connect(&self, request: &ConnectRequest) -> Result<OutputSummary> {
        self.run_args(&request.args())
    }

    pub fn devices(&self) -> Result<OutputSummary> {
        self.run_args(&[OsString::from("devices")])
    }

    pub fn connected_devices(&self) -> Result<Vec<DeviceEntry>> {
        let output = self.devices()?;
        Ok(parse_devices_output(&output.stdout))
    }

    pub fn wm_size(&self, serial: &str) -> Result<WmSizeInfo> {
        let output = self.run_args(&[
            OsString::from("-s"),
            OsString::from(serial),
            OsString::from("shell"),
            OsString::from("wm"),
            OsString::from("size"),
        ])?;

        parse_wm_size_output(&output.stdout).context("failed to parse `adb shell wm size` output")
    }

    pub fn physical_size(&self, serial: &str) -> Result<DisplaySize> {
        Ok(self.wm_size(serial)?.physical)
    }

    pub fn set_override_size(&self, serial: &str, size: DisplaySize) -> Result<OutputSummary> {
        self.run_args(&[
            OsString::from("-s"),
            OsString::from(serial),
            OsString::from("shell"),
            OsString::from("wm"),
            OsString::from("size"),
            OsString::from(format!("{}x{}", size.width, size.height)),
        ])
    }

    pub fn reset_override_size(&self, serial: &str) -> Result<OutputSummary> {
        self.run_args(&[
            OsString::from("-s"),
            OsString::from(serial),
            OsString::from("shell"),
            OsString::from("wm"),
            OsString::from("size"),
            OsString::from("reset"),
        ])
    }

    pub fn install(&self, request: &InstallRequest) -> Result<OutputSummary> {
        self.run_args(&request.args())
    }

    fn run_args(&self, args: &[OsString]) -> Result<OutputSummary> {
        let output = Command::new(&self.adb_path)
            .args(args)
            .output()
            .with_context(|| format!("failed to execute adb at {}", self.adb_path.display()))?;

        let summary = OutputSummary {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        };

        if output.status.success() {
            Ok(summary)
        } else {
            Err(AdbError::CommandFailed {
                status_code: summary.status_code,
                stderr: summary.stderr.clone(),
            }
            .into())
        }
    }
}

pub fn locate_adb(layout: &ProjectLayout) -> Option<AdbLocation> {
    candidate_paths(layout)
        .into_iter()
        .find(|(path, _)| path.exists())
        .map(|(path, source)| AdbLocation {
            path,
            source: source.to_string(),
        })
}

pub fn adb_status(layout: &ProjectLayout) -> ToolStatus {
    match locate_adb(layout) {
        Some(location) => ToolStatus {
            name: "adb",
            available: true,
            detail: format!("found via {}", location.source),
        },
        None => ToolStatus {
            name: "adb",
            available: false,
            detail: "not found; run scripts/bootstrap-tools.ps1 or install Android platform-tools"
                .to_string(),
        },
    }
}

pub fn resolve_adb_path(layout: &ProjectLayout, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok(path.to_path_buf());
        }

        bail!("explicit adb path does not exist: {}", path.display());
    }

    locate_adb(layout)
        .map(|location| location.path)
        .context("adb not found in repo tools, SDK directories, or PATH")
}

fn candidate_paths(layout: &ProjectLayout) -> Vec<(PathBuf, &'static str)> {
    let mut candidates = Vec::new();
    let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };

    for (key, source) in [("MINEPLAY_ADB", "MINEPLAY_ADB"), ("ADB", "ADB")] {
        if let Some(path) = env::var_os(key) {
            candidates.push((PathBuf::from(path), source));
        }
    }

    candidates.push((
        layout
            .root
            .join("tools")
            .join("platform-tools")
            .join(adb_name),
        "repo tools/platform-tools",
    ));

    for key in ["ANDROID_SDK_ROOT", "ANDROID_HOME"] {
        if let Some(path) = env::var_os(key) {
            candidates.push((
                PathBuf::from(path).join("platform-tools").join(adb_name),
                key,
            ));
        }
    }

    if cfg!(windows) {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            candidates.push((
                PathBuf::from(local_app_data)
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join(adb_name),
                "LOCALAPPDATA Android SDK",
            ));
        }
    }

    candidates.extend(path_candidates(adb_name));
    candidates
}

fn path_candidates(adb_name: &str) -> Vec<(PathBuf, &'static str)> {
    env::var_os("PATH")
        .map(|paths| {
            env::split_paths(&paths)
                .map(|path| (path.join(adb_name), "PATH"))
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_devices_output(stdout: &str) -> Vec<DeviceEntry> {
    stdout
        .lines()
        .skip(1)
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            let mut parts = trimmed.split_whitespace();
            let serial = parts.next()?;
            let state = parts.next()?;

            Some(DeviceEntry {
                serial: serial.to_string(),
                state: state.to_string(),
            })
        })
        .collect()
}

pub fn parse_physical_size(stdout: &str) -> Option<DisplaySize> {
    parse_wm_size_output(stdout).map(|info| info.physical)
}

pub fn parse_wm_size_output(stdout: &str) -> Option<WmSizeInfo> {
    let mut physical = None;
    let mut override_size = None;

    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some(size) = parse_named_size(line, "Physical size") {
            physical = Some(size);
            continue;
        }

        if let Some(size) = parse_named_size(line, "Override size") {
            override_size = Some(size);
            continue;
        }

        if physical.is_none() {
            physical = parse_raw_size(line);
        }
    }

    Some(WmSizeInfo {
        physical: physical?,
        override_size,
    })
}

fn parse_named_size(line: &str, label: &str) -> Option<DisplaySize> {
    let (name, value) = line.split_once(':')?;
    if name.trim() != label {
        return None;
    }

    parse_raw_size(value.trim())
}

fn parse_raw_size(value: &str) -> Option<DisplaySize> {
    let (width, height) = value.split_once('x')?;
    let width = width.trim().parse().ok()?;
    let height = height.trim().parse().ok()?;
    Some(DisplaySize { width, height })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_request_renders_expected_args() {
        let request = PairRequest {
            host_port: "192.168.1.2:37123".to_string(),
            code: "123456".to_string(),
        };

        let args = request.args();
        assert_eq!(args[0], "pair");
        assert_eq!(args[1], "192.168.1.2:37123");
        assert_eq!(args[2], "123456");
    }

    #[test]
    fn install_request_keeps_serial_and_apk() {
        let request = InstallRequest {
            serial: "192.168.1.2:5555".to_string(),
            apk_path: PathBuf::from("android/app/build/outputs/apk/debug/app-debug.apk"),
        };

        let args = request.args();
        assert_eq!(args[0], "-s");
        assert_eq!(args[1], "192.168.1.2:5555");
        assert_eq!(args[2], "install");
        assert_eq!(args[3], "-r");
    }

    #[test]
    fn parses_adb_device_list() {
        let devices = parse_devices_output(
            "List of devices attached\nadb-RZCX91BCJ4X-tnit0a._adb-tls-connect._tcp\tdevice\n",
        );

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].state, "device");
    }

    #[test]
    fn parses_physical_size_output() {
        let size = parse_physical_size("Physical size: 1080x2340\n").expect("size");
        assert_eq!(size.width, 1080);
        assert_eq!(size.height, 2340);
    }

    #[test]
    fn parses_wm_size_output_with_override() {
        let info = parse_wm_size_output("Physical size: 1080x2340\nOverride size: 1080x1920\n")
            .expect("info");
        assert_eq!(info.physical.width, 1080);
        assert_eq!(info.physical.height, 2340);
        assert_eq!(
            info.override_size,
            Some(DisplaySize {
                width: 1080,
                height: 1920
            })
        );
    }
}
