//! View types exchanged between the server and the browser (both builds).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    Offline,
    Idle,
    Starting,
    Running,
    Stopping,
    Error,
}

impl Status {
    pub const ALL: [Status; 6] = [
        Status::Running,
        Status::Starting,
        Status::Stopping,
        Status::Idle,
        Status::Error,
        Status::Offline,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Status::Offline => "Offline",
            Status::Idle => "Ready",
            Status::Starting => "Starting…",
            Status::Running => "Running",
            Status::Stopping => "Stopping…",
            Status::Error => "Error",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Status::Offline => "offline",
            Status::Idle => "idle",
            Status::Starting => "starting",
            Status::Running => "running",
            Status::Stopping => "stopping",
            Status::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Actor {
    Instructor,
    Board,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardView {
    pub host: String,
    pub label: Option<String>,
    pub status: Status,
    /// Extra words for the status, e.g. the error or what a job waits for.
    pub detail: Option<String>,
    /// Seconds since the last heartbeat; `None` if not seen since restart.
    pub seen_secs: Option<u64>,
    pub cpu_temp_c: Option<f32>,
    pub load1: Option<f32>,
    pub mem_used_pct: Option<u8>,
    pub uptime_s: Option<u64>,
    pub ip: Option<String>,
    pub agent_version: Option<String>,
    pub seb_version: Option<String>,
    pub image_hash: Option<String>,
    pub last_exit_code: Option<i32>,
    pub exam_name: Option<String>,
    /// Agent predates the proto crate: no stats, ignores exam configs.
    pub legacy_agent: bool,
}

impl BoardView {
    pub fn title(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.host)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExamChoice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetView {
    pub boards: Vec<BoardView>,
    pub exams: Vec<ExamChoice>,
    pub max_boards: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionReport {
    pub verb: String,
    pub queued: Vec<String>,
    /// (host, reason)
    pub skipped: Vec<(String, String)>,
    /// (host, note) for boards that were queued with a caveat.
    pub notes: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventView {
    pub id: u64,
    pub at_ms: i64,
    pub actor: Actor,
    pub level: Level,
    pub host: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardDetail {
    pub board: BoardView,
    pub first_seen_ms: i64,
    pub mem_total_kb: Option<u64>,
    pub mem_available_kb: Option<u64>,
    pub load: Option<[f32; 3]>,
    /// Oldest first, one sample per heartbeat.
    pub temp_history: Vec<f32>,
    pub history: Vec<EventView>,
    pub exams: Vec<ExamChoice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExamView {
    pub id: String,
    pub name: String,
    pub start_url: String,
    pub allowed_domains: Vec<String>,
    pub allow_quit: bool,
    pub created_ms: i64,
    pub config_json: String,
    /// The access code itself never leaves the server except via the gate.
    pub has_access_code: bool,
    /// Where boards start; `None` when EXAM_PUBLIC_URL is unset (no gate).
    pub gate_url: Option<String>,
    pub config_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub agent_token_missing: bool,
    pub version: String,
}
