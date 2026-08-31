use std::thread;
use std::time::Duration;

fn main() {
    let name = std::env::var("HOSTNAME")
        .unwrap_or_else(|_| "unknown".into());

    let base = std::env::var("CONTROL_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8000".into());

    let url = format!("{base}/heartbeat");
    let body = format!(r#"{{"host":"{name}","state":"idle"}}"#);

    loop {
        println!("heartbeat host={name}");

        match ureq::post(&url)
            .header("Content-Type", "application/json")
            .send(&body)
        {
            Ok(_) => {}
            Err(e) => eprintln!("heartbeat failed: {e}"),
        }

        thread::sleep(Duration::from_secs(5));
    }
}
