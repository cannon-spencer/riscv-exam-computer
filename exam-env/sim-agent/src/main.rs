//! Fake exam boards. Each board is a thread that sends heartbeats like
//! `seb-agent` and pretends to start and stop SEB with realistic delays.
//!
//!     cargo run -p sim-agent -- --count 12 --chaos 0.1
//!
//! Options:
//!   --server URL     exam server base URL (default http://127.0.0.1:8000)
//!   --count N        number of boards (default 12)
//!   --prefix P       host name prefix (default "sim-")
//!   --chaos P        0..1 chance of failures: failed starts, crashes, dropouts
//!   --legacy N       make the first N boards speak the old {"host","state"} protocol
//!   --interval S     seconds between heartbeats (default 5, like seb-agent)
//! EXAM_AGENT_TOKEN is sent as a bearer token when set.

use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use proto::{Heartbeat, HeartbeatReply, JobKind, JobResult, MemStats, state};

struct Opts {
    server: String,
    count: usize,
    prefix: String,
    chaos: f64,
    legacy: usize,
    interval: Duration,
    token: Option<String>,
}

fn parse_args() -> Opts {
    let mut o = Opts {
        server: "http://127.0.0.1:8000".into(),
        count: 12,
        prefix: "sim-".into(),
        chaos: 0.0,
        legacy: 0,
        interval: Duration::from_secs(5),
        token: std::env::var("EXAM_AGENT_TOKEN").ok().filter(|t| !t.is_empty()),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().unwrap_or_else(|| die(&format!("{a} needs a value")));
        match a.as_str() {
            "--server" => o.server = val().trim_end_matches('/').to_string(),
            "--count" => o.count = val().parse().unwrap_or_else(|_| die("--count must be a number")),
            "--prefix" => o.prefix = val(),
            "--chaos" => o.chaos = val().parse::<f64>().unwrap_or_else(|_| die("--chaos must be 0..1")).clamp(0.0, 1.0),
            "--legacy" => o.legacy = val().parse().unwrap_or_else(|_| die("--legacy must be a number")),
            "--interval" => o.interval = Duration::from_secs_f64(val().parse().unwrap_or_else(|_| die("--interval must be seconds"))),
            "-h" | "--help" => {
                println!("{}", include_str!("main.rs").lines().take_while(|l| l.starts_with("//!")).map(|l| l.trim_start_matches("//!").trim_start_matches(' ')).collect::<Vec<_>>().join("\n"));
                std::process::exit(0);
            }
            other => die(&format!("unknown argument {other}")),
        }
    }
    o
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(2)
}

/// xorshift64*: good enough for jitter, no dependency.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
    fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }
}

enum Seb {
    Idle,
    Starting { done: Instant, job: u64, exam: Option<String>, fail: bool },
    Running { exam: Option<String> },
    Stopping { done: Instant, job: u64 },
}

struct Board {
    host: String,
    legacy: bool,
    rng: Rng,
    seb: Seb,
    temp: f64,
    load: [f64; 3],
    avail_kb: f64,
    boot: Instant,
    uptime0: u64,
    ip: String,
    last_exit: Option<i32>,
    job_result: Option<JobResult>,
    silent_until: Option<Instant>,
}

const MEM_TOTAL_KB: u64 = 4_012_000;

impl Board {
    fn new(i: usize, o: &Opts, seed: u64) -> Self {
        let mut rng = Rng::new(seed ^ (i as u64 + 1) * 7919);
        Board {
            host: format!("{}{:02}", o.prefix, i + 1),
            legacy: i < o.legacy,
            temp: rng.range(41.0, 49.0),
            load: [0.2, 0.2, 0.2],
            avail_kb: 3_200_000.0,
            boot: Instant::now(),
            uptime0: rng.range(300.0, 200_000.0) as u64,
            ip: format!("10.20.{}.{}", 30 + i / 200, 10 + i % 200),
            last_exit: None,
            job_result: None,
            silent_until: None,
            seb: Seb::Idle,
            rng,
        }
    }

    fn running(&self) -> bool {
        matches!(self.seb, Seb::Running { .. } | Seb::Stopping { .. })
    }

    /// Advance the simulated hardware and SEB by one tick.
    fn step(&mut self, chaos: f64) {
        let busy = self.running();
        let target = if busy { 63.0 } else { 46.0 };
        self.temp += (target - self.temp) * 0.15 + self.rng.range(-1.2, 1.2);
        self.temp = self.temp.clamp(32.0, 88.0);
        let l = if busy { self.rng.range(1.6, 3.4) } else { self.rng.range(0.05, 0.5) };
        self.load = [l, self.load[1] * 0.8 + l * 0.2, self.load[2] * 0.93 + l * 0.07];
        let mem_target = if busy { 1_500_000.0 } else { 3_150_000.0 };
        self.avail_kb += (mem_target - self.avail_kb) * 0.3 + self.rng.range(-40_000.0, 40_000.0);

        let now = Instant::now();
        self.seb = match std::mem::replace(&mut self.seb, Seb::Idle) {
            Seb::Starting { done, job, exam, fail } if now >= done => {
                if fail {
                    self.last_exit = Some(1);
                    self.job_result = Some(JobResult { id: job, ok: false, message: Some("SEB exited with code 1 before the exam opened".into()) });
                    println!("{}: start failed (chaos)", self.host);
                    Seb::Idle
                } else {
                    self.job_result = Some(JobResult { id: job, ok: true, message: None });
                    println!("{}: SEB running", self.host);
                    Seb::Running { exam }
                }
            }
            Seb::Running { .. } if self.rng.chance(chaos / 40.0) => {
                self.last_exit = Some(139);
                println!("{}: SEB crashed (chaos)", self.host);
                Seb::Idle
            }
            Seb::Stopping { done, job } if now >= done => {
                self.last_exit = Some(0);
                self.job_result = Some(JobResult { id: job, ok: true, message: None });
                println!("{}: SEB stopped", self.host);
                Seb::Idle
            }
            other => other,
        };

        if self.silent_until.is_none() && self.rng.chance(chaos / 60.0) {
            let secs = self.rng.range(20.0, 45.0);
            println!("{}: dropping off the network for {secs:.0} s (chaos)", self.host);
            self.silent_until = Some(now + Duration::from_secs_f64(secs));
        }
    }

