//! Board inventory, job lifecycle, exams, and the activity log.
//!
//! All methods take `now` so tests can drive time. The HTTP layer wraps a
//! `Fleet` in a mutex and never holds the lock across an await.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use proto::seb::ExamSpec;
use proto::{ExamPayload, Heartbeat, HeartbeatReply, JobKind, state};
use serde::{Deserialize, Serialize};

use crate::model::*;

/// No heartbeat for this long means offline. Agents beat every 5 s.
pub const OFFLINE_AFTER: Duration = Duration::from_secs(15);
/// A queued job the board never picked up is dropped after this.
pub const PICKUP_TTL: Duration = Duration::from_secs(60);
/// After pickup, SEB must be running within this long.
pub const START_TIMEOUT: Duration = Duration::from_secs(90);
/// After pickup, SEB must be gone within this long.
pub const STOP_TIMEOUT: Duration = Duration::from_secs(45);

pub const MAX_EVENTS: usize = 5000;
pub const MAX_EXAMS: usize = 200;
pub const MAX_LABEL: usize = 48;
pub const MAX_EXAM_NAME: usize = 80;
const TEMP_SAMPLES: usize = 120;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exam {
    pub id: String,
    pub name: String,
    pub spec: ExamSpec,
    pub created_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub at_ms: i64,
    pub actor: Actor,
    pub level: Level,
    #[serde(default)]
    pub host: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardRecord {
    pub host: String,
    #[serde(default)]
    pub label: Option<String>,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
}

/// Everything that survives a restart.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub boards: Vec<BoardRecord>,
    pub exams: Vec<Exam>,
    pub activity: Vec<Event>,
    pub next_event_id: u64,
    pub next_job_id: u64,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: u64,
    pub kind: JobKind,
    pub exam: Option<ExamPayload>,
    pub queued_at: Instant,
    pub picked_up_at: Option<Instant>,
}

#[derive(Debug, Clone)]
struct Live {
    last_seen: Instant,
    hb: Heartbeat,
    job: Option<Job>,
    error: Option<String>,
    temps: VecDeque<f32>,
    offline_noted: bool,
    exam_name: Option<String>,
}

#[derive(Debug, Clone)]
struct Board {
    rec: BoardRecord,
    /// `None` until the first heartbeat after a server restart.
    live: Option<Live>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HeartbeatError {
    InvalidHost,
    TooManyBoards,
}

pub struct Fleet {
    boards: BTreeMap<String, Board>,
    exams: Vec<Exam>,
    log: VecDeque<Event>,
    next_event_id: u64,
    next_job_id: u64,
    pub max_boards: usize,
    /// Set when something that is persisted changed.
    pub dirty: bool,
    cap_warned: bool,
}

pub fn valid_host(host: &str) -> bool {
    (1..=64).contains(&host.len())
        && host.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_'))
        && !host.starts_with(['-', '.'])
}

fn fmt_ago(d: Duration) -> String {
    let s = d.as_secs();
    match s {
        0..=59 => format!("{s} s ago"),
        60..=3599 => format!("{} min ago", s / 60),
        _ => format!("{} h ago", s / 3600),
    }
}

impl Fleet {
    pub fn new(max_boards: usize) -> Self {
        Fleet {
            boards: BTreeMap::new(),
            exams: Vec::new(),
            log: VecDeque::new(),
            next_event_id: 1,
            next_job_id: 1,
            max_boards,
            dirty: false,
            cap_warned: false,
        }
    }

    pub fn from_snapshot(s: Snapshot, max_boards: usize) -> Self {
        let mut f = Fleet::new(max_boards);
        for rec in s.boards.into_iter().filter(|r| valid_host(&r.host)) {
            f.boards.insert(rec.host.clone(), Board { rec, live: None });
        }
        f.exams = s.exams;
        f.log = s.activity.into_iter().collect();
        f.next_event_id = s.next_event_id.max(f.log.back().map_or(0, |e| e.id) + 1);
        f.next_job_id = s.next_job_id.max(1);
        f
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            version: 1,
            boards: self.boards.values().map(|b| b.rec.clone()).collect(),
            exams: self.exams.clone(),
            activity: self.log.iter().cloned().collect(),
            next_event_id: self.next_event_id,
            next_job_id: self.next_job_id,
        }
    }

