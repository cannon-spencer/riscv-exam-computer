//! Server functions called by the UI. Every route under `/api/` is behind
//! the session and Origin checks in `backend::http::guard`.

use leptos::prelude::*;

use crate::model::*;

#[cfg(feature = "ssr")]
mod imp {
    use super::*;
    pub use crate::backend::{Ctx, wall_ms};
    pub use std::time::Instant;

    pub fn ctx() -> Result<Ctx, ServerFnError> {
        use_context::<Ctx>().ok_or_else(|| ServerFnError::new("server context missing"))
    }

    pub fn fail(msg: impl Into<String>) -> ServerFnError {
        ServerFnError::new(msg.into())
    }
}

/// Text of a server function error, without the framework's prefix.
pub fn error_text(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => m.clone(),
        other => other.to_string(),
    }
}

#[server]
pub async fn get_session_info() -> Result<SessionInfo, ServerFnError> {
    let ctx = imp::ctx()?;
    Ok(SessionInfo {
        agent_token_missing: !ctx.app.auth.agent_token_required(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[server]
pub async fn get_fleet() -> Result<FleetView, ServerFnError> {
    let ctx = imp::ctx()?;
    let view = ctx.app.fleet.lock().unwrap().view(imp::Instant::now());
    Ok(view)
}

/// `hosts` is comma separated (host names cannot contain commas).
/// `verb` is `start` or `stop`.
#[server]
pub async fn board_action(hosts: String, verb: String, exam_id: String) -> Result<ActionReport, ServerFnError> {
    let ctx = imp::ctx()?;
    let kind = match verb.as_str() {
        "start" => proto::JobKind::Start,
        "stop" => proto::JobKind::Stop,
        _ => return Err(imp::fail("Unknown action")),
    };
    let hosts: Vec<String> = hosts.split(',').map(str::trim).filter(|h| !h.is_empty()).map(String::from).collect();
    if hosts.is_empty() {
        return Err(imp::fail("Select at least one board"));
    }
    let exam = (!exam_id.is_empty()).then_some(exam_id.as_str());
    let report = ctx.app.fleet.lock().unwrap().enqueue(&hosts, kind, exam, imp::Instant::now(), imp::wall_ms());
    report.map_err(imp::fail)
}

#[server]
pub async fn get_board(host: String) -> Result<Option<BoardDetail>, ServerFnError> {
    let ctx = imp::ctx()?;
    let detail = ctx.app.fleet.lock().unwrap().detail(&host, imp::Instant::now());
    Ok(detail)
}

#[server]
pub async fn rename_board(host: String, label: String) -> Result<(), ServerFnError> {
    let ctx = imp::ctx()?;
    let r = ctx.app.fleet.lock().unwrap().rename(&host, &label, imp::wall_ms());
    r.map_err(imp::fail)
}

#[server]
pub async fn forget_board(host: String) -> Result<(), ServerFnError> {
    let ctx = imp::ctx()?;
    let r = ctx.app.fleet.lock().unwrap().forget(&host, imp::wall_ms());
    r.map_err(imp::fail)
}

#[server]
pub async fn list_exams() -> Result<Vec<ExamView>, ServerFnError> {
    let ctx = imp::ctx()?;
    let exams = ctx.app.fleet.lock().unwrap().exam_views();
    Ok(exams)
}

#[server]
pub async fn create_exam(name: String, start_url: String, domains: String, allow_quit: bool) -> Result<String, ServerFnError> {
    let ctx = imp::ctx()?;
    let allowed_domains = proto::seb::parse_domain_list(&domains).map_err(imp::fail)?;
    let spec = proto::seb::ExamSpec { start_url, allowed_domains, allow_quit };
    let id = crate::backend::auth::random_hex(4);
    let r = ctx.app.fleet.lock().unwrap().create_exam(id, &name, spec, imp::wall_ms());
    r.map_err(imp::fail)
}

#[server]
pub async fn delete_exam(id: String) -> Result<(), ServerFnError> {
    let ctx = imp::ctx()?;
    let r = ctx.app.fleet.lock().unwrap().delete_exam(&id, imp::wall_ms());
    r.map_err(imp::fail)
}

/// `filter`: `all`, `instructor`, `board`, or `system`.
#[server]
pub async fn get_activity(filter: String) -> Result<Vec<EventView>, ServerFnError> {
    let ctx = imp::ctx()?;
    let actor = match filter.as_str() {
        "instructor" => Some(Actor::Instructor),
        "board" => Some(Actor::Board),
        "system" => Some(Actor::System),
        _ => None,
    };
    let events = ctx.app.fleet.lock().unwrap().events(None, actor, 300);
    Ok(events)
}
