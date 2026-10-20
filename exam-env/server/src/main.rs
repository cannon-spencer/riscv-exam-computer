use std::net::SocketAddr;
use std::sync::Arc;

use exam_console::backend::{App, Config, Ctx, http, spawn_background};

#[tokio::main]
async fn main() {
    let conf = leptos::config::get_configuration(None).expect("leptos configuration");
    let leptos = conf.leptos_options;

    let (cfg, generated) = Config::from_env();
    if generated {
        println!("================================================================");
        println!(" EXAM_ADMIN_PASSWORD is not set. Instructor password for this run:");
        println!();
        println!("     {}", cfg.admin_password);
        println!();
        println!(" Set EXAM_ADMIN_PASSWORD to keep it stable across restarts.");
        println!("================================================================");
    }
    if cfg.agent_token.is_none() {
        println!("WARNING: EXAM_AGENT_TOKEN is not set; any client can act as a board.");
    }
    if let Some(dir) = &cfg.data_dir {
        println!("data: {}", dir.display());
    }

    let app = Arc::new(App::new(cfg).expect("open data directory"));
    spawn_background(app.clone());

    let addr = leptos.site_addr;
    let router = http::router(Ctx { leptos, app: app.clone() });
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    println!("listening on http://{addr}");
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown())
        .await
        .expect("serve");

    app.save(true);
    println!("state saved, bye");
}

async fn shutdown() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("ctrl-c handler") };
    #[cfg(unix)]
    let term = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}
