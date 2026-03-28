mod adaptive;
mod cli;
mod host_display;
mod perf;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use adaptive::{AdaptiveLaunchPlan, apply_adaptive_tuning};
use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command as CliCommand};
use host_display::resolve_display_target;
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
use perf::{PerfSessionConfig, run_monitored_scrcpy, run_perf_probe};
use serde::{Deserialize, Serialize};
use tracing::warn;
use tracing_subscriber::{EnvFilter, fmt};

const DISPLAY_STATE_FILE_NAME: &str = "display-override-state.json";

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayDisplayPlan {
    crop: Option<String>,
    display_override: Option<PendingDisplayOverride>,
    target_size: DisplaySize,
    using_virtual_display: bool,
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
        CliCommand::PerfProbe {
            serial,
            adb,
            seconds,
            interval_ms,
        } => run_perf_probe_command(&layout, serial, adb, seconds, interval_ms),
        CliCommand::Play {
            serial,
            adb,
            scrcpy,
            perf_log,
            install_if_missing,
            dry_run,
        } => run_play(
            &layout,
            serial,
            adb,
            scrcpy,
            perf_log,
            install_if_missing,
            dry_run,
        ),
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
    perf_log: bool,
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
    let play_serial = options.serial.clone();
    options.adb_path = Some(adb_path);
    let target_display = resolve_display_target(&config);
    let adaptive_plan = apply_adaptive_tuning(
        layout,
        &runner,
        &scrcpy_path,
        &play_serial,
        &config,
        target_display.size,
        target_display.dynamic,
        &mut options,
    )?;
    let perf_session = build_perf_session_config(&config, perf_log);
    if perf_session.enable_output_log {
        options.print_fps = true;
    }
    let display_plan = build_display_plan(
        &runner,
        &options.serial,
        &config,
        adaptive_plan.target_size,
        adaptive_plan.using_virtual_display,
    )?;
    options.crop = display_plan.crop.clone();

    println!("scrcpy={}", scrcpy_path.display());
    println!("serial={}", options.serial);
    print_adaptive_plan(&adaptive_plan);
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
    let status = if perf_session.enable_output_log || perf_session.enable_adb_rtt_probe {
        let result = run_monitored_scrcpy(layout, &runner, &scrcpy_path, &options, perf_session)?;
        println!("perf_log={}", result.log_path.display());
        Ok(result.status)
    } else {
        launch_scrcpy(&scrcpy_path, &options)
    };

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

fn run_perf_probe_command(
    layout: &ProjectLayout,
    serial: Option<String>,
    adb: Option<PathBuf>,
    seconds: u64,
    interval_ms: u64,
) -> Result<()> {
    let config = load_config(&layout.files.config_file)
        .with_context(|| format!("missing config file {}", layout.files.config_file.display()))?;
    let runner = runner_for(layout, adb.as_deref())?;
    let devices = runner.connected_devices()?;
    let selected_serial = select_device_serial(serial, &config, &devices)?;
    let device_ip = runner.wifi_ipv4(&selected_serial)?;
    let probe = run_perf_probe(
        layout,
        &runner,
        &selected_serial,
        device_ip.as_deref(),
        seconds,
        interval_ms,
        config.diagnostics.adb_rtt_probe_timeout_ms,
    )?;
    let summary = &probe.summary;

    println!("serial={selected_serial}");
    println!(
        "device_ip={}",
        summary.device_ip.as_deref().unwrap_or("unavailable")
    );
    println!("samples={}", summary.sample_count);
    println!("min_rtt_ms={}", summary.min_rtt_ms);
    println!("avg_rtt_ms={}", summary.avg_rtt_ms);
    println!("p95_rtt_ms={}", summary.p95_rtt_ms);
    println!("max_rtt_ms={}", summary.max_rtt_ms);
    println!("ping_samples={}", summary.ping_sample_count);
    println!("min_ping_ms={}", summary.min_ping_ms);
    println!("avg_ping_ms={}", summary.avg_ping_ms);
    println!("p95_ping_ms={}", summary.p95_ping_ms);
    println!("max_ping_ms={}", summary.max_ping_ms);
    println!("perf_log={}", summary.log_path.display());
    if config.diagnostics.compare_previous_perf
        && let Some(comparison) = &probe.comparison
    {
        println!(
            "previous_perf_log={}",
            comparison.previous_log_path.display()
        );
        println!("delta_avg_rtt_ms={}", comparison.delta_avg_rtt_ms);
        println!("delta_p95_rtt_ms={}", comparison.delta_p95_rtt_ms);
        println!("delta_max_rtt_ms={}", comparison.delta_max_rtt_ms);
        println!("delta_avg_ping_ms={}", comparison.delta_avg_ping_ms);
        println!("delta_p95_ping_ms={}", comparison.delta_p95_ping_ms);
    }
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
    target_size: DisplaySize,
    using_virtual_display: bool,
) -> Result<PlayDisplayPlan> {
    if using_virtual_display {
        return Ok(PlayDisplayPlan {
            crop: None,
            display_override: None,
            target_size,
            using_virtual_display: true,
        });
    }

    match config.playback.fill_mode.as_str() {
        "fit" => Ok(PlayDisplayPlan {
            crop: None,
            display_override: None,
            target_size,
            using_virtual_display: false,
        }),
        "crop" => {
            let size_info = runner.wm_size(serial)?;
            let size = size_info.override_size.unwrap_or(size_info.physical);
            Ok(PlayDisplayPlan {
                crop: compute_crop(
                    size.width,
                    size.height,
                    target_size.width,
                    target_size.height,
                ),
                display_override: None,
                target_size,
                using_virtual_display: false,
            })
        }
        "auto" => build_auto_display_plan(runner, serial, target_size),
        other => anyhow::bail!("unsupported playback.fill_mode `{other}`"),
    }
}

fn build_auto_display_plan(
    runner: &AdbRunner,
    serial: &str,
    target_size: DisplaySize,
) -> Result<PlayDisplayPlan> {
    let size_info = runner.wm_size(serial)?;
    let requested = compute_display_override(
        size_info.physical.width,
        size_info.physical.height,
        target_size.width,
        target_size.height,
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
        target_size,
        using_virtual_display: false,
    })
}

