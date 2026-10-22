//! Axum router: agent endpoint, login/logout, and the guard middleware.

use std::net::SocketAddr;
use std::time::Instant;

use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, DefaultBodyLimit, FromRequest, Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use sha2::{Digest, Sha256};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use leptos_axum::{LeptosRoutes, generate_route_list};
use proto::{Heartbeat, HeartbeatReply};
use serde::Deserialize;

use super::auth::{self, LoginError, SESSION_COOKIE};
use super::fleet::HeartbeatError;
use super::{CookieSecure, Ctx, wall_ms};
use crate::app::{App, shell};

pub fn router(ctx: Ctx) -> Router {
    let routes = generate_route_list(App);
    let shell_opts = ctx.leptos.clone();
    Router::new()
        .route("/heartbeat", post(heartbeat).layer(DefaultBodyLimit::max(64 * 1024)))
        .route("/exam.seb", get(exam_get).post(exam_put).layer(DefaultBodyLimit::max(8 * 1024 * 1024)))
        .route("/healthz", get(|| async { "ok" }))
        .route("/auth/login", post(login).layer(DefaultBodyLimit::max(4 * 1024)))
        .route("/auth/logout", post(logout))
        .route("/machines", get(machines))
        .route("/gate/{id}", get(gate))
        .route("/exams/{id}/exam.seb", get(download_seb))
        .leptos_routes(&ctx, routes, move || shell(shell_opts.clone()))
        .fallback(leptos_axum::file_and_error_handler::<Ctx, _>(shell))
        .layer(middleware::from_fn_with_state(ctx.clone(), guard))
        .with_state(ctx)
}

// ----- agent ------------------------------------------------------------------

fn agent_error(status: StatusCode, msg: &str) -> Response {
    let body = HeartbeatReply { ok: false, error: Some(msg.into()), ..Default::default() };
    (status, Json(body)).into_response()
}

async fn heartbeat(State(ctx): State<Ctx>, headers: HeaderMap, body: Bytes) -> Response {
    let authz = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok());
    if !ctx.app.auth.agent_allowed(authz) {
        return agent_error(StatusCode::UNAUTHORIZED, "missing or wrong agent token");
    }
    let hb: Heartbeat = match serde_json::from_slice(&body) {
        Ok(hb) => hb,
        Err(_) => return agent_error(StatusCode::BAD_REQUEST, "malformed heartbeat"),
    };
    let host = hb.host.clone();
    let result = ctx.app.fleet.lock().unwrap().heartbeat(hb, Instant::now(), wall_ms());
    match result {
        Ok(reply) => {
            if let Some(job) = reply.job {
                println!("job {} -> {host} (id {})", job.as_str(), reply.job_id.unwrap_or(0));
            }
            Json(reply).into_response()
        }
        Err(HeartbeatError::InvalidHost) => agent_error(StatusCode::BAD_REQUEST, "invalid host name"),
        Err(HeartbeatError::TooManyBoards) => agent_error(StatusCode::TOO_MANY_REQUESTS, "board limit reached"),
    }
}

/// Raw `.seb` the board agent polls. `If-None-Match` with the same sha256 returns 304.
async fn exam_get(State(ctx): State<Ctx>, headers: HeaderMap) -> Response {
    let exam = ctx.app.exam_file.lock().unwrap();
    let Some(file) = exam.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let cached = headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(file.etag.as_str());
    if cached {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, file.etag.clone())]).into_response();
    }
    (
        StatusCode::OK,
        [
            (header::ETAG, file.etag.clone()),
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        ],
        file.bytes.clone(),
    )
        .into_response()
}

async fn exam_put(State(ctx): State<Ctx>, body: Bytes) -> Response {
    let etag = format!("\"{:x}\"", Sha256::digest(&body));
    let n = body.len();
    *ctx.app.exam_file.lock().unwrap() = Some(super::ExamBlob {
        bytes: body.to_vec(),
        etag: etag.clone(),
    });
    println!("exam.seb stored bytes={n} etag={etag}");
    Json(serde_json::json!({ "ok": true, "etag": etag, "bytes": n })).into_response()
}

/// JSON fleet listing for scripts (`curl -b exam_session=...`).
async fn machines(State(ctx): State<Ctx>) -> Response {
    Json(ctx.app.fleet.lock().unwrap().view(Instant::now()).boards).into_response()
}

// ----- SEB gate ---------------------------------------------------------------