    pub fn log(&mut self, wall_ms: i64, actor: Actor, level: Level, host: Option<&str>, text: impl Into<String>) {
        let ev = Event {
            id: self.next_event_id,
            at_ms: wall_ms,
            actor,
            level,
            host: host.map(str::to_owned),
            text: text.into(),
        };
        self.next_event_id += 1;
        self.log.push_back(ev);
        while self.log.len() > MAX_EVENTS {
            self.log.pop_front();
        }
        self.dirty = true;
    }

    // ----- agent side -------------------------------------------------------

    pub fn heartbeat(&mut self, hb: Heartbeat, now: Instant, wall_ms: i64) -> Result<HeartbeatReply, HeartbeatError> {
        if !valid_host(&hb.host) {
            return Err(HeartbeatError::InvalidHost);
        }
        let host = hb.host.clone();
        if !self.boards.contains_key(&host) {
            if self.boards.len() >= self.max_boards {
                if !self.cap_warned {
                    self.cap_warned = true;
                    let max = self.max_boards;
                    self.log(wall_ms, Actor::System, Level::Warn, Some(&host),
                        format!("Board limit of {max} reached; ignoring new boards. Forget unused boards or raise EXAM_MAX_BOARDS."));
                }
                return Err(HeartbeatError::TooManyBoards);
            }
            self.boards.insert(host.clone(), Board {
                rec: BoardRecord { host: host.clone(), label: None, first_seen_ms: wall_ms, last_seen_ms: wall_ms },
                live: None,
            });
            self.log(wall_ms, Actor::Board, Level::Info, Some(&host), "New board checked in");
        }

        let mut events: Vec<(Level, String)> = Vec::new();
        let board = self.boards.get_mut(&host).expect("inserted above");
        board.rec.last_seen_ms = wall_ms;

        let live = match board.live.as_mut() {
            Some(live) => {
                if live.offline_noted || now.duration_since(live.last_seen) > OFFLINE_AFTER {
                    events.push((Level::Info, "Back online".into()));
                }
                live
            }
            None => board.live.insert(Live {
                last_seen: now,
                hb: Heartbeat::default(),
                job: None,
                error: None,
                temps: VecDeque::new(),
                offline_noted: false,
                exam_name: None,
            }),
        };
        live.offline_noted = false;
        live.last_seen = now;
        let was_running = live.hb.state == state::RUNNING;
        let is_running = hb.state == state::RUNNING;
        if let Some(t) = hb.cpu_temp_c.filter(|t| t.is_finite()) {
            live.temps.push_back(t);
            while live.temps.len() > TEMP_SAMPLES {
                live.temps.pop_front();
            }
        }

        // Explicit job outcome from new agents.
        if let (Some(res), Some(job)) = (&hb.job_result, &live.job) {
            if res.id == job.id && !res.ok {
                let msg = res.message.clone().unwrap_or_else(|| "agent reported failure".into());
                let msg = truncate(&msg, 200);
                events.push((Level::Error, format!("{} failed: {msg}", cap(job.kind.as_str()))));
                live.error = Some(msg);
                live.job = None;
            }
        }

        // Job completion, judged from the reported state (works for old agents).
        let mut finished = None;
        if let Some(job) = &live.job {
            match job.kind {
                JobKind::Start if is_running => finished = Some(job.clone()),
                JobKind::Stop if !is_running && job.picked_up_at.is_some() => finished = Some(job.clone()),
                _ => {}
            }
        }
        if let Some(job) = finished {
            live.job = None;
            live.error = None;
            match job.kind {
                JobKind::Start => {
                    let name = job.exam.as_ref().map(|e| e.name.clone());
                    events.push((Level::Info, match &name {
                        Some(n) => format!("Exam running: {n}"),
                        None => "Exam running".into(),
                    }));
                    live.exam_name = name;
                }
                JobKind::Stop => {
                    events.push((Level::Info, "Exam stopped".into()));
                    live.exam_name = None;
                }
            }
        } else if was_running && !is_running && live.job.is_none() {
            // SEB ended without a stop job: student quit, crash, or reboot.
            live.exam_name = None;
            match hb.last_exit_code {
                Some(code) if code != 0 => {
                    let msg = format!("SEB exited unexpectedly with code {code}");
                    events.push((Level::Error, msg.clone()));
                    live.error = Some(msg);
                }
                Some(_) => events.push((Level::Info, "SEB session ended (exit code 0)".into())),
                None => events.push((Level::Warn, "SEB session ended without a stop command".into())),
            }
        }
        if is_running && live.exam_name.is_none() {
            live.exam_name = hb.exam_id.as_ref().and_then(|id| {
                self.exams.iter().find(|e| &e.id == id).map(|e| e.name.clone())
            });
        }

        live.hb = hb;

        // Deliver a waiting job.
        let mut reply = HeartbeatReply { ok: true, ..Default::default() };
        if let Some(job) = live.job.as_mut() {
            if job.picked_up_at.is_none() {
                job.picked_up_at = Some(now);
                reply.job = Some(job.kind);
                reply.job_id = Some(job.id);
                reply.exam = job.exam.clone();
            }
        }

        for (level, text) in events {
            self.log(wall_ms, Actor::Board, level, Some(&host), text);
        }
        Ok(reply)
    }

