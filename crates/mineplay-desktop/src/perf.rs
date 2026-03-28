use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use mineplay_android_shell::AdbRunner;
use mineplay_core::ProjectLayout;
use mineplay_scrcpy::{ScrcpyLaunchOptions, spawn_scrcpy};
use serde_json::{Value, json};

const PERF_LOG_DIR_NAME: &str = "perf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerfSessionConfig {
    pub enable_output_log: bool,
    pub enable_adb_rtt_probe: bool,
    pub adb_rtt_probe_interval: Duration,
    pub adb_rtt_slow_threshold: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfSessionResult {
    pub status: ExitStatus,
    pub log_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfProbeSummary {
    pub sample_count: usize,
    pub min_rtt_ms: u128,
    pub avg_rtt_ms: u128,
    pub p95_rtt_ms: u128,
    pub max_rtt_ms: u128,
    pub device_ip: Option<String>,
    pub ping_sample_count: usize,
    pub min_ping_ms: u128,
    pub avg_ping_ms: u128,
    pub p95_ping_ms: u128,
    pub max_ping_ms: u128,
    pub log_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfProbeRunResult {
    pub summary: PerfProbeSummary,
    pub comparison: Option<PerfProbeComparison>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfProbeComparison {
    pub previous_log_path: PathBuf,
    pub delta_avg_rtt_ms: i128,
    pub delta_p95_rtt_ms: i128,
    pub delta_max_rtt_ms: i128,
    pub delta_avg_ping_ms: i128,
    pub delta_p95_ping_ms: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaySessionSummary {
    pub fps_sample_count: usize,
    pub min_fps: u32,
    pub avg_fps: u32,
    pub max_fps: u32,
    pub max_skipped_frames: u32,
    pub adb_rtt_sample_count: usize,
    pub avg_rtt_ms: u128,
    pub p95_rtt_ms: u128,
    pub log_path: PathBuf,
}

#[derive(Clone)]
struct PerfLogWriter {
    file: Arc<Mutex<File>>,
}

impl PerfLogWriter {
    fn open(layout: &ProjectLayout, serial: &str, kind: &str) -> Result<(Self, PathBuf)> {
        let log_dir = layout.files.logs_dir.join(PERF_LOG_DIR_NAME);
        fs::create_dir_all(&log_dir)
            .with_context(|| format!("failed to create {}", log_dir.display()))?;
        let safe_serial = serial.replace(['\\', '/', ':', '.'], "_");
        let log_path = log_dir.join(format!(
            "{}-{}-{}.jsonl",
            kind,
            safe_serial,
            unix_timestamp_ms()
        ));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("failed to create {}", log_path.display()))?;

        Ok((
            Self {
                file: Arc::new(Mutex::new(file)),
            },
            log_path,
        ))
    }

    fn write_event(&self, value: serde_json::Value) -> Result<()> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| anyhow::anyhow!("failed to lock perf log file"))?;
        writeln!(file, "{value}").context("failed to write perf log event")
    }
}

pub fn run_monitored_scrcpy(
    layout: &ProjectLayout,
    runner: &AdbRunner,
    scrcpy_path: &Path,
    options: &ScrcpyLaunchOptions,
    config: PerfSessionConfig,
) -> Result<PerfSessionResult> {
    let (writer, log_path) = PerfLogWriter::open(layout, &options.serial, "play-session")?;
    writer.write_event(json!({
        "ts_unix_ms": unix_timestamp_ms(),
        "event": "session_start",
        "serial": options.serial,
        "args": options
            .args()
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
    }))?;

    let mut child = spawn_scrcpy(scrcpy_path, options, true)?;
    let running = Arc::new(AtomicBool::new(true));
    let mut workers = Vec::new();

    if let Some(stdout) = child.stdout.take() {
        workers.push(spawn_output_pump(
            "stdout",
            stdout,
            writer.clone(),
            false,
            config.enable_output_log,
        ));
    }

    if let Some(stderr) = child.stderr.take() {
        workers.push(spawn_output_pump(
            "stderr",
            stderr,
            writer.clone(),
            true,
            config.enable_output_log,
        ));
    }

    let probe_worker = if config.enable_adb_rtt_probe {
        Some(spawn_rtt_probe(
            runner.clone(),
            options.serial.clone(),
            writer.clone(),
            running.clone(),
            config.adb_rtt_probe_interval,
            config.adb_rtt_slow_threshold,
        ))
    } else {
        None
    };

    let status = child.wait().with_context(|| {
        format!(
            "failed while waiting for scrcpy at {}",
            scrcpy_path.display()
        )
    })?;
    running.store(false, Ordering::Relaxed);

    if let Some(worker) = probe_worker {
        let _ = worker.join();
    }

    for worker in workers {
        let _ = worker.join();
    }

    writer.write_event(json!({
        "ts_unix_ms": unix_timestamp_ms(),
        "event": "session_exit",
        "serial": options.serial,
        "exit_code": status.code(),
    }))?;

    Ok(PerfSessionResult { status, log_path })
}

pub fn run_perf_probe(
    layout: &ProjectLayout,
    runner: &AdbRunner,
    serial: &str,
    device_ip: Option<&str>,
    seconds: u64,
    interval_ms: u64,
    slow_threshold_ms: u64,
) -> Result<PerfProbeRunResult> {
    let (writer, log_path) = PerfLogWriter::open(layout, serial, "adb-rtt-probe")?;
    let deadline = Instant::now() + Duration::from_secs(seconds.max(1));
    let interval = Duration::from_millis(interval_ms.max(50));
    let slow_threshold = Duration::from_millis(slow_threshold_ms.max(1));
    let mut adb_samples = Vec::new();
    let mut ping_samples = Vec::new();

    writer.write_event(json!({
        "ts_unix_ms": unix_timestamp_ms(),
        "event": "probe_start",
        "serial": serial,
        "device_ip": device_ip,
        "seconds": seconds,
        "interval_ms": interval_ms,
    }))?;

    while Instant::now() < deadline {
        let started = Instant::now();
        match runner.shell_true(serial) {
            Ok(_) => {
                let rtt = started.elapsed().as_millis();
                adb_samples.push(rtt);
                writer.write_event(json!({
                    "ts_unix_ms": unix_timestamp_ms(),
                    "event": "adb_rtt_sample",
                    "serial": serial,
                    "rtt_ms": rtt,
                    "slow_threshold_ms": slow_threshold.as_millis(),
                    "is_slow": started.elapsed() > slow_threshold,
                }))?;
            }
            Err(error) => {
                writer.write_event(json!({
                    "ts_unix_ms": unix_timestamp_ms(),
                    "event": "adb_rtt_error",
                    "serial": serial,
                    "error": format!("{error:#}"),
                }))?;
            }
        }

        if let Some(device_ip) = device_ip
            && let Some(ping_ms) = probe_windows_ping(device_ip)
        {
            ping_samples.push(ping_ms);
            writer.write_event(json!({
                "ts_unix_ms": unix_timestamp_ms(),
                "event": "wifi_ping_sample",
                "serial": serial,
                "device_ip": device_ip,
                "ping_ms": ping_ms,
            }))?;
        }

        thread::sleep(interval);
    }

    adb_samples.sort_unstable();
    ping_samples.sort_unstable();
    let summary = summarize_probe_samples(adb_samples, ping_samples, device_ip, log_path);
    let comparison = load_previous_probe_summary(layout, serial, Some(&summary.log_path))?
        .map(|previous| compare_probe_summaries(&summary, &previous));
    writer.write_event(json!({
        "ts_unix_ms": unix_timestamp_ms(),
        "event": "probe_end",
        "serial": serial,
        "sample_count": summary.sample_count,
        "min_rtt_ms": summary.min_rtt_ms,
        "avg_rtt_ms": summary.avg_rtt_ms,
        "p95_rtt_ms": summary.p95_rtt_ms,
        "max_rtt_ms": summary.max_rtt_ms,
        "ping_sample_count": summary.ping_sample_count,
        "min_ping_ms": summary.min_ping_ms,
        "avg_ping_ms": summary.avg_ping_ms,
        "p95_ping_ms": summary.p95_ping_ms,
        "max_ping_ms": summary.max_ping_ms,
    }))?;
    if let Some(comparison) = &comparison {
        writer.write_event(json!({
            "ts_unix_ms": unix_timestamp_ms(),
            "event": "probe_compare",
            "serial": serial,
            "previous_log_path": comparison.previous_log_path.display().to_string(),
            "delta_avg_rtt_ms": comparison.delta_avg_rtt_ms,
            "delta_p95_rtt_ms": comparison.delta_p95_rtt_ms,
            "delta_max_rtt_ms": comparison.delta_max_rtt_ms,
            "delta_avg_ping_ms": comparison.delta_avg_ping_ms,
            "delta_p95_ping_ms": comparison.delta_p95_ping_ms,
        }))?;
    }
    Ok(PerfProbeRunResult {
        summary,
        comparison,
    })
}

fn summarize_probe_samples(
    adb_samples: Vec<u128>,
    ping_samples: Vec<u128>,
    device_ip: Option<&str>,
    log_path: PathBuf,
) -> PerfProbeSummary {
    if adb_samples.is_empty() {
        return PerfProbeSummary {
            sample_count: 0,
            min_rtt_ms: 0,
            avg_rtt_ms: 0,
            p95_rtt_ms: 0,
            max_rtt_ms: 0,
            device_ip: device_ip.map(str::to_string),
            ping_sample_count: 0,
            min_ping_ms: 0,
            avg_ping_ms: 0,
            p95_ping_ms: 0,
            max_ping_ms: 0,
            log_path,
        };
    }

    let min_rtt_ms = *adb_samples.first().unwrap_or(&0);
    let max_rtt_ms = *adb_samples.last().unwrap_or(&0);
    let avg_rtt_ms = adb_samples.iter().sum::<u128>() / adb_samples.len() as u128;
    let p95_index = ((adb_samples.len() - 1) as f64 * 0.95).round() as usize;
    let p95_rtt_ms = adb_samples[p95_index];

    let (ping_sample_count, min_ping_ms, avg_ping_ms, p95_ping_ms, max_ping_ms) =
        if ping_samples.is_empty() {
            (0, 0, 0, 0, 0)
        } else {
            let min_ping_ms = *ping_samples.first().unwrap_or(&0);
            let max_ping_ms = *ping_samples.last().unwrap_or(&0);
            let avg_ping_ms = ping_samples.iter().sum::<u128>() / ping_samples.len() as u128;
            let p95_index = ((ping_samples.len() - 1) as f64 * 0.95).round() as usize;
            (
                ping_samples.len(),
                min_ping_ms,
                avg_ping_ms,
                ping_samples[p95_index],
                max_ping_ms,
            )
        };

    PerfProbeSummary {
        sample_count: adb_samples.len(),
        min_rtt_ms,
        avg_rtt_ms,
        p95_rtt_ms,
        max_rtt_ms,
        device_ip: device_ip.map(str::to_string),
        ping_sample_count,
        min_ping_ms,
        avg_ping_ms,
        p95_ping_ms,
        max_ping_ms,
        log_path,
    }
}

pub fn load_latest_probe_summary(
    layout: &ProjectLayout,
    serial: &str,
) -> Result<Option<PerfProbeSummary>> {
    load_previous_probe_summary(layout, serial, None)
}

pub fn load_previous_probe_summary(
    layout: &ProjectLayout,
    serial: &str,
    exclude_path: Option<&Path>,
) -> Result<Option<PerfProbeSummary>> {
    load_latest_summary_for_kind(
        layout,
        "adb-rtt-probe",
        serial,
        exclude_path,
        parse_probe_summary,
    )
}

pub fn load_latest_play_session_summary(
    layout: &ProjectLayout,
    serial: &str,
) -> Result<Option<PlaySessionSummary>> {
    load_latest_summary_for_kind(
        layout,
        "play-session",
        serial,
        None,
        parse_play_session_summary,
    )
}

#[must_use]
pub fn compare_probe_summaries(
    current: &PerfProbeSummary,
    previous: &PerfProbeSummary,
) -> PerfProbeComparison {
    PerfProbeComparison {
        previous_log_path: previous.log_path.clone(),
        delta_avg_rtt_ms: current.avg_rtt_ms as i128 - previous.avg_rtt_ms as i128,
        delta_p95_rtt_ms: current.p95_rtt_ms as i128 - previous.p95_rtt_ms as i128,
        delta_max_rtt_ms: current.max_rtt_ms as i128 - previous.max_rtt_ms as i128,
        delta_avg_ping_ms: current.avg_ping_ms as i128 - previous.avg_ping_ms as i128,
        delta_p95_ping_ms: current.p95_ping_ms as i128 - previous.p95_ping_ms as i128,
    }
}

fn spawn_output_pump<R: std::io::Read + Send + 'static>(
    stream: &'static str,
    reader: R,
    writer: PerfLogWriter,
    to_stderr: bool,
    enable_output_log: bool,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines().map_while(Result::ok) {
            if to_stderr {
                eprintln!("{line}");
            } else {
                println!("{line}");
            }

            if enable_output_log {
                let _ = writer.write_event(json!({
                    "ts_unix_ms": unix_timestamp_ms(),
                    "event": "scrcpy_output",
                    "stream": stream,
                    "line": line,
                }));
            }
        }
    })
}

