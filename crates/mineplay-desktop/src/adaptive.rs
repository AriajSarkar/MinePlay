use std::path::Path;

use anyhow::Result;
use mineplay_android_shell::{AdbRunner, DisplaySize};
use mineplay_config::AppConfig;
use mineplay_core::ProjectLayout;
use mineplay_scrcpy::{
    NewDisplaySpec, ScrcpyLaunchOptions, choose_preferred_h264_encoder, list_video_encoders,
    supports_option,
};

use crate::perf::{
    PerfProbeSummary, PlaySessionSummary, load_latest_play_session_summary,
    load_latest_probe_summary,
};

const HIGH_RTT_AVG_MS: u128 = 120;
const HIGH_RTT_P95_MS: u128 = 200;
const LOW_FPS_MARGIN: u32 = 6;
const HIGH_SKIPPED_FRAMES: u32 = 8;
const MIN_BITRATE_KBPS: u32 = 12_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveLaunchPlan {
    pub target_size: DisplaySize,
    pub dynamic_target: bool,
    pub device_sdk: Option<u32>,
    pub using_virtual_display: bool,
    pub selected_encoder: Option<String>,
    pub previous_probe: Option<PerfProbeSummary>,
    pub previous_session: Option<PlaySessionSummary>,
    pub notes: Vec<String>,
}

pub fn apply_adaptive_tuning(
    layout: &ProjectLayout,
    runner: &AdbRunner,
    scrcpy_path: &Path,
    serial: &str,
    config: &AppConfig,
    target_size: DisplaySize,
    dynamic_target: bool,
    options: &mut ScrcpyLaunchOptions,
) -> Result<AdaptiveLaunchPlan> {
    let previous_probe = load_latest_probe_summary(layout, serial)?;
    let previous_session = load_latest_play_session_summary(layout, serial)?;
    let device_sdk = runner.sdk_version(serial).ok();
    let mut notes = Vec::new();

    options.max_size = options
        .max_size
        .min(target_size.width.max(target_size.height));

    if options.video_encoder.is_none()
        && let Ok(encoders) = list_video_encoders(scrcpy_path, serial, options.adb_path.as_deref())
        && let Some(encoder) = choose_preferred_h264_encoder(&encoders)
    {
        options.video_encoder = Some(encoder.clone());
        notes.push(format!("video_encoder={encoder}"));
    }

    let using_virtual_display = config.playback.fill_mode == "auto"
        && config.playback.prefer_virtual_display
        && device_sdk.is_some_and(|sdk| sdk >= 29)
        && supports_option(scrcpy_path, "--new-display").unwrap_or(false);
    if using_virtual_display {
        options.new_display = Some(NewDisplaySpec {
            width: target_size.width,
            height: target_size.height,
            dpi: config.playback.virtual_display_dpi,
        });
        notes.push(format!(
            "virtual_display={}x{}",
            target_size.width, target_size.height
        ));
    }

    let probe_is_bad = previous_probe.as_ref().is_some_and(is_probe_bad);
    let session_is_bad = previous_session
        .as_ref()
        .is_some_and(|summary| is_session_bad(summary, options.max_fps));

    if probe_is_bad || session_is_bad {
        options.bitrate_kbps = ((options.bitrate_kbps as u64 * 85) / 100) as u32;
        options.bitrate_kbps = options.bitrate_kbps.max(MIN_BITRATE_KBPS);
        notes.push(format!("adaptive_bitrate_kbps={}", options.bitrate_kbps));
    }

    if session_is_bad {
        let target_max = target_size.width.max(target_size.height);
        let reduced_max = ((target_max as u64 * 92) / 100) as u32;
        let lower_bound = target_max.min(1280);
        options.max_size = reduced_max.max(lower_bound);
        notes.push(format!("adaptive_max_size={}", options.max_size));
    }

    Ok(AdaptiveLaunchPlan {
        target_size,
        dynamic_target,
        device_sdk,
        using_virtual_display,
        selected_encoder: options.video_encoder.clone(),
        previous_probe,
        previous_session,
        notes,
    })
}

fn is_probe_bad(summary: &PerfProbeSummary) -> bool {
    summary.avg_rtt_ms >= HIGH_RTT_AVG_MS || summary.p95_rtt_ms >= HIGH_RTT_P95_MS
}

fn is_session_bad(summary: &PlaySessionSummary, max_fps: Option<u16>) -> bool {
    summary.max_skipped_frames >= HIGH_SKIPPED_FRAMES
        || max_fps.is_some_and(|limit| {
            summary.fps_sample_count >= 6 && summary.avg_fps + LOW_FPS_MARGIN < limit as u32
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_quality_when_previous_metrics_are_clean() {
        let previous_probe = Some(PerfProbeSummary {
            sample_count: 4,
            min_rtt_ms: 40,
            avg_rtt_ms: 80,
            p95_rtt_ms: 120,
            max_rtt_ms: 140,
            device_ip: None,
            ping_sample_count: 0,
            min_ping_ms: 0,
            avg_ping_ms: 0,
            p95_ping_ms: 0,
            max_ping_ms: 0,
            log_path: "probe.jsonl".into(),
        });
        let previous_session = Some(PlaySessionSummary {
            fps_sample_count: 4,
            min_fps: 58,
            avg_fps: 59,
            max_fps: 60,
            max_skipped_frames: 1,
            adb_rtt_sample_count: 4,
            avg_rtt_ms: 80,
            p95_rtt_ms: 120,
            log_path: "play.jsonl".into(),
        });

        assert!(!previous_probe.as_ref().is_some_and(is_probe_bad));
        assert!(
            !previous_session
                .as_ref()
                .is_some_and(|summary| is_session_bad(summary, Some(60)))
        );
    }

    #[test]
    fn reduces_quality_when_previous_probe_is_bad() {
        let previous_probe = Some(PerfProbeSummary {
            sample_count: 4,
            min_rtt_ms: 60,
            avg_rtt_ms: 150,
            p95_rtt_ms: 260,
            max_rtt_ms: 260,
            device_ip: None,
            ping_sample_count: 0,
            min_ping_ms: 0,
            avg_ping_ms: 0,
            p95_ping_ms: 0,
            max_ping_ms: 0,
            log_path: "probe.jsonl".into(),
        });

        assert!(previous_probe.as_ref().is_some_and(is_probe_bad));
    }
}
