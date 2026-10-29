//! Board daemon: heartbeats the exam server and runs SEB on request.
//!
//! Protocol 3: a `start` job carries the exam's standard `.seb` file. The
//! agent writes the plain XML plist to `~/exam.seb` (atomically) and
//! `start-seb.sh` opens it, so SEB sends that exam's Config Key hashes to
//! the server's gate. This SEB build stops after the `plnd` marker and does
//! not open the gzip-wrapped settings inside a standard file. A start job
//! without a `.seb` (older server) falls back to whatever `~/exam.seb`
//! already holds.

mod stats;

use std::fs;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use base64::Engine;
use proto::{ExamPayload, Heartbeat, HeartbeatReply, JobKind, JobResult, PROTOCOL_VERSION, state};

const DEFAULT_SERVER: &str = "https://riscv-exam-computer.download";

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/home/orangepi".into())
}

fn script(name: &str) -> String {
    format!("{}/{name}", home())
}

fn exam_path() -> String {
    format!("{}/exam.seb", home())
}

/// Whether the SEB session is still alive. When it has ended, records the
/// start script's exit code in `last_exit` for the heartbeat.
fn running(child: &mut Option<Child>, last_exit: &mut Option<i32>) -> bool {
    let Some(c) = child.as_mut() else { return false };
    match c.try_wait() {
        Ok(None) => true,
        r => {
            if let Ok(Some(st)) = r {
                *last_exit = st.code();
            }
            *child = None;
            false
        }
    }
}

/// Plain XML plist. The board's SEB opens that, and a standard `.seb` is
/// gzip(`plnd` + gzip(xml)).
fn seb_on_disk(bytes: &[u8]) -> Result<Vec<u8>, String> {
    proto::seb_file::container::decode(bytes, None)
}

/// Writes the exam's .seb next to the scripts. Temp file + rename, so SEB
/// never reads a half-written file.
fn install_exam(exam: &ExamPayload) -> Result<(), String> {
    let Some(b64) = &exam.seb else { return Ok(()) };
    let raw = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|e| format!("bad .seb payload: {e}"))?;
    let bytes = seb_on_disk(&raw)?;
    let path = exam_path();
    let tmp = format!("{path}.tmp");
    fs::write(&tmp, &bytes).and_then(|_| fs::rename(&tmp, &path)).map_err(|e| format!("writing {path}: {e}"))?;
    println!(
        "exam {} installed ({} bytes, config key {})",
        exam.id,
        bytes.len(),
        exam.config_key.as_deref().unwrap_or("?")
    );
    Ok(())
}

fn start_seb(child: &mut Option<Child>, last_exit: &mut Option<i32>) -> Result<(), String> {
    if running(child, last_exit) {
        return Ok(());
    }
    let s = script("start-seb.sh");
    let c = Command::new("sudo").args(["-n", &s]).stdin(Stdio::null()).spawn().map_err(|e| format!("start seb: {e}"))?;
    println!("started {s}");
    *child = Some(c);
    Ok(())
}

fn stop_seb(child: &mut Option<Child>, last_exit: &mut Option<i32>) -> Result<(), String> {
    let s = script("stop-seb.sh");
    let r = match Command::new("sudo").args(["-n", &s]).status() {
        Ok(st) if st.success() => Ok(()),
        Ok(st) => Err(format!("stop script exited with {st}")),
        Err(e) => Err(format!("stop seb: {e}")),
    };
    if let Some(mut c) = child.take() {
        if let Ok(st) = c.wait() {
            *last_exit = st.code();
        }
    }
    r
}

fn main() {
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into());
    let base = std::env::var("EXAM_SERVER").unwrap_or_else(|_| DEFAULT_SERVER.into());
    let url = format!("{}/heartbeat", base.trim_end_matches('/'));
    let token = std::env::var("EXAM_AGENT_TOKEN").ok().filter(|t| !t.is_empty());

    let mut seb = None;
    let mut exam_id: Option<String> = None;
    let mut job_result: Option<JobResult> = None;
    let mut last_exit: Option<i32> = None;

    loop {
        let is_running = running(&mut seb, &mut last_exit);
        if !is_running {
            exam_id = None;
        }
        let hb = Heartbeat {
            host: host.clone(),
            state: if is_running { state::RUNNING } else { state::IDLE }.into(),
            protocol: Some(PROTOCOL_VERSION),
            agent_version: Some(env!("CARGO_PKG_VERSION").into()),
            ip: stats::ip(),
            cpu_temp_c: stats::cpu_temp_c(),
            load: stats::load(),
            mem: stats::mem(),
            uptime_s: stats::uptime_s(),
            last_exit_code: last_exit,
            exam_id: exam_id.clone(),
            job_result: job_result.take(),
            ..Default::default()
        };
        let body = serde_json::to_string(&hb).expect("heartbeat serializes");

        let mut req = ureq::post(&url).header("Content-Type", "application/json");
        if let Some(t) = &token {
            req = req.header("Authorization", format!("Bearer {t}"));
        }
        let reply = req
            .send(&body)
            .map_err(|e| e.to_string())
            .and_then(|r| r.into_body().read_to_string().map_err(|e| e.to_string()))
            .and_then(|t| serde_json::from_str::<HeartbeatReply>(&t).map_err(|e| e.to_string()));

        match reply {
            Ok(r) => {
                let outcome = match r.job {
                    Some(JobKind::Start) => r
                        .exam
                        .as_ref()
                        .map_or(Ok(()), install_exam)
                        .and_then(|_| start_seb(&mut seb, &mut last_exit))
                        .map(|_| exam_id = r.exam.as_ref().map(|e| e.id.clone())),
                    Some(JobKind::Stop) => stop_seb(&mut seb, &mut last_exit),
                    None => Ok(()),
                };
                if let (Some(id), Some(_)) = (r.job_id, r.job) {
                    if let Err(e) = &outcome {
                        eprintln!("job {id} failed: {e}");
                    }
                    job_result = Some(JobResult { id, ok: outcome.is_ok(), message: outcome.err() });
                }
            }
            Err(e) => eprintln!("heartbeat failed: {e}"),
        }

        thread::sleep(Duration::from_secs(5));
    }
}

#[cfg(test)]
mod tests {
    use super::seb_on_disk;

    #[test]
    fn standard_seb_is_written_as_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?><plist></plist>"#;
        let wrapped = proto::seb_file::container::encode(xml, None);
        assert!(wrapped.starts_with(&[0x1f, 0x8b]));
        assert_eq!(seb_on_disk(&wrapped).unwrap(), xml.as_bytes());
        assert_eq!(seb_on_disk(xml.as_bytes()).unwrap(), xml.as_bytes());
    }
}