fn load_latest_summary_for_kind<T>(
    layout: &ProjectLayout,
    kind: &str,
    serial: &str,
    exclude_path: Option<&Path>,
    parser: fn(&Path) -> Result<Option<T>>,
) -> Result<Option<T>> {
    let log_dir = layout.files.logs_dir.join(PERF_LOG_DIR_NAME);
    if !log_dir.exists() {
        return Ok(None);
    }

    let safe_serial = serial.replace(['\\', '/', ':', '.'], "_");
    let prefix = format!("{kind}-{safe_serial}-");
    let mut entries = fs::read_dir(&log_dir)
        .with_context(|| format!("failed to read {}", log_dir.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with(&prefix) && value.ends_with(".jsonl"))
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries.reverse();

    for path in entries {
        if exclude_path.is_some_and(|exclude| exclude == path) {
            continue;
        }

        if let Some(summary) = parser(&path)? {
            return Ok(Some(summary));
        }
    }

    Ok(None)
}

fn parse_probe_summary(path: &Path) -> Result<Option<PerfProbeSummary>> {
    for line in fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .lines()
        .rev()
    {
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("event").and_then(Value::as_str) != Some("probe_end") {
            continue;
        }

        return Ok(Some(PerfProbeSummary {
            sample_count: value_u64(&value, "sample_count") as usize,
            min_rtt_ms: value_u64(&value, "min_rtt_ms") as u128,
            avg_rtt_ms: value_u64(&value, "avg_rtt_ms") as u128,
            p95_rtt_ms: value_u64(&value, "p95_rtt_ms") as u128,
            max_rtt_ms: value_u64(&value, "max_rtt_ms") as u128,
            device_ip: value
                .get("device_ip")
                .and_then(Value::as_str)
                .map(str::to_string),
            ping_sample_count: value_u64(&value, "ping_sample_count") as usize,
            min_ping_ms: value_u64(&value, "min_ping_ms") as u128,
            avg_ping_ms: value_u64(&value, "avg_ping_ms") as u128,
            p95_ping_ms: value_u64(&value, "p95_ping_ms") as u128,
            max_ping_ms: value_u64(&value, "max_ping_ms") as u128,
            log_path: path.to_path_buf(),
        }));
    }

    Ok(None)
}

fn parse_play_session_summary(path: &Path) -> Result<Option<PlaySessionSummary>> {
    let mut fps_samples = Vec::new();
    let mut rtt_samples = Vec::new();

    for line in fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .lines()
    {
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };

        match value.get("event").and_then(Value::as_str) {
            Some("scrcpy_output") => {
                if let Some(line) = value.get("line").and_then(Value::as_str)
                    && let Some((fps, skipped)) = parse_fps_line(line)
                {
                    fps_samples.push((fps, skipped));
                }
            }
            Some("adb_rtt_sample") => {
                if let Some(rtt_ms) = value.get("rtt_ms").and_then(Value::as_u64) {
                    rtt_samples.push(rtt_ms as u128);
                }
            }
            _ => {}
        }
    }

    if fps_samples.is_empty() && rtt_samples.is_empty() {
        return Ok(None);
    }

    let stabilized_fps_samples = if fps_samples.len() > 3 {
        fps_samples.iter().copied().skip(2).collect::<Vec<_>>()
    } else {
        fps_samples.clone()
    };
    let mut fps_values = stabilized_fps_samples
        .iter()
        .map(|(fps, _)| *fps)
        .collect::<Vec<_>>();
    fps_values.sort_unstable();
    rtt_samples.sort_unstable();

    let (min_fps, avg_fps, max_fps) = if fps_values.is_empty() {
        (0, 0, 0)
    } else {
        (
            *fps_values.first().unwrap_or(&0),
            fps_values.iter().sum::<u32>() / fps_values.len() as u32,
            *fps_values.last().unwrap_or(&0),
        )
    };
    let max_skipped_frames = stabilized_fps_samples
        .iter()
        .map(|(_, skipped)| *skipped)
        .max()
        .unwrap_or(0);
    let p95_rtt_ms = if rtt_samples.is_empty() {
        0
    } else {
        let index = ((rtt_samples.len() - 1) as f64 * 0.95).round() as usize;
        rtt_samples[index]
    };
    let avg_rtt_ms = if rtt_samples.is_empty() {
        0
    } else {
        rtt_samples.iter().sum::<u128>() / rtt_samples.len() as u128
    };

    Ok(Some(PlaySessionSummary {
        fps_sample_count: fps_values.len(),
        min_fps,
        avg_fps,
        max_fps,
        max_skipped_frames,
        adb_rtt_sample_count: rtt_samples.len(),
        avg_rtt_ms,
        p95_rtt_ms,
        log_path: path.to_path_buf(),
    }))
}

