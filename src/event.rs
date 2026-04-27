use std::time::Duration;

use chrono::{DateTime, Local};
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum ProbeOutcome {
    Reply {
        rtt: Duration,
        peer: String,
        bytes: Option<usize>,
        ttl: Option<u8>,
        detail: Vec<(String, String)>,
    },
    Timeout {
        detail: Vec<(String, String)>,
    },
    Error(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Recovery {
    pub lost: u64,
    pub duration_ms: u128,
}

#[derive(Debug, Clone)]
pub struct ProbeEvent {
    pub ts: DateTime<Local>,
    pub protocol: &'static str,
    pub target: String,
    pub seq: u64,
    pub outcome: ProbeOutcome,
    pub recovery: Option<Recovery>,
}

#[derive(Debug, Serialize)]
pub struct JsonProbeEvent<'a> {
    pub ts: String,
    pub protocol: &'static str,
    pub target: &'a str,
    pub seq: u64,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtt_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub detail: Vec<(&'a str, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<&'a Recovery>,
}

impl ProbeEvent {
    pub fn as_json(&self, ts: String) -> JsonProbeEvent<'_> {
        match &self.outcome {
            ProbeOutcome::Reply {
                rtt,
                peer,
                bytes,
                ttl,
                detail,
            } => JsonProbeEvent {
                ts,
                protocol: self.protocol,
                target: &self.target,
                seq: self.seq,
                status: "reply",
                rtt_ms: Some(rtt.as_secs_f64() * 1000.0),
                peer: Some(peer),
                bytes: *bytes,
                ttl: *ttl,
                detail: detail
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect(),
                error: None,
                recovery: self.recovery.as_ref(),
            },
            ProbeOutcome::Timeout { detail } => JsonProbeEvent {
                ts,
                protocol: self.protocol,
                target: &self.target,
                seq: self.seq,
                status: "timeout",
                rtt_ms: None,
                peer: None,
                bytes: None,
                ttl: None,
                detail: detail
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect(),
                error: None,
                recovery: self.recovery.as_ref(),
            },
            ProbeOutcome::Error(error) => JsonProbeEvent {
                ts,
                protocol: self.protocol,
                target: &self.target,
                seq: self.seq,
                status: "error",
                rtt_ms: None,
                peer: None,
                bytes: None,
                ttl: None,
                detail: Vec::new(),
                error: Some(error),
                recovery: self.recovery.as_ref(),
            },
        }
    }
}
