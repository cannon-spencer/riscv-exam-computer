//! Axum router: agent endpoint, login/logout, and the guard middleware.

use std::net::SocketAddr;
use std::time::Instant;

use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, DefaultBodyLimit, FromRequest, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
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
        .route("/healthz", get(|| async { "ok" }))
        .route("/auth/login", post(login).layer(DefaultBodyLimit::max(4 * 1024)))
        .route("/auth/logout", post(logout))
        .route("/machines", get(machines))
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

/// JSON fleet listing for scripts (`curl -b exam_session=...`).
async fn machines(State(ctx): State<Ctx>) -> Response {
    Json(ctx.app.fleet.lock().unwrap().view(Instant::now()).boards).into_response()
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
    matches!(path, "/heartbeat" | "/healthz" | "/login" | "/auth/login" | "/favicon.svg")
        || path.starts_with("/pkg/")
        || path.starts_with("/fonts/")
}

/// Origin check on state-changing requests, then session check on
/// everything that is not public. Adds security headers to all responses.
async fn guard(State(ctx): State<Ctx>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let unsafe_method = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);

    if unsafe_method && path != "/heartbeat" {
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
