//! Server-only code: state, persistence, auth, and HTTP handlers.

pub mod auth;
pub mod fleet;
pub mod http;
pub mod store;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::FromRef;
use leptos::config::LeptosOptions;

use auth::Auth;
use fleet::Fleet;
use store::Store;

/// Raw `.seb` bytes the board agent polls at `GET /exam.seb`.
/// Lost on restart, same as the pre-console server.
pub struct ExamBlob {
    pub bytes: Vec<u8>,
    pub etag: String,
}

pub fn wall_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieSecure {
    Always,
    Never,
    /// Secure when the proxy says the client used HTTPS.
    Auto,
}

pub struct Config {
    pub admin_password: String,
    pub agent_token: Option<String>,
    pub data_dir: Option<PathBuf>,
    pub max_boards: usize,
    pub allowed_origins: Vec<String>,
    pub cookie_secure: CookieSecure,
    /// Base URL boards use to reach this server; enables the SEB gate.
    pub public_url: Option<String>,
}

impl Config {
    /// Reads `EXAM_*` variables. Returns the config and whether the admin
    /// password was generated (so the caller can print it).
    pub fn from_env() -> (Self, bool) {
        let var = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let (admin_password, generated) = match var("EXAM_ADMIN_PASSWORD") {
            Some(p) => (p, false),
            None => (auth::random_password(), true),
        };
        let cfg = Config {
            admin_password,
            agent_token: var("EXAM_AGENT_TOKEN"),
            data_dir: Some(var("EXAM_DATA_DIR").map_or_else(|| PathBuf::from("data"), PathBuf::from)),
            max_boards: var("EXAM_MAX_BOARDS").and_then(|v| v.parse().ok()).unwrap_or(200).clamp(1, 5000),
            allowed_origins: var("EXAM_ALLOWED_ORIGINS")
                .map(|v| v.split(',').map(|s| s.trim().trim_end_matches('/').to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default(),
            cookie_secure: match var("EXAM_COOKIE_SECURE").as_deref() {
                Some("1" | "true" | "always") => CookieSecure::Always,
                Some("0" | "false" | "never") => CookieSecure::Never,
                _ => CookieSecure::Auto,
            },
            public_url: var("EXAM_PUBLIC_URL"),
        };
        (cfg, generated)
    }
}

pub struct App {
    pub fleet: Mutex<Fleet>,
    pub auth: Auth,
    pub store: Option<Store>,
    pub exam_file: Mutex<Option<ExamBlob>>,
    pub allowed_origins: Vec<String>,
    pub cookie_secure: CookieSecure,
}

impl App {
    pub fn new(cfg: Config) -> std::io::Result<Self> {
        let store = cfg.data_dir.as_deref().map(Store::new).transpose()?;
        let mut fleet = match &store {
            Some(s) => {
                let (snap, warning) = s.load();
                let mut f = Fleet::from_snapshot(snap, cfg.max_boards);
                if let Some(w) = warning {
                    eprintln!("warning: {w}");
                    f.log(wall_ms(), crate::model::Actor::System, crate::model::Level::Warn, None, w);
                }
                f
            }
            None => Fleet::new(cfg.max_boards),
        };
        if let Some(u) = &cfg.public_url {
            if proto::seb::url_host(u).is_err() {
                eprintln!("warning: EXAM_PUBLIC_URL={u} is not an https URL; the SEB gate stays off");
            } else {
                fleet.set_public_url(Some(u.clone()));
            }
        }
        if fleet.public_url().is_none() {
            fleet.log(wall_ms(), crate::model::Actor::System, crate::model::Level::Warn, None,
                "EXAM_PUBLIC_URL is not set: boards open Canvas directly and the Config Key is not checked");
        }
        fleet.log(wall_ms(), crate::model::Actor::System, crate::model::Level::Info, None, "Server started");
        Ok(App {
            fleet: Mutex::new(fleet),
            auth: Auth::new(&cfg.admin_password, cfg.agent_token.as_deref()),
            store,
            exam_file: Mutex::new(None),
            allowed_origins: cfg.allowed_origins,
            cookie_secure: cfg.cookie_secure,
        })
    }

    /// Writes state if anything persisted changed (or `force`).
    pub fn save(&self, force: bool) {
        let Some(store) = &self.store else { return };
        let snap = {
            let mut f = self.fleet.lock().unwrap();
            if !f.dirty && !force {
                return;
            }
            f.dirty = false;
            f.snapshot()
        };
        if let Err(e) = store.save(&snap) {
            eprintln!("error: saving {}: {e}", store.path().display());
            self.fleet.lock().unwrap().dirty = true;
        }
    }

    pub fn tick(&self) {
        self.fleet.lock().unwrap().tick(Instant::now(), wall_ms());
    }
}

/// Router state; also provided to server functions as context.
#[derive(Clone)]
pub struct Ctx {
    pub leptos: LeptosOptions,
    pub app: Arc<App>,
}

impl FromRef<Ctx> for LeptosOptions {
    fn from_ref(c: &Ctx) -> Self {
        c.leptos.clone()
    }
}

/// Ticks job expiry every second and saves when dirty (and every minute,
/// to keep last-seen times fresh).
pub fn spawn_background(app: Arc<App>) {
    tokio::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_secs(1));
        let mut n: u64 = 0;
        loop {
            every.tick().await;
            n += 1;
            app.tick();
            if n % 2 == 0 {
                let app = app.clone();
                let force = n % 60 == 0;
                let _ = tokio::task::spawn_blocking(move || app.save(force)).await;
            }
        }
    });
}