    /// Expire stale jobs and notice boards going offline.
    pub fn tick(&mut self, now: Instant, wall_ms: i64) {
        let mut events: Vec<(String, Level, String)> = Vec::new();
        for (host, b) in self.boards.iter_mut() {
            let Some(live) = b.live.as_mut() else { continue };
            let offline = now.duration_since(live.last_seen) > OFFLINE_AFTER;
            if offline && !live.offline_noted {
                live.offline_noted = true;
                events.push((host.clone(), Level::Warn, "Went offline (no heartbeat for 15 s)".into()));
            }
            let Some(job) = &live.job else { continue };
            let verb = cap(job.kind.as_str());
            match job.picked_up_at {
                None if now.duration_since(job.queued_at) > PICKUP_TTL => {
                    events.push((host.clone(), Level::Warn,
                        format!("{verb} expired: board did not pick it up within {} s", PICKUP_TTL.as_secs())));
                    live.job = None;
                }
                Some(t) if job.kind == JobKind::Start && now.duration_since(t) > START_TIMEOUT => {
                    let msg = format!("Start timed out: SEB not running {} s after pickup", START_TIMEOUT.as_secs());
                    events.push((host.clone(), Level::Error, msg.clone()));
                    live.error = Some(msg);
                    live.job = None;
                }
                Some(t) if job.kind == JobKind::Stop && now.duration_since(t) > STOP_TIMEOUT => {
                    let msg = format!("Stop timed out: SEB still running {} s after pickup", STOP_TIMEOUT.as_secs());
                    events.push((host.clone(), Level::Error, msg.clone()));
                    live.error = Some(msg);
                    live.job = None;
                }
                _ => {}
            }
        }
        for (host, level, text) in events {
            let actor = if level == Level::Warn && text.starts_with("Went offline") { Actor::Board } else { Actor::System };
            self.log(wall_ms, actor, level, Some(&host), text);
        }
    }

    // ----- instructor side --------------------------------------------------

