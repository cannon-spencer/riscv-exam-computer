//! End-to-end tests through the real Axum router: agent compatibility,
//! instructor auth, Origin checks, and security headers.
#![cfg(feature = "ssr")]

use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use leptos::config::LeptosOptions;
use exam_console::backend::{App, Config, CookieSecure, Ctx, http, wall_ms};
use tower::ServiceExt;

const HOST: &str = "exam.test";
const ORIGIN: &str = "http://exam.test";

fn setup(agent_token: Option<&str>) -> (Router, Arc<App>) {
    let app = Arc::new(
        App::new(Config {
            admin_password: "pw-for-tests".into(),
            agent_token: agent_token.map(String::from),
            data_dir: None,
            max_boards: 10,
            allowed_origins: vec![],
            cookie_secure: CookieSecure::Never,
            public_url: Some("https://exam.test".into()),
        })
        .unwrap(),
    );
    let leptos = LeptosOptions::builder().output_name("console").build();
    (http::router(Ctx { leptos, app: app.clone() }), app)
}

async fn send(router: &Router, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, String) {
    let res = router.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes();
    (parts.status, parts.headers, String::from_utf8_lossy(&bytes).into_owned())
}

fn heartbeat(body: &str, token: Option<&str>) -> Request<Body> {
    let mut b = Request::post("/heartbeat").header(header::HOST, HOST).header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).unwrap()
}

fn login(password: &str) -> Request<Body> {
    Request::post("/auth/login")
        .header(header::HOST, HOST)
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!("password={password}")))
        .unwrap()
}

async fn session(router: &Router) -> String {
    let (status, headers, _) = send(router, login("pw-for-tests")).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    cookie.split(';').next().unwrap().to_string()
}

