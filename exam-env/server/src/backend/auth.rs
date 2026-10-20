//! Instructor sessions, login rate limiting, agent tokens, and Origin checks.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub const SESSION_COOKIE: &str = "exam_session";
pub const SESSION_TTL: Duration = Duration::from_secs(12 * 3600);
const MAX_SESSIONS: usize = 256;

/// Per client: this many failures within `WINDOW` locks that client out.
const PER_CLIENT_FAILURES: usize = 5;
/// Across all clients (client keys can be spoofed behind some proxies).
const GLOBAL_FAILURES: usize = 30;
const WINDOW: Duration = Duration::from_secs(5 * 60);
const GLOBAL_WINDOW: Duration = Duration::from_secs(60);

fn digest(s: &str) -> [u8; 32] {
    Sha256::digest(s.as_bytes()).into()
}

pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

/// Readable random password, e.g. `kq7m-3xtd-9fpw-ha4c`.
pub fn random_password() -> String {
    const ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    let mut buf = [0u8; 16];
    rand::rng().fill_bytes(&mut buf);
    let chars: Vec<char> = buf.iter().map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char).collect();
    chars.chunks(4).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>().join("-")
}

#[derive(Default)]
struct Limiter {
    per_client: HashMap<String, VecDeque<Instant>>,
    global: VecDeque<Instant>,
}

impl Limiter {
    fn prune(&mut self, now: Instant) {
        while self.global.front().is_some_and(|t| now.duration_since(*t) > GLOBAL_WINDOW) {
            self.global.pop_front();
        }
        self.per_client.retain(|_, q| {
            while q.front().is_some_and(|t| now.duration_since(*t) > WINDOW) {
                q.pop_front();
            }
            !q.is_empty()
        });
    }
}

pub struct Auth {
    password: [u8; 32],
    agent_token: Option<[u8; 32]>,
    sessions: Mutex<HashMap<[u8; 32], Instant>>,
    limiter: Mutex<Limiter>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoginError {
    BadPassword,
    RateLimited,
}

impl Auth {
    pub fn new(password: &str, agent_token: Option<&str>) -> Self {
        Auth {
            password: digest(password),
            agent_token: agent_token.filter(|t| !t.is_empty()).map(digest),
            sessions: Mutex::new(HashMap::new()),
            limiter: Mutex::new(Limiter::default()),
        }
    }

    pub fn agent_token_required(&self) -> bool {
        self.agent_token.is_some()
    }

    /// `authorization` is the raw `Authorization` header, if any.
    pub fn agent_allowed(&self, authorization: Option<&str>) -> bool {
        let Some(expected) = &self.agent_token else { return true };
        let Some(token) = authorization.and_then(|h| h.strip_prefix("Bearer ")) else { return false };
        bool::from(digest(token.trim()).ct_eq(expected))
    }

    /// Checks the password, rate-limited per client key and globally.
    /// Returns a new session token on success.
    pub fn login(&self, client: &str, password: &str, now: Instant) -> Result<String, LoginError> {
        let mut lim = self.limiter.lock().unwrap();
        lim.prune(now);
        let client_failures = lim.per_client.get(client).map_or(0, VecDeque::len);
        if client_failures >= PER_CLIENT_FAILURES || lim.global.len() >= GLOBAL_FAILURES {
            return Err(LoginError::RateLimited);
        }
        if !bool::from(digest(password).ct_eq(&self.password)) {
            lim.per_client.entry(client.to_string()).or_default().push_back(now);
            lim.global.push_back(now);
            return Err(LoginError::BadPassword);
        }
        lim.per_client.remove(client);
        drop(lim);

        let token = random_hex(32);
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|_, exp| *exp > now);
        if sessions.len() >= MAX_SESSIONS {
            // Drop the session closest to expiry.
            if let Some(k) = sessions.iter().min_by_key(|(_, e)| **e).map(|(k, _)| *k) {
                sessions.remove(&k);
            }
        }
        sessions.insert(digest(&token), now + SESSION_TTL);
        Ok(token)
    }

    pub fn session_valid(&self, token: &str, now: Instant) -> bool {
        if token.len() != 64 {
            return false;
        }
        self.sessions.lock().unwrap().get(&digest(token)).is_some_and(|exp| *exp > now)
    }