    pub fn enqueue(&mut self, hosts: &[String], kind: JobKind, exam_id: Option<&str>, now: Instant, wall_ms: i64) -> Result<ActionReport, String> {
        let exam = match kind {
            JobKind::Start => {
                let id = exam_id.filter(|s| !s.is_empty()).ok_or("Choose an exam to start")?;
                let exam = self.exams.iter().find(|e| e.id == id).ok_or("That exam no longer exists")?;
                let config = proto::seb::seb_config(&exam.spec)?;
                Some(ExamPayload { id: exam.id.clone(), name: exam.name.clone(), config })
            }
            JobKind::Stop => None,
        };
        let mut report = ActionReport {
            verb: match kind { JobKind::Start => "Start", JobKind::Stop => "Stop" }.into(),
            queued: vec![],
            skipped: vec![],
            notes: vec![],
        };
        let mut seen = std::collections::HashSet::new();
        let mut events = Vec::new();
        for host in hosts.iter().take(self.max_boards) {
            if !seen.insert(host.as_str()) {
                continue;
            }
            let Some(b) = self.boards.get_mut(host) else {
                report.skipped.push((host.clone(), "Unknown board".into()));
                continue;
            };
            let Some(live) = b.live.as_mut() else {
                report.skipped.push((host.clone(), "Offline: not seen since the server restarted".into()));
                continue;
            };
            let since = now.duration_since(live.last_seen);
            if since > OFFLINE_AFTER {
                report.skipped.push((host.clone(), format!("Offline: last seen {}", fmt_ago(since))));
                continue;
            }
            let running = live.hb.state == state::RUNNING;
            let current = live.job.as_ref().map(|j| (j.kind, j.picked_up_at.is_some()));
            let skip = match (kind, current) {
                (JobKind::Start, Some((JobKind::Start, _))) => Some("Already starting"),
                (JobKind::Start, Some((JobKind::Stop, _))) => Some("Still stopping; try again in a moment"),
                (JobKind::Start, None) if running => Some("SEB is already running"),
                (JobKind::Stop, Some((JobKind::Stop, _))) => Some("Already stopping"),
                (JobKind::Stop, None) if !running && live.error.is_none() => Some("SEB is not running"),
                _ => None,
            };
            if let Some(reason) = skip {
                report.skipped.push((host.clone(), reason.into()));
                continue;
            }
            if kind == JobKind::Stop && current == Some((JobKind::Start, false)) {
                // The board never saw the start; cancelling is enough.
                live.job = None;
                report.queued.push(host.clone());
                report.notes.push((host.clone(), "Start cancelled before the board picked it up".into()));
                events.push((host.clone(), "Start cancelled".to_string()));
                continue;
            }
            live.error = None;
            live.job = Some(Job { id: self.next_job_id, kind, exam: exam.clone(), queued_at: now, picked_up_at: None });
            self.next_job_id += 1;
            if kind == JobKind::Start && live.hb.protocol.unwrap_or(1) < 2 {
                report.notes.push((host.clone(), "Old agent: it will run its local config, not this exam".into()));
            }
            report.queued.push(host.clone());
            events.push((host.clone(), match &exam {
                Some(e) => format!("Start queued: {}", e.name),
                None => "Stop queued".into(),
            }));
        }
        for (host, text) in events {
            self.log(wall_ms, Actor::Instructor, Level::Info, Some(&host), text);
        }
        if !report.queued.is_empty() || !report.skipped.is_empty() {
            let what = match &exam {
                Some(e) => format!("Start \u{201c}{}\u{201d}", e.name),
                None => "Stop".into(),
            };
            let n = report.queued.len();
            let mut text = format!("{what} on {n} board{}", if n == 1 { "" } else { "s" });
            if !report.skipped.is_empty() {
                text.push_str(&format!(", {} skipped", report.skipped.len()));
            }
            self.log(wall_ms, Actor::Instructor, Level::Info, None, text);
        }
        Ok(report)
    }

    pub fn rename(&mut self, host: &str, label: &str, wall_ms: i64) -> Result<(), String> {
        let label = label.trim();
        if label.chars().count() > MAX_LABEL {
            return Err(format!("Names can be at most {MAX_LABEL} characters"));
        }
        if label.chars().any(char::is_control) {
            return Err("Names cannot contain control characters".into());
        }
        let b = self.boards.get_mut(host).ok_or("Unknown board")?;
        let new = (!label.is_empty() && label != host).then(|| label.to_string());
        if b.rec.label == new {
            return Ok(());
        }
        b.rec.label = new.clone();
        let text = match new {
            Some(n) => format!("Renamed to \u{201c}{n}\u{201d}"),
            None => "Name cleared".into(),
        };
        self.log(wall_ms, Actor::Instructor, Level::Info, Some(host), text);
        Ok(())
    }

    pub fn forget(&mut self, host: &str, wall_ms: i64) -> Result<(), String> {
        self.boards.remove(host).ok_or("Unknown board")?;
        self.cap_warned = false;
        self.log(wall_ms, Actor::Instructor, Level::Info, Some(host), "Board forgotten");
        Ok(())
    }