    fn heartbeat(&mut self) -> Heartbeat {
        let st = if self.running() { state::RUNNING } else { state::IDLE }.to_string();
        if self.legacy {
            return Heartbeat { host: self.host.clone(), state: st, ..Default::default() };
        }
        Heartbeat {
            host: self.host.clone(),
            state: st,
            protocol: Some(proto::PROTOCOL_VERSION),
            agent_version: Some(format!("{}-sim", env!("CARGO_PKG_VERSION"))),
            seb_version: Some("sim-3.9.0".into()),
            image_hash: Some("sim-9f2c4e1a".into()),
            ip: Some(self.ip.clone()),
            cpu_temp_c: Some((self.temp * 10.0).round() as f32 / 10.0),
            load: Some(self.load.map(|l| (l * 100.0).round() as f32 / 100.0)),
            mem: Some(MemStats { total_kb: MEM_TOTAL_KB, available_kb: self.avail_kb.clamp(100_000.0, MEM_TOTAL_KB as f64) as u64 }),
            uptime_s: Some(self.uptime0 + self.boot.elapsed().as_secs()),
            last_exit_code: self.last_exit,
            job_result: self.job_result.clone(),
            exam_id: match &self.seb {
                Seb::Running { exam } => exam.clone(),
                _ => None,
            },
        }
    }

    fn handle(&mut self, reply: HeartbeatReply, chaos: f64) {
        let Some(kind) = reply.job else { return };
        let id = reply.job_id.unwrap_or(0);
        let now = Instant::now();
        match kind {
            JobKind::Start if matches!(self.seb, Seb::Idle) => {
                let exam = reply.exam.as_ref().map(|e| e.id.clone());
                let name = reply.exam.as_ref().map_or("local config".into(), |e| e.name.clone());
                println!("{}: start ({name})", self.host);
                self.seb = Seb::Starting {
                    done: now + Duration::from_secs_f64(self.rng.range(3.0, 8.0)),
                    job: id,
                    exam,
                    fail: self.rng.chance(chaos),
                };
            }
            JobKind::Stop => {
                println!("{}: stop", self.host);
                self.seb = Seb::Stopping { done: now + Duration::from_secs_f64(self.rng.range(1.0, 3.0)), job: id };
            }
            JobKind::Start => {}
        }
    }
}

fn run(mut b: Board, o: &Opts) {
    let url = format!("{}/heartbeat", o.server);
    let mut failures = 0u32;
    loop {
        b.step(o.chaos);
        if let Some(until) = b.silent_until {
            if Instant::now() < until {
                thread::sleep(o.interval);
                continue;
            }
            b.silent_until = None;
        }
        let body = serde_json::to_string(&b.heartbeat()).expect("serialize");
        let mut req = ureq::post(&url).header("Content-Type", "application/json");
        if let Some(t) = &o.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        match req.send(&body).map_err(|e| e.to_string()).and_then(|r| r.into_body().read_to_string().map_err(|e| e.to_string())) {
            Ok(text) => {
                failures = 0;
                b.job_result = None;
                match serde_json::from_str::<HeartbeatReply>(&text) {
                    Ok(reply) => b.handle(reply, o.chaos),
                    Err(e) => eprintln!("{}: bad reply: {e}", b.host),
                }
            }
            Err(e) => {
                failures += 1;
                if failures == 1 || failures % 12 == 0 {
                    eprintln!("{}: heartbeat failed: {e}", b.host);
                }
            }
        }
        // Jitter so boards do not beat in lockstep.
        thread::sleep(o.interval.mul_f64(b.rng.range(0.9, 1.1)));
    }
}

fn main() {
    let o: &'static Opts = Box::leak(Box::new(parse_args()));
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    println!(
        "simulating {} boards against {} (chaos {}, legacy {}){}",
        o.count, o.server, o.chaos, o.legacy,
        if o.token.is_some() { ", with agent token" } else { "" }
    );
    let handles: Vec<_> = (0..o.count)
        .map(|i| {
            let b = Board::new(i, o, seed);
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(150 * i as u64));
                run(b, o)
            })
        })
        .collect();
    for h in handles {
        let _ = h.join();
    }
}