    pub fn logout(&self, token: &str) {
        self.sessions.lock().unwrap().remove(&digest(token));
    }
}

/// Reads one cookie from a `Cookie` header value.
pub fn cookie<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|kv| {
        let (k, v) = kv.trim().split_once('=')?;
        (k == name).then_some(v)
    })
}

/// True when `origin` (an `Origin` or `Referer` value) names the same host
/// as the request's `Host` header, or one of `extra` allowed origins.
pub fn same_origin(origin: &str, host: &str, extra: &[String]) -> bool {
    let origin = origin.trim();
    if extra.iter().any(|o| o.eq_ignore_ascii_case(origin.trim_end_matches('/'))) {
        return true;
    }
    let Some(rest) = origin.strip_prefix("https://").or_else(|| origin.strip_prefix("http://")) else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    !authority.is_empty() && !authority.contains('@') && authority.eq_ignore_ascii_case(host.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_login_and_sessions() {
        let a = Auth::new("correct horse", None);
        let t = Instant::now();
        assert_eq!(a.login("1.2.3.4", "wrong", t), Err(LoginError::BadPassword));
        let tok = a.login("1.2.3.4", "correct horse", t).unwrap();
        assert!(a.session_valid(&tok, t));
        assert!(!a.session_valid(&tok, t + SESSION_TTL + Duration::from_secs(1)));
        assert!(!a.session_valid("nope", t));
        a.logout(&tok);
        assert!(!a.session_valid(&tok, t));
    }

    #[test]
    fn login_is_rate_limited_per_client_and_globally() {
        let a = Auth::new("pw", None);
        let t = Instant::now();
        for _ in 0..PER_CLIENT_FAILURES {
            assert_eq!(a.login("x", "bad", t), Err(LoginError::BadPassword));
        }
        // Even the right password is refused while locked out.
        assert_eq!(a.login("x", "pw", t), Err(LoginError::RateLimited));
        assert!(a.login("y", "pw", t).is_ok());
        // The lockout lapses.
        assert!(a.login("x", "pw", t + WINDOW + Duration::from_secs(1)).is_ok());

        let b = Auth::new("pw", None);
        for i in 0..GLOBAL_FAILURES {
            let _ = b.login(&format!("c{i}"), "bad", t);
        }
        assert_eq!(b.login("fresh", "pw", t), Err(LoginError::RateLimited));
        assert!(b.login("fresh", "pw", t + GLOBAL_WINDOW + Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn agent_token() {
        let open = Auth::new("pw", None);
        assert!(open.agent_allowed(None));
        let a = Auth::new("pw", Some("s3cret"));
        assert!(!open.agent_token_required());
        assert!(a.agent_token_required());
        assert!(!a.agent_allowed(None));
        assert!(!a.agent_allowed(Some("Bearer wrong")));
        assert!(!a.agent_allowed(Some("s3cret")));
        assert!(a.agent_allowed(Some("Bearer s3cret")));
    }

    #[test]
    fn origin_matching() {
        assert!(same_origin("https://exam.example.edu", "exam.example.edu", &[]));
        assert!(same_origin("http://127.0.0.1:8000", "127.0.0.1:8000", &[]));
        assert!(same_origin("http://127.0.0.1:8000/fleet?x", "127.0.0.1:8000", &[]));
        assert!(!same_origin("https://evil.test", "exam.example.edu", &[]));
        assert!(!same_origin("https://exam.example.edu.evil.test", "exam.example.edu", &[]));
        assert!(!same_origin("https://exam.example.edu@evil.test", "exam.example.edu", &[]));
        assert!(!same_origin("null", "exam.example.edu", &[]));
        assert!(same_origin("https://other.test", "x", &["https://other.test".into()]));
    }

    #[test]
    fn cookie_parsing() {
        assert_eq!(cookie("a=1; exam_session=abc; theme=dark", SESSION_COOKIE), Some("abc"));
        assert_eq!(cookie("xexam_session=abc", SESSION_COOKIE), None);
    }

    #[test]
    fn random_password_shape() {
        let p = random_password();
        assert_eq!(p.len(), 19);
        assert_eq!(p.matches('-').count(), 3);
    }
}
