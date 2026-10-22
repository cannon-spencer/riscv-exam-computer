mod exam;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Machine {
    last_seen: Instant,
    state: String,
    pending: Option<String>,
}

#[derive(Clone)]
struct AppState {
    machines: Db,
    exam: Arc<Mutex<Option<exam::ExamFile>>>,
}

type Db = Arc<Mutex<HashMap<String, Machine>>>;

const ONLINE: Duration = Duration::from_secs(15);

#[derive(Deserialize)]
struct Heartbeat {
    host: String,
    #[serde(default)]
    state: String,
}

#[derive(Serialize)]
struct MachineView {
    host: String,
    state: String,
    pending: Option<String>,
    seen_secs_ago: u64,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        machines: Arc::new(Mutex::new(HashMap::new())),
        exam: Arc::new(Mutex::new(None)),
    };
    let app = Router::new()
        .route("/heartbeat", post(heartbeat))
        .route("/machines", get(machines))
        .route("/start", post(enqueue_start))
        .route("/stop", post(enqueue_stop))
        .route("/exam.seb", get(exam::get).post(exam::put))
        .with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("listening on {addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn heartbeat(State(app): State<AppState>, Json(hb): Json<Heartbeat>) -> Json<Value> {
    let job = {
        let mut map = app.machines.lock().unwrap();
        let m = map.entry(hb.host.clone()).or_insert(Machine {
            last_seen: Instant::now(),
            state: String::new(),
            pending: None,
        });
        m.last_seen = Instant::now();
        m.state.clone_from(&hb.state);
        let take = match m.pending.as_deref() {
            Some("start") if hb.state != "running" => true,
            Some("stop") => true,
            _ => false,
        };
        if take {
            m.pending.take()
        } else {
            None
        }
    };
    println!(
        "heartbeat host={} state={} job={}",
        hb.host,
        hb.state,
        job.as_deref().unwrap_or("-")
    );
    Json(json!({ "ok": true, "job": job }))
}

async fn machines(State(app): State<AppState>) -> Json<Vec<MachineView>> {
    let map = app.machines.lock().unwrap();
    let now = Instant::now();
    let mut rows: Vec<_> = map
        .iter()
        .map(|(host, m)| MachineView {
            host: host.clone(),
            state: m.state.clone(),
            pending: m.pending.clone(),
            seen_secs_ago: now.saturating_duration_since(m.last_seen).as_secs(),
        })
        .collect();
    rows.sort_by(|a, b| a.host.cmp(&b.host));
    Json(rows)
}

async fn enqueue_start(State(app): State<AppState>) -> Json<Value> {
    Json(json!({ "ok": true, "queued": enqueue(&app.machines, "start") }))
}

async fn enqueue_stop(State(app): State<AppState>) -> Json<Value> {
    Json(json!({ "ok": true, "queued": enqueue(&app.machines, "stop") }))
}

fn enqueue(db: &Db, job: &str) -> Vec<String> {
    let mut map = db.lock().unwrap();
    let now = Instant::now();
    let mut queued = Vec::new();
    for (host, m) in map.iter_mut() {
        if now.saturating_duration_since(m.last_seen) > ONLINE {
            continue;
        }
        m.pending = Some(job.to_string());
        queued.push(host.clone());
    }
    queued.sort();
    println!("{job} queued {:?}", queued);
    queued
}
