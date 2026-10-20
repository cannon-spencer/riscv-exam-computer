//! Wire types shared by the exam server, `seb-agent`, and `sim-agent`.
//!
//! Compatibility contract: agents built before this crate existed send
//! `{"host","state"}` and read `{"job":"start"|"stop"|null}`. Every field added
//! since then is optional on both sides, and `job` is always serialized (as
//! `null` when there is no job) so old agents keep working.

pub mod seb;

use serde::{Deserialize, Serialize};

/// Bumped when agents gain new behavior the server may rely on.
pub const PROTOCOL_VERSION: u32 = 2;

/// State strings an agent reports. Kept as strings on the wire because old
/// agents send free-form `state` (and an empty string when missing).
pub mod state {
    pub const IDLE: &str = "idle";
    pub const RUNNING: &str = "running";
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Heartbeat {
    pub host: String,
    #[serde(default)]
    pub state: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seb_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_temp_c: Option<f32>,
    /// 1, 5 and 15 minute load averages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem: Option<MemStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uptime_s: Option<u64>,
    /// Exit code of the last SEB session (start script) that ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_exit_code: Option<i32>,
    /// Outcome of the most recent job the agent executed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_result: Option<JobResult>,
    /// Exam the running SEB session was started with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exam_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MemStats {
    pub total_kb: u64,
    pub available_kb: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobResult {
    pub id: u64,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobKind {
    Start,
    Stop,
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JobKind::Start => "start",
            JobKind::Stop => "stop",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatReply {
    pub ok: bool,
    /// Always present (possibly `null`): old agents read only this field.
    pub job: Option<JobKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<u64>,
    /// Exam to run, attached to `start` jobs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exam: Option<ExamPayload>,
    /// Human-readable reason when `ok` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExamPayload {
    pub id: String,
    pub name: String,
    /// seb-linux JSON configuration, written to disk verbatim by the agent.
    pub config: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn old_agent_heartbeat_parses() {
        let hb: Heartbeat = serde_json::from_str(r#"{"host":"orangepirv","state":"idle"}"#).unwrap();
        assert_eq!(hb.host, "orangepirv");
        assert_eq!(hb.state, "idle");
        assert!(hb.cpu_temp_c.is_none());
    }

    #[test]
    fn heartbeat_without_state_parses() {
        let hb: Heartbeat = serde_json::from_str(r#"{"host":"a"}"#).unwrap();
        assert_eq!(hb.state, "");
    }

    #[test]
    fn reply_always_has_job_field() {
        let v = serde_json::to_value(HeartbeatReply { ok: true, ..Default::default() }).unwrap();
        assert_eq!(v, json!({"ok": true, "job": null}));

        let v = serde_json::to_value(HeartbeatReply {
            ok: true,
            job: Some(JobKind::Start),
            job_id: Some(7),
            ..Default::default()
        })
        .unwrap();
        // The old agent does `v.get("job").and_then(|j| j.as_str())`.
        assert_eq!(v.get("job").and_then(|j| j.as_str()), Some("start"));
    }

    #[test]
    fn new_reply_parses_old_shape() {
        let r: HeartbeatReply = serde_json::from_str(r#"{"ok":true,"job":"stop"}"#).unwrap();
        assert_eq!(r.job, Some(JobKind::Stop));
        assert!(r.exam.is_none());
    }
}