fn print_display_plan(fill_mode: &str, plan: &PlayDisplayPlan) {
    println!(
        "display_target={}x{}",
        plan.target_size.width, plan.target_size.height
    );

    if plan.using_virtual_display {
        println!("display_mode=virtual");
        println!("display_override=none");
        return;
    }

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

fn print_adaptive_plan(plan: &AdaptiveLaunchPlan) {
    println!("target_dynamic={}", plan.dynamic_target);
    println!(
        "device_sdk={}",
        plan.device_sdk
            .map(|sdk| sdk.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!("virtual_display={}", plan.using_virtual_display);
    println!(
        "selected_encoder={}",
        plan.selected_encoder.as_deref().unwrap_or("default")
    );
    if let Some(previous_probe) = &plan.previous_probe {
        println!("previous_probe_log={}", previous_probe.log_path.display());
        println!("previous_probe_avg_rtt_ms={}", previous_probe.avg_rtt_ms);
        println!("previous_probe_p95_rtt_ms={}", previous_probe.p95_rtt_ms);
    }
    if let Some(previous_session) = &plan.previous_session {
        println!(
            "previous_session_log={}",
            previous_session.log_path.display()
        );
        println!("previous_session_avg_fps={}", previous_session.avg_fps);
        println!(
            "previous_session_max_skipped_frames={}",
            previous_session.max_skipped_frames
        );
    }
    for note in &plan.notes {
        println!("adaptive_note={note}");
    }
}

fn build_perf_session_config(
    config: &mineplay_config::AppConfig,
    force_perf_log: bool,
) -> PerfSessionConfig {
    PerfSessionConfig {
        enable_output_log: force_perf_log
            || config.diagnostics.enable_session_perf_log
            || config.diagnostics.enable_scrcpy_fps_counter,
        enable_adb_rtt_probe: force_perf_log || config.diagnostics.enable_adb_rtt_probe,
        adb_rtt_probe_interval: std::time::Duration::from_millis(
            config.diagnostics.adb_rtt_probe_interval_ms.max(50),
        ),
        adb_rtt_slow_threshold: std::time::Duration::from_millis(
            config.diagnostics.adb_rtt_probe_timeout_ms.max(1),
        ),
    }
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