#[tokio::test]
async fn old_agent_heartbeat_and_job_shape_are_unchanged() {
    let (router, app) = setup(None);
    // Exactly what the original seb-agent sends.
    let (status, _, body) = send(&router, heartbeat(r#"{"host":"orangepirv","state":"idle"}"#, None)).await;
    assert_eq!(status, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v.get("job").is_some_and(|j| j.is_null()), "{body}");

    {
        let mut f = app.fleet.lock().unwrap();
        let id = f
            .create_exam("e1".into(), "Quiz", proto::seb::ExamSpec { start_url: "https://ufl.instructure.com/q".into(), allowed_domains: vec![], allow_quit: false, access_code: "TIGER42".into() }, wall_ms())
            .unwrap();
        let r = f.enqueue(&["orangepirv".into()], proto::JobKind::Start, Some(&id), Instant::now(), wall_ms()).unwrap();
        assert_eq!(r.queued, vec!["orangepirv"]);
    }
    let (_, _, body) = send(&router, heartbeat(r#"{"host":"orangepirv","state":"idle"}"#, None)).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    // The old agent does `v.get("job").and_then(|j| j.as_str())`.
    assert_eq!(v.get("job").and_then(|j| j.as_str()), Some("start"));
    assert_eq!(v["exam"]["config"]["browser"]["startUrl"], "https://ufl.instructure.com/q");

    let (_, _, body) = send(&router, heartbeat(r#"{"host":"orangepirv","state":"running"}"#, None)).await;
    assert!(body.contains(r#""job":null"#), "{body}");
}

#[tokio::test]
async fn new_agent_stats_are_accepted() {
    let (router, app) = setup(None);
    let hb = r#"{"host":"b1","state":"idle","protocol":2,"cpu_temp_c":51.5,"load":[0.5,0.4,0.3],
        "mem":{"total_kb":4000000,"available_kb":3000000},"uptime_s":600,"ip":"10.0.0.7",
        "agent_version":"0.2.0","seb_version":"sha256:abc","image_hash":"deadbeef","last_exit_code":0}"#;
    let (status, _, _) = send(&router, heartbeat(hb, None)).await;
    assert_eq!(status, StatusCode::OK);
    let v = app.fleet.lock().unwrap().view(Instant::now());
    let b = &v.boards[0];
    assert_eq!(b.cpu_temp_c, Some(51.5));
    assert_eq!(b.mem_used_pct, Some(25));
    assert_eq!(b.ip.as_deref(), Some("10.0.0.7"));
    assert!(!b.legacy_agent);
}

#[tokio::test]
async fn heartbeat_rejects_bad_input() {
    let (router, _) = setup(None);
    let (s, _, _) = send(&router, heartbeat(r#"{"host":"<img src=x>","state":"idle"}"#, None)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _, _) = send(&router, heartbeat("not json", None)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let big = format!(r#"{{"host":"a","state":"{}"}}"#, "x".repeat(70_000));
    let (s, _, _) = send(&router, heartbeat(&big, None)).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn agent_token_is_enforced_when_set() {
    let (router, _) = setup(Some("tok-123"));
    let body = r#"{"host":"b","state":"idle"}"#;
    assert_eq!(send(&router, heartbeat(body, None)).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(send(&router, heartbeat(body, Some("nope"))).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(send(&router, heartbeat(body, Some("tok-123"))).await.0, StatusCode::OK);
}

#[tokio::test]
async fn pages_and_api_require_a_session() {
    let (router, _) = setup(None);
    for path in ["/", "/exams", "/boards/x", "/machines"] {
        let (s, h, _) = send(&router, Request::get(path).header(header::HOST, HOST).body(Body::empty()).unwrap()).await;
        assert_eq!(s, StatusCode::SEE_OTHER, "{path}");
        assert_eq!(h[header::LOCATION], "/login");
    }
    let req = Request::post("/api/anything").header(header::HOST, HOST).header(header::ORIGIN, ORIGIN).body(Body::empty()).unwrap();
    assert_eq!(send(&router, req).await.0, StatusCode::UNAUTHORIZED);

    // The login page itself is public and carries a nonce-based CSP.
    let (s, h, body) = send(&router, Request::get("/login").header(header::HOST, HOST).body(Body::empty()).unwrap()).await;
    assert_eq!(s, StatusCode::OK);
    assert!(body.contains("Instructor password"));
    let csp = h[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
    assert!(csp.contains("'nonce-") && !csp.contains("script-src 'self' 'unsafe-inline'"), "{csp}");
    assert_eq!(h["x-frame-options"], "DENY");
}

#[tokio::test]
async fn login_sets_strict_httponly_cookie_and_unlocks_pages() {
    let (router, _) = setup(None);
    let (s, h, _) = send(&router, login("wrong")).await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    assert_eq!(h[header::LOCATION], "/login?e=bad");
    assert!(h.get(header::SET_COOKIE).is_none());

    let (_, h, _) = send(&router, login("pw-for-tests")).await;
    let set = h[header::SET_COOKIE].to_str().unwrap();
    assert!(set.contains("HttpOnly") && set.contains("SameSite=Strict") && set.contains("Path=/"), "{set}");
    let cookie = set.split(';').next().unwrap().to_string();

    let (s, _, body) = send(&router, Request::get("/").header(header::HOST, HOST).header(header::COOKIE, &cookie).body(Body::empty()).unwrap()).await;
    assert_eq!(s, StatusCode::OK);
    assert!(body.contains("Fleet"));

    let (s, _, body) = send(&router, Request::get("/machines").header(header::HOST, HOST).header(header::COOKIE, &cookie).body(Body::empty()).unwrap()).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body, "[]");

    // Logout ends the session.
    let req = Request::post("/auth/logout").header(header::HOST, HOST).header(header::ORIGIN, ORIGIN).header(header::COOKIE, &cookie).body(Body::empty()).unwrap();
    assert_eq!(send(&router, req).await.0, StatusCode::SEE_OTHER);
    let (s, _, _) = send(&router, Request::get("/machines").header(header::HOST, HOST).header(header::COOKIE, &cookie).body(Body::empty()).unwrap()).await;
    assert_eq!(s, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn login_is_rate_limited() {
    let (router, _) = setup(None);
    for _ in 0..5 {
        assert_eq!(send(&router, login("wrong")).await.1[header::LOCATION], "/login?e=bad");
    }
    assert_eq!(send(&router, login("pw-for-tests")).await.1[header::LOCATION], "/login?e=locked");
}

#[tokio::test]
async fn state_changes_need_same_origin() {
    let (router, _) = setup(None);
    let cookie = session(&router).await;
    let base = || Request::post("/auth/logout").header(header::HOST, HOST).header(header::COOKIE, &cookie);
    // No Origin or Referer.
    assert_eq!(send(&router, base().body(Body::empty()).unwrap()).await.0, StatusCode::FORBIDDEN);
    // Foreign origin.
    let req = base().header(header::ORIGIN, "https://evil.test").body(Body::empty()).unwrap();
    assert_eq!(send(&router, req).await.0, StatusCode::FORBIDDEN);
    // Cross-origin login attempts are refused before the password is checked.
    let req = Request::post("/auth/login").header(header::HOST, HOST).header(header::ORIGIN, "https://evil.test")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded").body(Body::from("password=pw-for-tests")).unwrap();
    assert_eq!(send(&router, req).await.0, StatusCode::FORBIDDEN);
    // Referer from the same site is accepted when Origin is absent.
    let req = base().header(header::REFERER, "http://exam.test/boards/x").body(Body::empty()).unwrap();
    assert_eq!(send(&router, req).await.0, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn server_functions_work_with_a_session() {
    let (router, _) = setup(None);
    send(&router, heartbeat(r#"{"host":"b1","state":"idle"}"#, None)).await;
    let cookie = session(&router).await;
    let path = server_fn::axum::server_fn_paths()
        .find(|(p, _)| p.contains("get_fleet"))
        .map(|(p, _)| p.to_string())
        .expect("get_fleet registered");
    let req = Request::post(&path)
        .header(header::HOST, HOST)
        .header(header::ORIGIN, ORIGIN)
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::empty())
        .unwrap();
    let (s, _, body) = send(&router, req).await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert!(body.contains(r#""host":"b1""#), "{body}");
}

#[tokio::test]
async fn start_job_carries_seb_and_gate_checks_config_key() {
    use base64::Engine;
    let (router, app) = setup(None);
    send(&router, heartbeat(r#"{"host":"b1","state":"idle","protocol":3}"#, None)).await;
    let spec = proto::seb::ExamSpec {
        start_url: "https://ufl.instructure.com/courses/1/quizzes/2".into(),
        allowed_domains: vec!["ufl.edu".into()],
        allow_quit: false,
        access_code: "TIGER42".into(),
    };
    {
        let mut f = app.fleet.lock().unwrap();
        f.create_exam("e1".into(), "Quiz <1>", spec, wall_ms()).unwrap();
        f.enqueue(&["b1".into()], proto::JobKind::Start, Some("e1"), Instant::now(), wall_ms()).unwrap();
    }
    let (_, _, body) = send(&router, heartbeat(r#"{"host":"b1","state":"idle","protocol":3}"#, None)).await;
    let reply: proto::HeartbeatReply = serde_json::from_str(&body).unwrap();
    let exam = reply.exam.unwrap();
    let ck = exam.config_key.unwrap();

    // The .seb the board gets opens, starts at the gate, and has that key.
    let bytes = base64::engine::general_purpose::STANDARD.decode(exam.seb.unwrap()).unwrap();
    let xml = proto::seb_file::container::decode(&bytes, None).unwrap();
    let settings = proto::seb_file::from_xml(&xml).unwrap();
    assert_eq!(settings["startURL"].as_string(), Some("https://exam.test/gate/e1"));
    assert_eq!(settings["sendBrowserExamKey"].as_boolean(), Some(true));
    assert_eq!(proto::seb_file::config_key(&settings), ck);

    let gate = |hash: Option<String>| {
        let mut b = Request::get("/gate/e1").header(header::HOST, HOST);
        if let Some(h) = hash {
            b = b.header("X-SafeExamBrowser-ConfigKeyHash", h);
        }
        b.body(Body::empty()).unwrap()
    };
    // No header: the JS API page, which reveals nothing by itself.
    let (s, headers, body) = send(&router, gate(None)).await;
    assert_eq!(s, StatusCode::OK);
    assert!(body.contains("SafeExamBrowser") && !body.contains("TIGER42"), "{body}");
    let csp = headers.get(header::CONTENT_SECURITY_POLICY).unwrap().to_str().unwrap().to_string();
    let nonce = csp.split("'nonce-").nth(1).unwrap().split('\'').next().unwrap();
    assert!(body.contains(&format!("nonce=\"{nonce}\"")));
    let wrong = proto::seb_file::config_key_hash("https://exam.test/gate/e2", &ck);
    let (s, _, _) = send(&router, gate(Some(wrong))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let right = proto::seb_file::config_key_hash("https://exam.test/gate/e1", &ck);
    let (s, headers, body) = send(&router, gate(Some(right))).await;
    assert_eq!(s, StatusCode::OK);
    assert!(body.contains("TIGER42"), "{body}");
    assert!(body.contains("Quiz &lt;1&gt;"), "names are escaped");
    assert!(headers.get(header::CONTENT_SECURITY_POLICY).unwrap().to_str().unwrap().contains("default-src 'none'"));

    // JS API path: page URL + configKey posted back.
    let verify = |url: &str, ck: &str| {
        let body = format!("url={}&ck={ck}", url.replace(':', "%3A").replace('/', "%2F").replace('?', "%3F").replace('#', "%23"));
        Request::post("/gate/e1/verify")
            .header(header::HOST, HOST)
            .header(header::ORIGIN, ORIGIN)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap()
    };
    let js_ck = proto::seb_file::config_key_hash("https://exam.test/gate/e1", &ck);
    let (s, _, body) = send(&router, verify("https://exam.test/gate/e1#top", &js_ck)).await;
    assert_eq!(s, StatusCode::OK);
    assert!(body.contains("TIGER42"));
    let (s, _, _) = send(&router, verify("https://exam.test/gate/e1", "00")).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    // A valid hash for another page of the same server is not accepted here.
    let other = proto::seb_file::config_key_hash("https://exam.test/gate/e1x", &ck);
    let (s, _, _) = send(&router, verify("https://exam.test/gate/e1x", &other)).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let elsewhere = proto::seb_file::config_key_hash("https://evil.test/gate/e1", &ck);
    let (s, _, _) = send(&router, verify("https://evil.test/gate/e1", &elsewhere)).await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // .seb download is for signed-in instructors only.
    let dl = |cookie: Option<&str>| {
        let mut b = Request::get("/exams/e1/exam.seb").header(header::HOST, HOST);
        if let Some(c) = cookie {
            b = b.header(header::COOKIE, c);
        }
        b.body(Body::empty()).unwrap()
    };
    let (s, _, _) = send(&router, dl(None)).await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    let cookie = session(&router).await;
    let (s, headers, _) = send(&router, dl(Some(&cookie))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(headers.get("x-seb-config-key").unwrap(), ck.as_str());
    assert!(headers.get(header::CONTENT_DISPOSITION).unwrap().to_str().unwrap().contains("quiz-1.seb"));

    // Password-encrypted copy for official SEB on student machines.
    let locked = |pw: &str| {
        Request::post("/exams/e1/exam.seb")
            .header(header::HOST, HOST)
            .header(header::ORIGIN, ORIGIN)
            .header(header::COOKIE, &cookie)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(format!("password={pw}")))
            .unwrap()
    };
    let res = router.clone().oneshot(locked("correct-horse")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert!(proto::seb_file::container::decode(&bytes, None).is_err());
    let xml = proto::seb_file::container::decode(&bytes, Some("correct-horse")).unwrap();
    assert_eq!(proto::seb_file::config_key(&proto::seb_file::from_xml(&xml).unwrap()), ck);
    let (s, _, _) = send(&router, locked("ab")).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn gate_without_access_code_redirects_to_canvas() {
    let (router, app) = setup(None);
    let spec = proto::seb::ExamSpec { start_url: "https://ufl.instructure.com/q".into(), ..Default::default() };
    let ck = {
        let mut f = app.fleet.lock().unwrap();
        f.create_exam("e9".into(), "Open", spec, wall_ms()).unwrap();
        let e = f.exam("e9").unwrap().clone();
        f.seb_build(&e).unwrap().config_key
    };
    let req = Request::get("/gate/e9?x=1")
        .header(header::HOST, HOST)
        .header("X-SafeExamBrowser-ConfigKeyHash", proto::seb_file::config_key_hash("https://exam.test/gate/e9?x=1", &ck))
        .body(Body::empty())
        .unwrap();
    let (s, headers, _) = send(&router, req).await;
    assert_eq!(s, StatusCode::SEE_OTHER);
    assert_eq!(headers.get(header::LOCATION).unwrap(), "https://ufl.instructure.com/q");
}
