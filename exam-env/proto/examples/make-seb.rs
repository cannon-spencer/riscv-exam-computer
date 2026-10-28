//! Writes a .seb for testing outside the exam server, and prints its Config Key.
//!
//!   cargo run -p proto --features container --example make-seb -- \
//!       https://ufl.instructure.com/courses/1/quizzes/2 out.seb [domain ...]
//!
//! Set SEB_PASSWORD to password-encrypt, SEB_START_URL to override the start
//! page (e.g. a gate URL), SEB_XML=1 to write plain XML instead.

use proto::seb::ExamSpec;
use proto::seb_file::{SebOptions, config_key, container, exam_settings, to_xml};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: make-seb <start-url> <out.seb> [allowed-domain ...]";
    let start_url = args.next().ok_or(usage)?;
    let out = args.next().ok_or(usage)?;
    let spec = ExamSpec { start_url, allowed_domains: args.collect(), ..Default::default() };
    let opts = SebOptions {
        start_url_override: std::env::var("SEB_START_URL").ok(),
        exam_key_salt: container::new_salt(),
        quit_password: None,
    };
    let settings = exam_settings(&spec, &opts)?;
    let xml = to_xml(&settings);
    let bytes = if std::env::var_os("SEB_XML").is_some() {
        xml.into_bytes()
    } else {
        container::encode(&xml, std::env::var("SEB_PASSWORD").ok().as_deref())
    };
    std::fs::write(&out, bytes).map_err(|e| format!("{out}: {e}"))?;
    println!("{}", config_key(&settings));
    Ok(())
}
