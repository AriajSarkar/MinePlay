use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ControlMode {
    ShellInjected,
    AccessibilityFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionId(Uuid);

impl SessionId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientHello {
    pub protocol_version: u16,
    pub session_id: SessionId,
    pub requested_mode: ControlMode,
    pub game_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerHello {
    pub protocol_version: u16,
    pub accepted_mode: ControlMode,
    pub video: VideoDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoDescriptor {
    pub width: u32,
    pub height: u32,
    pub fps: u16,
    pub bitrate_kbps: u32,
    pub codec: CodecKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CodecKind {
    H264,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoConfig {
    pub codec: CodecKind,
    pub codec_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoFrame {
    pub pts_micros: u64,
    pub keyframe: bool,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelemetrySnapshot {
    pub rtt_millis: u32,
    pub packet_loss_ppm: u32,
    pub encode_queue_depth: u16,
    pub present_queue_depth: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ControlEvent {
    MouseMotion {
        dx: i32,
        dy: i32,
        timestamp_micros: u64,
    },
    MouseButton {
        button: MouseButton,
        state: ButtonState,
        timestamp_micros: u64,
    },
    MouseWheel {
        lines: i16,
        timestamp_micros: u64,
    },
    Key {
        physical_key: String,
        state: ButtonState,
        timestamp_micros: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Payload {
    ClientHello(ClientHello),
    ServerHello(ServerHello),
    VideoConfig(VideoConfig),
    VideoFrame(VideoFrame),
    Control(ControlEvent),
    Telemetry(TelemetrySnapshot),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    pub sequence: u64,
    pub sent_at_micros: u64,
    pub payload: Payload,
}

pub fn encode_envelope(envelope: &Envelope) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(envelope)
}

pub fn decode_envelope(bytes: &[u8]) -> Result<Envelope, bincode::Error> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_roundtrip() {
        let envelope = Envelope {
            sequence: 7,
            sent_at_micros: 99,
            payload: Payload::ClientHello(ClientHello {
                protocol_version: PROTOCOL_VERSION,
                session_id: SessionId::new(),
                requested_mode: ControlMode::ShellInjected,
                game_profile: "bedrock".to_string(),
            }),
        };

        let encoded = encode_envelope(&envelope).expect("encode");
        let decoded = decode_envelope(&encoded).expect("decode");

        assert_eq!(decoded, envelope);
    }
}
