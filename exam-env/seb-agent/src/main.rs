use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/home/orangepi".into())
}

fn start_script() -> String {
    format!("{}/start-seb.sh", home())
}

fn stop_script() -> String {
    format!("{}/stop-seb.sh", home())
}

fn running(child: &mut Option<Child>) -> bool {
    let Some(c) = child.as_mut() else {
        return false;
    };
    match c.try_wait() {
        Ok(None) => true,
        _ => {
            *child = None;
            false
        }
    }
}

fn start_seb(child: &mut Option<Child>) {
    if running(child) {
        return;
    }
    let script = start_script();
    match Command::new("sudo")
        .args(["-n", &script])
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(c) => {
            println!("started {script}");
            *child = Some(c);
        }
        Err(e) => eprintln!("start seb failed: {e}"),
    }
}

fn stop_seb(child: &mut Option<Child>) {
    let script = stop_script();
    match Command::new("sudo").args(["-n", &script]).status() {
        Ok(st) if st.success() => println!("stopped seb"),
        Ok(st) => eprintln!("stop seb exit {st}"),
        Err(e) => eprintln!("stop seb failed: {e}"),
    }
    if let Some(mut c) = child.take() {
        let _ = c.wait();
    }
}

fn main() {
    let name = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into());
    let url = "https://riscv-exam-computer.download/heartbeat";
    let mut seb = None;

    loop {
        let state = if running(&mut seb) { "running" } else { "idle" };
        let body = format!(r#"{{"host":"{name}","state":"{state}"}}"#);
        println!("heartbeat host={name} state={state}");

        match ureq::post(url)
            .header("Content-Type", "application/json")
            .send(&body)
        {
            Ok(resp) => match resp.into_body().read_to_string() {
                Ok(text) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        match v.get("job").and_then(|j| j.as_str()) {
                            Some("start") => start_seb(&mut seb),
                            Some("stop") => stop_seb(&mut seb),
                            _ => {}
                        }
                    }
                }
                Err(e) => eprintln!("heartbeat body: {e}"),
            },
            Err(e) => eprintln!("heartbeat failed: {e}"),
        }

        thread::sleep(Duration::from_secs(5));
    }
}
