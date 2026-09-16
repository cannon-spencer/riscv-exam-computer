use axum::{Router, routing::post, Json};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/heartbeat", post(heartbeat));
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("listening on {addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn heartbeat(body: String) -> Json<serde_json::Value> {
    println!("heartbeat {body}");
    Json(serde_json::json!({ "ok": true }))
}