/// Header SEB adds when `sendBrowserExamKey` is on.
const CONFIG_KEY_HEADER: &str = "x-safeexambrowser-configkeyhash";

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn gate_page(status: StatusCode, title: &str, body: &str) -> Response {
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{t}</title><style>\
         body{{font:17px/1.5 system-ui,sans-serif;background:#0b0b0d;color:#f5f5f7;display:grid;place-items:center;min-height:100vh;margin:0}}\
         main{{max-width:34rem;padding:2rem;background:#1c1c20;border-radius:20px}}\
         h1{{font-size:1.5rem;margin:0 0 .5rem}} .code{{font:600 2rem ui-monospace,monospace;letter-spacing:.1em;margin:1rem 0}}\
         a.btn{{display:inline-block;background:#f5f5f7;color:#0b0b0d;padding:.6rem 1.2rem;border-radius:999px;text-decoration:none;font-weight:600}}\
         .muted{{color:#a1a1a6}}</style></head><body><main><h1>{t}</h1>{body}</main></body></html>",
        t = html_escape(title),
    );
    let mut res = (status, [(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response();
    res.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"),
    );
    res
}

/// Start page of gated exams. SEB opens it with
/// `X-SafeExamBrowser-ConfigKeyHash = sha256(this URL + config key)`; only a
/// browser running this exam's .seb can produce that. On success the page
/// shows the Canvas access code (or redirects if the exam has none).
async fn gate(State(ctx): State<Ctx>, Path(id): Path<String>, req: Request) -> Response {
    let (build, spec, name, public) = {
        let fleet = ctx.app.fleet.lock().unwrap();
        let Some(exam) = fleet.exam(&id) else {
            return gate_page(StatusCode::NOT_FOUND, "Exam not found", "<p class=\"muted\">This exam no longer exists. Ask your instructor.</p>");
        };
        let Some(public) = fleet.public_url().map(str::to_owned) else {
            return gate_page(StatusCode::SERVICE_UNAVAILABLE, "Gate is off", "<p class=\"muted\">EXAM_PUBLIC_URL is not set on the exam server.</p>");
        };
        match fleet.seb_build(exam) {
            Ok(b) => (b, exam.spec.clone(), exam.name.clone(), public),
            Err(e) => return gate_page(StatusCode::INTERNAL_SERVER_ERROR, "Exam is misconfigured", &format!("<p class=\"muted\">{}</p>", html_escape(&e))),
        }
    };

    // The absolute URL SEB requested: our public base plus path and query.
    let pq = req.uri().path_and_query().map_or("/", |p| p.as_str());
    let url = format!("{public}{pq}");
    let received = req.headers().get(CONFIG_KEY_HEADER).and_then(|v| v.to_str().ok());
    let ok = received.is_some_and(|h| proto::seb_file::verify_config_key_hash(&url, &build.config_key, h));

    if !ok {
        let why = if received.is_some() { "wrong Config Key" } else { "no Config Key header" };
        ctx.app.fleet.lock().unwrap().log(
            wall_ms(), crate::model::Actor::System, crate::model::Level::Warn, None,
            format!("Gate refused for {name}: {why}"));
        return gate_page(
            StatusCode::FORBIDDEN,
            "Open this exam on an exam board",
            "<p>This page only works inside Safe Exam Browser started from the exam console.</p>\
             <p class=\"muted\">If you are on an exam board, ask your instructor to restart it.</p>",
        );
    }

    if spec.access_code.is_empty() {
        return Redirect::to(&spec.start_url).into_response();
    }
    gate_page(
        StatusCode::OK,
        &name,
        &format!(
            "<p>Exam board verified. Enter this access code when Canvas asks for it:</p>\
             <p class=\"code\">{code}</p><p><a class=\"btn\" href=\"{href}\">Continue to the exam</a></p>",
            code = html_escape(&spec.access_code),
            href = html_escape(&spec.start_url),
        ),
    )
}

/// The exam's .seb for instructors (behind the session guard). Opens in
/// official SEB too, which is handy for testing the gate on a laptop.
async fn download_seb(State(ctx): State<Ctx>, Path(id): Path<String>) -> Response {
    let fleet = ctx.app.fleet.lock().unwrap();
    let Some(exam) = fleet.exam(&id) else { return (StatusCode::NOT_FOUND, "Unknown exam").into_response() };
    let build = match fleet.seb_build(exam) {
        Ok(b) => b,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let slug: String = exam.name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
    let disposition = format!("attachment; filename=\"{}.seb\"", if slug.is_empty() { "exam" } else { &slug });
    let bytes = proto::seb_file::container::encode(&build.xml, None);
    let mut res = (StatusCode::OK, bytes).into_response();
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/seb"));
    if let Ok(v) = HeaderValue::from_str(&disposition) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    h.insert("x-seb-config-key", HeaderValue::from_str(&build.config_key).unwrap());
    res
}

// ----- instructor login -------------------------------------------------------

#[derive(Deserialize)]
struct LoginForm {
    password: String,
}

/// Rate-limit key: the proxy's client IP when behind Cloudflare, else the peer.
fn client_key(req: &Request) -> String {
    if let Some(ip) = req.headers().get("cf-connecting-ip").and_then(|v| v.to_str().ok()) {
        return ip.chars().take(64).collect();
    }
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(|| "unknown".into(), |c| c.0.ip().to_string())
}

fn wants_secure_cookie(ctx: &Ctx, headers: &HeaderMap) -> bool {
    match ctx.app.cookie_secure {
        CookieSecure::Always => true,
        CookieSecure::Never => false,
        CookieSecure::Auto => headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("https")),
    }
}

fn see_other(location: &'static str) -> Response {
    Redirect::to(location).into_response()
}

async fn login(State(ctx): State<Ctx>, req: Request) -> Response {
    let key = client_key(&req);
    let secure = wants_secure_cookie(&ctx, req.headers());
    let Ok(Form(form)) = Form::<LoginForm>::from_request(req, &()).await else {
        return see_other("/login?e=bad");
    };
    match ctx.app.auth.login(&key, &form.password, Instant::now()) {
        Ok(token) => {
            ctx.app.fleet.lock().unwrap().log(
                wall_ms(), crate::model::Actor::Instructor, crate::model::Level::Info, None, "Signed in");
            let cookie = format!(
                "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
                auth::SESSION_TTL.as_secs(),
                if secure { "; Secure" } else { "" }
            );
            let mut res = see_other("/");
            res.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());
            res
        }
        Err(LoginError::BadPassword) => {
            ctx.app.fleet.lock().unwrap().log(
                wall_ms(), crate::model::Actor::System, crate::model::Level::Warn, None,
                format!("Failed sign-in attempt from {key}"));
            see_other("/login?e=bad")
        }
        Err(LoginError::RateLimited) => see_other("/login?e=locked"),
    }
}

async fn logout(State(ctx): State<Ctx>, headers: HeaderMap) -> Response {
    if let Some(tok) = session_token(&headers) {
        ctx.app.auth.logout(tok);
    }
    let mut res = see_other("/login");
    let clear = format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    res.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_str(&clear).unwrap());
    res
}

fn session_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|v| auth::cookie(v, SESSION_COOKIE))
}

// ----- guard ------------------------------------------------------------------

fn is_public(path: &str) -> bool {
    matches!(path, "/heartbeat" | "/exam.seb" | "/healthz" | "/login" | "/auth/login" | "/favicon.svg")
        || path.starts_with("/gate/")
        || path.starts_with("/pkg/")
        || path.starts_with("/fonts/")
}

/// Origin check on state-changing requests, then session check on
/// everything that is not public. Adds security headers to all responses.
async fn guard(State(ctx): State<Ctx>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let unsafe_method = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);

    if unsafe_method && path != "/heartbeat" && path != "/exam.seb" {
        let h = req.headers();
        let host = h.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
        let origin = h
            .get(header::ORIGIN)
            .or_else(|| h.get(header::REFERER))
            .and_then(|v| v.to_str().ok());
        let ok = origin.is_some_and(|o| auth::same_origin(o, host, &ctx.app.allowed_origins));
        if !ok {
            return with_headers((StatusCode::FORBIDDEN, "Cross-origin request refused").into_response());
        }
    }

    if !is_public(&path) {
        let ok = session_token(req.headers()).is_some_and(|t| ctx.app.auth.session_valid(t, Instant::now()));
        if !ok {
            let res = if *req.method() == Method::GET {
                see_other("/login")
            } else {
                (StatusCode::UNAUTHORIZED, "Sign in first").into_response()
            };
            return with_headers(res);
        }
    }

    with_headers(next.run(req).await)
}

fn with_headers(mut res: Response<Body>) -> Response<Body> {
    let h = res.headers_mut();
    h.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    h.insert("referrer-policy", HeaderValue::from_static("same-origin"));
    if !h.contains_key(header::CONTENT_SECURITY_POLICY) {
        // Pages set their own CSP with a script nonce (see `app::shell`).
        h.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; script-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"),
        );
    }
    if !h.contains_key(header::CACHE_CONTROL) {
        h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    res
}

/// CSP for rendered pages. Scripts need the per-response nonce; WebAssembly
/// needs `wasm-unsafe-eval`. Styles allow inline `style=` attributes, which
/// Leptos uses for dynamic widths.
pub fn page_csp(nonce: &str) -> String {
    format!(
        "default-src 'self'; script-src 'self' 'nonce-{nonce}' 'wasm-unsafe-eval'; \
         style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; \
         connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"
    )
}
