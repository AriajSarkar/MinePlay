use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "mineplay-desktop",
    about = "Desktop bootstrap for non-root Android Minecraft play"
)]
pub struct Cli {
    #[arg(long, default_value = ".")]
    pub workspace_root: PathBuf,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    Doctor,
    SessionPlan,
    Devices {
        #[arg(long)]
        adb: Option<PathBuf>,
    },
    Pair {
        host_port: String,
        code: String,
        #[arg(long)]
        adb: Option<PathBuf>,
    },
    Connect {
        serial: String,
        #[arg(long)]
        adb: Option<PathBuf>,
    },
    InstallAgent {
        serial: String,
        apk: PathBuf,
        #[arg(long)]
        adb: Option<PathBuf>,
    },
    InstallScrcpy {
        #[arg(long)]
        version: Option<String>,
    },
    ResetDisplay {
        #[arg(long)]
        serial: Option<String>,
        #[arg(long)]
        adb: Option<PathBuf>,
    },
    PerfProbe {
        #[arg(long)]
        serial: Option<String>,
        #[arg(long)]
        adb: Option<PathBuf>,
        #[arg(long, default_value_t = 15)]
        seconds: u64,
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
    },
    Play {
        #[arg(long)]
        serial: Option<String>,
        #[arg(long)]
        adb: Option<PathBuf>,
        #[arg(long)]
        scrcpy: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        perf_log: bool,
        #[arg(long, default_value_t = true)]
        install_if_missing: bool,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}
