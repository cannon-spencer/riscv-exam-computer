use crate::AppState;
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

// In-memory .seb. Lost on server restart, same as the machine list.
pub struct ExamFile {
    bytes: Vec<u8>,
    etag: String,
}

fn etag(bytes: &[u8]) -> String {
    format!("\"{:x}\"", Sha256::digest(bytes))
}

pub async fn get(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let exam = app.exam.lock().unwrap();
    let Some(file) = exam.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // Agent sends If-None-Match: "<sha256>". Same tag: 304, no body.
    let cached = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(file.etag.as_str());
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

pub async fn put(State(app): State<AppState>, body: Bytes) -> Json<Value> {
    let etag = etag(&body);
    let n = body.len();
    *app.exam.lock().unwrap() = Some(ExamFile {
        bytes: body.to_vec(),
        etag: etag.clone(),
    });
    println!("exam.seb stored bytes={n} etag={etag}");
    Json(json!({ "ok": true, "etag": etag, "bytes": n }))
}