fn parse_fps_line(line: &str) -> Option<(u32, u32)> {
    let fps_index = line.find(" fps")?;
    let fps_digits = line[..fps_index]
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    let fps = fps_digits.parse().ok()?;

    let skipped = line
        .split("(+")
        .nth(1)
        .and_then(|value| {
            let digits: String = value.chars().take_while(|ch| ch.is_ascii_digit()).collect();
            (!digits.is_empty()).then_some(digits)
        })
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(0);

    Some((fps, skipped))
}

fn value_u64(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn spawn_rtt_probe(
    runner: AdbRunner,
    serial: String,
    writer: PerfLogWriter,
    running: Arc<AtomicBool>,
    interval: Duration,
    slow_threshold: Duration,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while running.load(Ordering::Relaxed) {
            let started = Instant::now();
            match runner.shell_true(&serial) {
                Ok(_) => {
                    let elapsed = started.elapsed();
                    let _ = writer.write_event(json!({
                        "ts_unix_ms": unix_timestamp_ms(),
                        "event": "adb_rtt_sample",
                        "serial": serial,
                        "rtt_ms": elapsed.as_millis(),
                        "slow_threshold_ms": slow_threshold.as_millis(),
                        "is_slow": elapsed > slow_threshold,
                    }));
                }
                Err(error) => {
                    let _ = writer.write_event(json!({
                        "ts_unix_ms": unix_timestamp_ms(),
                        "event": "adb_rtt_error",
                        "serial": serial,
                        "error": format!("{error:#}"),
                    }));
                }
            }

            thread::sleep(interval);
        }
    })
}

fn unix_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn probe_windows_ping(device_ip: &str) -> Option<u128> {
    let output = std::process::Command::new("ping")
        .args(["-n", "1", "-w", "1000", device_ip])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_windows_ping_ms(&stdout)
}

fn parse_windows_ping_ms(stdout: &str) -> Option<u128> {
    for line in stdout.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(index) = lower.find("time=") {
            let value = &line[index + 5..];
            let digits: String = value.chars().take_while(|ch| ch.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return digits.parse().ok();
            }
        }

        if lower.contains("time<1ms") {
            return Some(1);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fps_lines_with_and_without_skips() {
        assert_eq!(parse_fps_line("INFO: 59 fps"), Some((59, 0)));
        assert_eq!(
            parse_fps_line("INFO: 30 fps (+4 frames skipped)"),
            Some((30, 4))
        );
    }
}
