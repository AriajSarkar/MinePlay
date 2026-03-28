mod cli;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command as CliCommand};
use mineplay_android_shell::{
    AdbRunner, ConnectRequest, DeviceEntry, DisplaySize, InstallRequest, PairRequest, adb_status,
    resolve_adb_path,
};
use mineplay_config::{load_config, load_profile, write_default_files};
use mineplay_core::{
    DoctorReport, ProjectLayout, SessionBootstrapPlan, ToolStatus, ensure_root_exists,
};
use mineplay_scrcpy::{
    ScrcpyLaunchOptions, compute_crop, compute_display_override, install_latest_scrcpy,
    launch_scrcpy, resolve_scrcpy_path, scrcpy_status,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use tracing_subscriber::{EnvFilter, fmt};

const DISPLAY_STATE_FILE_NAME: &str = "display-override-state.json";

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayDisplayPlan {
    crop: Option<String>,
    display_override: Option<PendingDisplayOverride>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingDisplayOverride {
    requested: DisplaySize,
    restore: RestoreDisplayOverride,
    should_apply: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestoreDisplayOverride {
    Reset,
    Restore(DisplaySize),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedDisplayState {
    serial: String,
    restore: PersistedRestoreDisplayOverride,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum PersistedRestoreDisplayOverride {
    Reset,
    Restore { width: u32, height: u32 },
}

fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let workspace_root = canonical_or_original(cli.workspace_root);
    ensure_root_exists(&workspace_root)?;
    let layout = ProjectLayout::new(workspace_root);

    match cli.command {
        CliCommand::Init => run_init(&layout),
        CliCommand::Doctor => run_doctor(&layout),
        CliCommand::SessionPlan => run_session_plan(&layout),
        CliCommand::Devices { adb } => {
            let runner = runner_for(&layout, adb.as_deref())?;
            print_output(runner.devices()?);
            Ok(())
        }
        CliCommand::Pair {
            host_port,
            code,
            adb,
        } => {
            let runner = runner_for(&layout, adb.as_deref())?;
            let output = runner.pair(&PairRequest { host_port, code })?;
            print_output(output);
            Ok(())
        }
        CliCommand::Connect { serial, adb } => {
            let runner = runner_for(&layout, adb.as_deref())?;
            let output = runner.connect(&ConnectRequest { serial })?;
            print_output(output);
            Ok(())
        }
        CliCommand::InstallAgent { serial, apk, adb } => {
            let runner = runner_for(&layout, adb.as_deref())?;
            let output = runner.install(&InstallRequest {
                serial,
                apk_path: apk,
            })?;
            print_output(output);
            Ok(())
        }
        CliCommand::InstallScrcpy { version } => run_install_scrcpy(&layout, version.as_deref()),
        CliCommand::ResetDisplay { serial, adb } => run_reset_display(&layout, serial, adb),
        CliCommand::Play {
            serial,
            adb,
            scrcpy,
            install_if_missing,
            dry_run,
        } => run_play(&layout, serial, adb, scrcpy, install_if_missing, dry_run),
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).without_time().try_init();
}

fn canonical_or_original(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn run_init(layout: &ProjectLayout) -> Result<()> {
    write_default_files(&layout.files)?;
    println!(
        "initialized workspace defaults at {}",
        layout.root.display()
    );
    println!("config: {}", layout.files.config_file.display());
    println!("profile: {}", layout.files.profile_file.display());
    Ok(())
}

fn run_doctor(layout: &ProjectLayout) -> Result<()> {
    let report = build_doctor_report(layout);
    print_doctor(&report);
    Ok(())
}

fn run_session_plan(layout: &ProjectLayout) -> Result<()> {
    let config = load_config(&layout.files.config_file)
        .with_context(|| format!("missing config file {}", layout.files.config_file.display()))?;
    let profile = load_profile(&layout.files.profile_file).with_context(|| {
        format!(
            "missing profile file {}",
            layout.files.profile_file.display()
        )
    })?;
    let plan = SessionBootstrapPlan::from_config(&config);

    println!("game_profile={}", plan.game_profile);
    println!("mode={:?}", plan.mode);
    println!("bindings={}", profile.bindings.len());
    println!("config={}", layout.files.config_file.display());
    println!("profile={}", layout.files.profile_file.display());

    for (index, step) in plan.steps.iter().enumerate() {
        println!("step_{}={}", index + 1, step);
    }

    Ok(())
}

fn build_doctor_report(layout: &ProjectLayout) -> DoctorReport {
    DoctorReport {
        config_present: layout.files.config_file.exists(),
        profile_present: layout.files.profile_file.exists(),
        android_wrapper_present: layout.has_android_wrapper(),
        docs_present: layout.docs_exist(),
        tool_statuses: vec![
            command_status("cargo", &["--version"]),
            command_status("rustc", &["--version"]),
            command_status("java", &["-version"]),
            adb_status(layout),
            scrcpy_status(layout),
        ],
    }
}

fn print_doctor(report: &DoctorReport) {
    println!("config_present={}", report.config_present);
    println!("profile_present={}", report.profile_present);
    println!("android_wrapper_present={}", report.android_wrapper_present);
    println!("docs_present={}", report.docs_present);

    for tool in &report.tool_statuses {
        println!(
            "tool={} available={} detail={}",
            tool.name, tool.available, tool.detail
        );
    }
}

fn command_status(program: &'static str, args: &[&str]) -> ToolStatus {
    match Command::new(program).args(args).output() {
        Ok(output) if output.status.success() => ToolStatus {
            name: program,
            available: true,
            detail: first_non_empty_line(&output.stdout, &output.stderr),
        },
        Ok(output) => ToolStatus {
            name: program,
            available: false,
            detail: first_non_empty_line(&output.stdout, &output.stderr),
        },
        Err(error) => ToolStatus {
            name: program,
            available: false,
            detail: error.to_string(),
        },
    }
}

fn first_non_empty_line(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    if let Some(line) = stdout.lines().find(|line| !line.trim().is_empty()) {
        return line.trim().to_string();
    }

    let stderr = String::from_utf8_lossy(stderr);
    stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| "no output".to_string())
}

fn runner_for(layout: &ProjectLayout, explicit_adb: Option<&Path>) -> Result<AdbRunner> {
    Ok(AdbRunner::new(resolve_adb_path(layout, explicit_adb)?))
}

fn run_install_scrcpy(layout: &ProjectLayout, version: Option<&str>) -> Result<()> {
    let result = install_latest_scrcpy(layout, version)?;
    println!("scrcpy_version={}", result.version);
    println!("scrcpy_path={}", result.binary_path.display());
    Ok(())
}

fn run_play(
    layout: &ProjectLayout,
    serial: Option<String>,
    adb: Option<PathBuf>,
    scrcpy: Option<PathBuf>,
    install_if_missing: bool,
    dry_run: bool,
) -> Result<()> {
    let config = load_config(&layout.files.config_file)
        .with_context(|| format!("missing config file {}", layout.files.config_file.display()))?;
    let runner = runner_for(layout, adb.as_deref())?;
    recover_stale_display_state(layout, &runner)?;
    let devices = runner.connected_devices()?;
    let selected_serial = select_device_serial(serial, &config, &devices)?;
    let scrcpy_path = resolve_scrcpy_path(layout, scrcpy.as_deref(), install_if_missing)?;
    let adb_path = resolve_adb_path(layout, adb.as_deref())?;
    let mut options = ScrcpyLaunchOptions::from_config(selected_serial, &config);
    options.adb_path = Some(adb_path);
    let display_plan = build_display_plan(&runner, &options.serial, &config)?;
    options.crop = display_plan.crop.clone();

    println!("scrcpy={}", scrcpy_path.display());
    println!("serial={}", options.serial);
    print_display_plan(config.playback.fill_mode.as_str(), &display_plan);

    let rendered_args: Vec<String> = options
        .args()
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    println!("args={}", rendered_args.join(" "));

    if dry_run {
        return Ok(());
    }

    let active_override = prepare_display_override(
        layout,
        &runner,
        &options.serial,
        display_plan.display_override,
    )?;
    let status = launch_scrcpy(&scrcpy_path, &options);

    if let Some(active_override) = active_override {
        if let Err(error) =
            finalize_display_override(layout, &runner, &options.serial, active_override)
        {
            warn!(
                "failed to restore `wm size` for {}: {error:#}",
                options.serial
            );
        }
    }

    let status = status?;
    println!("exit_status={status}");
    Ok(())
}

fn run_reset_display(
    layout: &ProjectLayout,
    serial: Option<String>,
    adb: Option<PathBuf>,
) -> Result<()> {
    let config = load_config(&layout.files.config_file)
        .with_context(|| format!("missing config file {}", layout.files.config_file.display()))?;
    let runner = runner_for(layout, adb.as_deref())?;

    if recover_stale_display_state(layout, &runner)? {
        println!("display_reset=restored_from_state");
        return Ok(());
    }

    let devices = runner.connected_devices()?;
    let selected_serial = select_device_serial(serial, &config, &devices)?;
    runner
        .reset_override_size(&selected_serial)
        .context("failed to reset `adb shell wm size`")?;
    let size_info = runner.wm_size(&selected_serial)?;
    clear_display_state(layout)?;
    println!("serial={selected_serial}");
    println!(
        "display_reset=physical:{}x{}",
        size_info.physical.width, size_info.physical.height
    );
    println!(
        "display_override={}",
        match size_info.override_size {
            Some(size) => format!("{}x{}", size.width, size.height),
            None => "none".to_string(),
        }
    );
    Ok(())
}

fn build_display_plan(
    runner: &AdbRunner,
    serial: &str,
    config: &mineplay_config::AppConfig,
) -> Result<PlayDisplayPlan> {
    match config.playback.fill_mode.as_str() {
        "fit" => Ok(PlayDisplayPlan {
            crop: None,
            display_override: None,
        }),
        "crop" => {
            let size_info = runner.wm_size(serial)?;
            let size = size_info.override_size.unwrap_or(size_info.physical);
            Ok(PlayDisplayPlan {
                crop: compute_crop(
                    size.width,
                    size.height,
                    config.playback.target_aspect_width,
                    config.playback.target_aspect_height,
                ),
                display_override: None,
            })
        }
        "auto" => build_auto_display_plan(runner, serial, config),
        other => anyhow::bail!("unsupported playback.fill_mode `{other}`"),
    }
}

fn build_auto_display_plan(
    runner: &AdbRunner,
    serial: &str,
    config: &mineplay_config::AppConfig,
) -> Result<PlayDisplayPlan> {
    let size_info = runner.wm_size(serial)?;
    let requested = compute_display_override(
        size_info.physical.width,
        size_info.physical.height,
        config.playback.target_aspect_width,
        config.playback.target_aspect_height,
    );

    let display_override = requested.and_then(|requested| {
        Some(PendingDisplayOverride {
            requested,
            restore: match size_info.override_size {
                Some(previous) if previous == requested => RestoreDisplayOverride::Reset,
                Some(previous) => RestoreDisplayOverride::Restore(previous),
                None => RestoreDisplayOverride::Reset,
            },
            should_apply: size_info.override_size != Some(requested),
        })
    });

    Ok(PlayDisplayPlan {
        crop: None,
        display_override,
    })
}

fn print_display_plan(fill_mode: &str, plan: &PlayDisplayPlan) {
    if let Some(crop) = &plan.crop {
        println!("display_mode=crop");
        println!("display_crop={crop}");
        return;
    }

    if let Some(display_override) = plan.display_override {
        println!("display_mode=auto");
        println!(
            "display_override={}x{}",
            display_override.requested.width, display_override.requested.height
        );
        return;
    }

    println!("display_mode={fill_mode}");
    println!("display_override=none");
}

fn apply_display_override(
    runner: &AdbRunner,
    serial: &str,
    display_override: PendingDisplayOverride,
) -> Result<()> {
    if !display_override.should_apply {
        return Ok(());
    }

    runner
        .set_override_size(serial, display_override.requested)
        .with_context(|| {
            format!(
                "failed to apply `adb shell wm size {}x{}`",
                display_override.requested.width, display_override.requested.height
            )
        })?;

    Ok(())
}

fn prepare_display_override(
    layout: &ProjectLayout,
    runner: &AdbRunner,
    serial: &str,
    display_override: Option<PendingDisplayOverride>,
) -> Result<Option<PendingDisplayOverride>> {
    let Some(display_override) = display_override else {
        clear_display_state(layout)?;
        return Ok(None);
    };

    apply_display_override(runner, serial, display_override)?;
    persist_display_state(layout, serial, display_override.restore)?;

    Ok(Some(display_override))
}

fn finalize_display_override(
    layout: &ProjectLayout,
    runner: &AdbRunner,
    serial: &str,
    display_override: PendingDisplayOverride,
) -> Result<()> {
    restore_display_override(runner, serial, display_override)?;
    clear_display_state(layout)?;
    Ok(())
}

fn restore_display_override(
    runner: &AdbRunner,
    serial: &str,
    display_override: PendingDisplayOverride,
) -> Result<()> {
    match display_override.restore {
        RestoreDisplayOverride::Reset => {
            runner
                .reset_override_size(serial)
                .context("failed to reset `adb shell wm size`")?;
        }
        RestoreDisplayOverride::Restore(previous) => {
            runner
                .set_override_size(serial, previous)
                .with_context(|| {
                    format!(
                        "failed to restore `adb shell wm size {}x{}`",
                        previous.width, previous.height
                    )
                })?;
        }
    }

    Ok(())
}

fn recover_stale_display_state(layout: &ProjectLayout, runner: &AdbRunner) -> Result<bool> {
    let state_path = display_state_path(layout);
    if !state_path.exists() {
        return Ok(false);
    }

    let state = load_display_state(layout)?;
    restore_persisted_display_state(runner, &state).with_context(|| {
        format!(
            "failed to recover stale display override for device {}",
            state.serial
        )
    })?;
    clear_display_state(layout)?;
    println!("display_cleanup=restored_stale_override");
    println!("serial={}", state.serial);
    Ok(true)
}

fn restore_persisted_display_state(
    runner: &AdbRunner,
    state: &PersistedDisplayState,
) -> Result<()> {
    match persisted_restore_to_runtime(&state.restore) {
        RestoreDisplayOverride::Reset => {
            runner
                .reset_override_size(&state.serial)
                .context("failed to reset stale `adb shell wm size`")?;
        }
        RestoreDisplayOverride::Restore(previous) => {
            runner
                .set_override_size(&state.serial, previous)
                .with_context(|| {
                    format!(
                        "failed to restore stale `adb shell wm size {}x{}`",
                        previous.width, previous.height
                    )
                })?;
        }
    }

    Ok(())
}

fn persist_display_state(
    layout: &ProjectLayout,
    serial: &str,
    restore: RestoreDisplayOverride,
) -> Result<()> {
    fs::create_dir_all(&layout.files.logs_dir)
        .with_context(|| format!("failed to create {}", layout.files.logs_dir.display()))?;
    let state = PersistedDisplayState {
        serial: serial.to_string(),
        restore: runtime_restore_to_persisted(restore),
    };
    let body = serde_json::to_string_pretty(&state).context("failed to encode display state")?;
    fs::write(display_state_path(layout), body).context("failed to persist display state")?;
    Ok(())
}

fn load_display_state(layout: &ProjectLayout) -> Result<PersistedDisplayState> {
    let body =
        fs::read_to_string(display_state_path(layout)).context("failed to read display state")?;
    serde_json::from_str(&body).context("failed to parse display state")
}

fn clear_display_state(layout: &ProjectLayout) -> Result<()> {
    let state_path = display_state_path(layout);
    match fs::remove_file(&state_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("failed to remove {}", state_path.display()))
        }
    }
}

fn display_state_path(layout: &ProjectLayout) -> PathBuf {
    layout.files.logs_dir.join(DISPLAY_STATE_FILE_NAME)
}

fn runtime_restore_to_persisted(
    restore: RestoreDisplayOverride,
) -> PersistedRestoreDisplayOverride {
    match restore {
        RestoreDisplayOverride::Reset => PersistedRestoreDisplayOverride::Reset,
        RestoreDisplayOverride::Restore(size) => PersistedRestoreDisplayOverride::Restore {
            width: size.width,
            height: size.height,
        },
    }
}

fn persisted_restore_to_runtime(
    restore: &PersistedRestoreDisplayOverride,
) -> RestoreDisplayOverride {
    match restore {
        PersistedRestoreDisplayOverride::Reset => RestoreDisplayOverride::Reset,
        PersistedRestoreDisplayOverride::Restore { width, height } => {
            RestoreDisplayOverride::Restore(DisplaySize {
                width: *width,
                height: *height,
            })
        }
    }
}

fn select_device_serial(
    explicit_serial: Option<String>,
    config: &mineplay_config::AppConfig,
    devices: &[DeviceEntry],
) -> Result<String> {
    if let Some(serial) = explicit_serial {
        return Ok(serial);
    }

    if let Some(serial) = config.playback.preferred_serial.clone() {
        return Ok(serial);
    }

    if let Some(serial) = config.android.paired_device.clone() {
        return Ok(serial);
    }

    devices
        .iter()
        .find(|device| device.state == "device")
        .map(|device| device.serial.clone())
        .context("no connected adb devices found")
}

fn print_output(output: mineplay_android_shell::OutputSummary) {
    if !output.stdout.is_empty() {
        println!("{}", output.stdout);
    }

    if !output.stderr.is_empty() {
        eprintln!("{}", output.stderr);
    }
}