    pub fn create_exam(&mut self, id: String, name: &str, spec: ExamSpec, wall_ms: i64) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Give the exam a name".into());
        }
        if name.chars().count() > MAX_EXAM_NAME || name.chars().any(char::is_control) {
            return Err(format!("Exam names can be at most {MAX_EXAM_NAME} characters"));
        }
        if self.exams.len() >= MAX_EXAMS {
            return Err(format!("At most {MAX_EXAMS} exams; delete old ones first"));
        }
        let spec = ExamSpec {
            start_url: spec.start_url.trim().to_string(),
            allowed_domains: spec
                .allowed_domains
                .iter()
                .map(|d| proto::seb::normalize_domain(d))
                .collect::<Result<_, _>>()?,
            allow_quit: spec.allow_quit,
        };
        proto::seb::seb_config(&spec)?;
        self.exams.push(Exam { id: id.clone(), name: name.to_string(), spec, created_ms: wall_ms });
        self.log(wall_ms, Actor::Instructor, Level::Info, None, format!("Exam created: {name}"));
        Ok(id)
    }

    pub fn delete_exam(&mut self, id: &str, wall_ms: i64) -> Result<(), String> {
        let i = self.exams.iter().position(|e| e.id == id).ok_or("Unknown exam")?;
        let e = self.exams.remove(i);
        self.log(wall_ms, Actor::Instructor, Level::Info, None, format!("Exam deleted: {}", e.name));
        Ok(())
    }

    // ----- views --------------------------------------------------------------

    fn status_of(b: &Board, now: Instant) -> (Status, Option<String>) {
        let Some(live) = &b.live else { return (Status::Offline, None) };
        if now.duration_since(live.last_seen) > OFFLINE_AFTER {
            return (Status::Offline, None);
        }
        if let Some(job) = &live.job {
            let waiting = job.picked_up_at.is_none();
            return match job.kind {
                JobKind::Start => (Status::Starting, Some(if waiting { "Waiting for the board to pick up" } else { "Launching SEB" }.into())),
                JobKind::Stop => (Status::Stopping, Some(if waiting { "Waiting for the board to pick up" } else { "Closing SEB" }.into())),
            };
        }
        if let Some(e) = &live.error {
            return (Status::Error, Some(e.clone()));
        }
        if live.hb.state == state::RUNNING {
            (Status::Running, None)
        } else {
            (Status::Idle, None)
        }
    }

    fn board_view(b: &Board, now: Instant) -> BoardView {
        let (status, detail) = Self::status_of(b, now);
        let live = b.live.as_ref();
        let hb = live.map(|l| &l.hb);
        BoardView {
            host: b.rec.host.clone(),
            label: b.rec.label.clone(),
            status,
            detail,
            seen_secs: live.map(|l| now.duration_since(l.last_seen).as_secs()),
            cpu_temp_c: hb.and_then(|h| h.cpu_temp_c),
            load1: hb.and_then(|h| h.load.map(|l| l[0])),
            mem_used_pct: hb.and_then(|h| h.mem).filter(|m| m.total_kb > 0).map(|m| {
                (100 - (m.available_kb.min(m.total_kb) * 100 / m.total_kb)) as u8
            }),
            uptime_s: hb.and_then(|h| h.uptime_s),
            ip: hb.and_then(|h| h.ip.clone()),
            agent_version: hb.and_then(|h| h.agent_version.clone()),
            seb_version: hb.and_then(|h| h.seb_version.clone()),
            image_hash: hb.and_then(|h| h.image_hash.clone()),
            last_exit_code: hb.and_then(|h| h.last_exit_code),
            exam_name: live.and_then(|l| l.exam_name.clone()),
            legacy_agent: hb.is_some_and(|h| h.protocol.unwrap_or(1) < 2),
        }
    }

    pub fn exam_choices(&self) -> Vec<ExamChoice> {
        self.exams.iter().rev().map(|e| ExamChoice { id: e.id.clone(), name: e.name.clone() }).collect()
    }

    pub fn view(&self, now: Instant) -> FleetView {
        FleetView {
            boards: self.boards.values().map(|b| Self::board_view(b, now)).collect(),
            exams: self.exam_choices(),
            max_boards: self.max_boards,
        }
    }

    pub fn detail(&self, host: &str, now: Instant) -> Option<BoardDetail> {
        let b = self.boards.get(host)?;
        let hb = b.live.as_ref().map(|l| &l.hb);
        Some(BoardDetail {
            board: Self::board_view(b, now),
            first_seen_ms: b.rec.first_seen_ms,
            mem_total_kb: hb.and_then(|h| h.mem.map(|m| m.total_kb)),
            mem_available_kb: hb.and_then(|h| h.mem.map(|m| m.available_kb)),
            load: hb.and_then(|h| h.load),
            temp_history: b.live.as_ref().map(|l| l.temps.iter().copied().collect()).unwrap_or_default(),
            history: self.events(Some(host), None, 100),
            exams: self.exam_choices(),
        })
    }

    /// Newest first.
    pub fn events(&self, host: Option<&str>, actor: Option<Actor>, limit: usize) -> Vec<EventView> {
        self.log
            .iter()
            .rev()
            .filter(|e| host.is_none_or(|h| e.host.as_deref() == Some(h)))
            .filter(|e| actor.is_none_or(|a| e.actor == a))
            .take(limit)
            .map(|e| EventView { id: e.id, at_ms: e.at_ms, actor: e.actor, level: e.level, host: e.host.clone(), text: e.text.clone() })
            .collect()
    }

    pub fn exam_views(&self) -> Vec<ExamView> {
        self.exams
            .iter()
            .rev()
            .map(|e| ExamView {
                id: e.id.clone(),
                name: e.name.clone(),
                start_url: e.spec.start_url.clone(),
                allowed_domains: e.spec.allowed_domains.clone(),
                allow_quit: e.spec.allow_quit,
                created_ms: e.created_ms,
                config_json: proto::seb::seb_config(&e.spec)
                    .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
                    .unwrap_or_else(|err| format!("// invalid: {err}")),
            })
            .collect()
    }

    pub fn board_count(&self) -> usize {
        self.boards.len()
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

fn truncate(s: &str, max: usize) -> String {
    let clean: String = s.chars().filter(|c| !c.is_control()).collect();
    if clean.chars().count() <= max {
        clean
    } else {
        clean.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::JobResult;

    const T0: i64 = 1_700_000_000_000;

    fn hb(host: &str, st: &str) -> Heartbeat {
        Heartbeat { host: host.into(), state: st.into(), ..Default::default() }
    }

    fn hb2(host: &str, st: &str) -> Heartbeat {
        Heartbeat { protocol: Some(2), ..hb(host, st) }
    }

    fn fleet_with_exam() -> (Fleet, String) {
        let mut f = Fleet::new(8);
        let spec = ExamSpec { start_url: "https://ufl.instructure.com/courses/1/quizzes/2".into(), allowed_domains: vec!["ufl.edu".into()], allow_quit: false };
        let id = f.create_exam("e1".into(), "Quiz 1", spec, T0).unwrap();
        (f, id)
    }

    fn status(f: &Fleet, host: &str, now: Instant) -> Status {
        f.view(now).boards.into_iter().find(|b| b.host == host).unwrap().status
    }

    #[test]
    fn full_start_stop_lifecycle() {
        let (mut f, exam) = fleet_with_exam();
        let t = Instant::now();
        f.heartbeat(hb2("b1", "idle"), t, T0).unwrap();
        assert_eq!(status(&f, "b1", t), Status::Idle);

        let r = f.enqueue(&["b1".into()], JobKind::Start, Some(&exam), t, T0).unwrap();
        assert_eq!(r.queued, vec!["b1"]);
        assert_eq!(status(&f, "b1", t), Status::Starting);

        // Pickup: the reply carries the job and the generated config.
        let reply = f.heartbeat(hb2("b1", "idle"), t + Duration::from_secs(1), T0).unwrap();
        assert_eq!(reply.job, Some(JobKind::Start));
        let payload = reply.exam.unwrap();
        assert_eq!(payload.name, "Quiz 1");
        assert_eq!(payload.config["browser"]["startUrl"], "https://ufl.instructure.com/courses/1/quizzes/2");
        // Delivered only once.
        let again = f.heartbeat(hb2("b1", "idle"), t + Duration::from_secs(2), T0).unwrap();
        assert_eq!(again.job, None);
        assert_eq!(status(&f, "b1", t + Duration::from_secs(2)), Status::Starting);

        f.heartbeat(hb2("b1", "running"), t + Duration::from_secs(5), T0).unwrap();
        let v = f.view(t + Duration::from_secs(5));
        assert_eq!(v.boards[0].status, Status::Running);
        assert_eq!(v.boards[0].exam_name.as_deref(), Some("Quiz 1"));

        f.enqueue(&["b1".into()], JobKind::Stop, None, t + Duration::from_secs(6), T0).unwrap();
        assert_eq!(status(&f, "b1", t + Duration::from_secs(6)), Status::Stopping);
        let reply = f.heartbeat(hb2("b1", "running"), t + Duration::from_secs(7), T0).unwrap();
        assert_eq!(reply.job, Some(JobKind::Stop));
        assert!(reply.exam.is_none());
        f.heartbeat(hb2("b1", "idle"), t + Duration::from_secs(9), T0).unwrap();
        assert_eq!(status(&f, "b1", t + Duration::from_secs(9)), Status::Idle);
        assert!(f.events(Some("b1"), None, 10).iter().any(|e| e.text == "Exam stopped"));
    }

    #[test]
    fn offline_and_busy_boards_are_skipped_with_reasons() {
        let (mut f, exam) = fleet_with_exam();
        let t = Instant::now();
        f.heartbeat(hb("old", "idle"), t, T0).unwrap();
        f.heartbeat(hb("run", "running"), t + Duration::from_secs(30), T0).unwrap();
        f.heartbeat(hb("ok", "idle"), t + Duration::from_secs(30), T0).unwrap();
        let now = t + Duration::from_secs(31);
        let hosts: Vec<String> = ["old", "run", "ok", "ghost", "ok"].map(String::from).to_vec();
        let r = f.enqueue(&hosts, JobKind::Start, Some(&exam), now, T0).unwrap();
        assert_eq!(r.queued, vec!["ok"]);
        let reasons: BTreeMap<_, _> = r.skipped.iter().cloned().collect();
        assert_eq!(reasons["old"], "Offline: last seen 31 s ago");
        assert_eq!(reasons["run"], "SEB is already running");
        assert_eq!(reasons["ghost"], "Unknown board");
        // Legacy agent gets a note that it ignores the exam config.
        assert!(r.notes.iter().any(|(h, n)| h == "ok" && n.contains("Old agent")));

        let r = f.enqueue(&["ok".into()], JobKind::Start, Some(&exam), now, T0).unwrap();
        assert_eq!(r.skipped, vec![("ok".into(), "Already starting".into())]);
        assert!(f.enqueue(&["ok".into()], JobKind::Start, None, now, T0).is_err());
    }

    #[test]
    fn stale_jobs_expire() {
        let (mut f, exam) = fleet_with_exam();
        let t = Instant::now();
        f.heartbeat(hb("b", "idle"), t, T0).unwrap();
        f.enqueue(&["b".into()], JobKind::Start, Some(&exam), t, T0).unwrap();
        // Board stops checking in; the pickup TTL drops the job.
        f.tick(t + PICKUP_TTL + Duration::from_secs(1), T0);
        let now = t + PICKUP_TTL + Duration::from_secs(1);
        assert_eq!(status(&f, "b", now), Status::Offline);
        let ev = f.events(Some("b"), None, 10);
        assert!(ev.iter().any(|e| e.text.starts_with("Start expired")), "{ev:?}");
        assert!(ev.iter().any(|e| e.text.starts_with("Went offline")));
    }

    #[test]
    fn start_that_never_runs_times_out_into_error() {
        let (mut f, exam) = fleet_with_exam();
        let mut t = Instant::now();
        f.heartbeat(hb("b", "idle"), t, T0).unwrap();
        f.enqueue(&["b".into()], JobKind::Start, Some(&exam), t, T0).unwrap();
        assert_eq!(f.heartbeat(hb("b", "idle"), t, T0).unwrap().job, Some(JobKind::Start));
        for _ in 0..20 {
            t += Duration::from_secs(5);
            f.heartbeat(hb("b", "idle"), t, T0).unwrap();
            f.tick(t, T0);
        }
        let v = f.view(t);
        assert_eq!(v.boards[0].status, Status::Error);
        assert!(v.boards[0].detail.as_deref().unwrap().starts_with("Start timed out"));
        // A new start clears the error.
        f.enqueue(&["b".into()], JobKind::Start, Some(&exam), t, T0).unwrap();
        assert_eq!(status(&f, "b", t), Status::Starting);
    }

    #[test]
    fn agent_reported_failure_and_crash_become_errors() {
        let (mut f, exam) = fleet_with_exam();
        let t = Instant::now();
        f.heartbeat(hb2("b", "idle"), t, T0).unwrap();
        f.enqueue(&["b".into()], JobKind::Start, Some(&exam), t, T0).unwrap();
        let id = f.heartbeat(hb2("b", "idle"), t, T0).unwrap().job_id.unwrap();
        let fail = Heartbeat {
            job_result: Some(JobResult { id, ok: false, message: Some("SEB exited with code 1".into()) }),
            last_exit_code: Some(1),
            ..hb2("b", "idle")
        };
        f.heartbeat(fail, t + Duration::from_secs(5), T0).unwrap();
        let v = f.view(t + Duration::from_secs(5));
        assert_eq!(v.boards[0].status, Status::Error);
        assert_eq!(v.boards[0].detail.as_deref(), Some("SEB exited with code 1"));

        // Running, then SEB dies with no stop job.
        f.heartbeat(hb2("b", "running"), t + Duration::from_secs(10), T0).unwrap();
        f.heartbeat(Heartbeat { last_exit_code: Some(139), ..hb2("b", "idle") }, t + Duration::from_secs(15), T0).unwrap();
        let v = f.view(t + Duration::from_secs(15));
        assert_eq!(v.boards[0].status, Status::Error);
        assert!(v.boards[0].detail.as_deref().unwrap().contains("139"));
        // Stop is allowed on an errored board even though SEB is not running.
        let r = f.enqueue(&["b".into()], JobKind::Stop, None, t + Duration::from_secs(15), T0).unwrap();
        assert_eq!(r.queued, vec!["b"]);
    }

    #[test]
    fn stop_cancels_unpicked_start() {
        let (mut f, exam) = fleet_with_exam();
        let t = Instant::now();
        f.heartbeat(hb("b", "idle"), t, T0).unwrap();
        f.enqueue(&["b".into()], JobKind::Start, Some(&exam), t, T0).unwrap();
        let r = f.enqueue(&["b".into()], JobKind::Stop, None, t, T0).unwrap();
        assert_eq!(r.queued, vec!["b"]);
        assert_eq!(status(&f, "b", t), Status::Idle);
        assert_eq!(f.heartbeat(hb("b", "idle"), t, T0).unwrap().job, None);
    }

    #[test]
    fn host_validation_and_board_cap() {
        let mut f = Fleet::new(2);
        let t = Instant::now();
        for bad in ["", "<script>", "a b", "-x", "x/../y", &"a".repeat(65)] {
            assert_eq!(f.heartbeat(hb(bad, "idle"), t, T0), Err(HeartbeatError::InvalidHost), "{bad:?}");
        }
        f.heartbeat(hb("a", "idle"), t, T0).unwrap();
        f.heartbeat(hb("b", "idle"), t, T0).unwrap();
        assert_eq!(f.heartbeat(hb("c", "idle"), t, T0), Err(HeartbeatError::TooManyBoards));
        // Known boards still work at the cap.
        assert!(f.heartbeat(hb("a", "idle"), t, T0).is_ok());
        f.forget("b", T0).unwrap();
        assert!(f.heartbeat(hb("c", "idle"), t, T0).is_ok());
    }

    #[test]
    fn snapshot_roundtrip_keeps_names_exams_and_log() {
        let (mut f, _) = fleet_with_exam();
        let t = Instant::now();
        f.heartbeat(hb("b", "idle"), t, T0).unwrap();
        f.rename("b", "  Row 1, seat 3 ", T0).unwrap();
        let snap = f.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back: Snapshot = serde_json::from_str(&json).unwrap();
        let g = Fleet::from_snapshot(back, 8);
        let v = g.view(t);
        assert_eq!(v.boards[0].label.as_deref(), Some("Row 1, seat 3"));
        assert_eq!(v.boards[0].status, Status::Offline);
        assert_eq!(g.exam_views().len(), 1);
        assert_eq!(g.events(None, None, 100).len(), f.events(None, None, 100).len());
    }

    #[test]
    fn rename_rules() {
        let mut f = Fleet::new(4);
        f.heartbeat(hb("b", "idle"), Instant::now(), T0).unwrap();
        assert!(f.rename("b", &"x".repeat(49), T0).is_err());
        assert!(f.rename("b", "a\u{0007}", T0).is_err());
        f.rename("b", "b", T0).unwrap();
        assert_eq!(f.view(Instant::now()).boards[0].label, None);
    }
}
