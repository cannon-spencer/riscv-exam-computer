//! Standard Safe Exam Browser config files (`.seb`) and the Config Key.
//!
//! This follows the public SEB developer docs so the files we generate open
//! in official SEB (Windows/macOS/iOS) as well as in our seb-linux fork:
//!
//! - settings are an Apple XML property list with the SEB key names
//!   (`startURL`, `URLFilterRules`, `sendBrowserExamKey`, ...);
//! - the Config Key is SHA256 over the "SEB-JSON" form of those settings
//!   (keys sorted case-insensitively, `originatorVersion` removed, no
//!   whitespace, no slash/backslash escaping, data as base64);
//! - with `sendBrowserExamKey` on, SEB sends
//!   `X-SafeExamBrowser-ConfigKeyHash = sha256(url_without_fragment + config_key)`
//!   with each request, which [`verify_config_key_hash`] checks.
//!
//! The binary container (gzip + `plnd`/`pswd` prefix) needs the `container`
//! feature, see [`container`].

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use plist::{Dictionary, Value};
use sha2::{Digest, Sha256};

use crate::seb::{ExamSpec, domain_rule, effective_domains, normalize_domain};

/// Written as `originatorVersion`; excluded from the Config Key.
pub const ORIGINATOR: &str = concat!("riscv-exam-env_", env!("CARGO_PKG_VERSION"));

/// `sebConfigPurpose`: 0 = start an exam, 1 = configure a client.
const PURPOSE_START_EXAM: i64 = 0;
/// `URLFilterRules[].action`: 0 = block, 1 = allow.
const FILTER_ALLOW: i64 = 1;

/// Everything besides the instructor's [`ExamSpec`] that goes into a file.
#[derive(Debug, Clone, Default)]
pub struct SebOptions {
    /// Start URL the board opens instead of `spec.start_url` (the gate page).
    /// Its host is added to the allow list.
    pub start_url_override: Option<String>,
    /// 32 random bytes, fixed per exam. Part of the settings, so it makes
    /// each exam's Config Key unique even when two specs are identical.
    pub exam_key_salt: Vec<u8>,
    /// Quit password; stored as SHA256 hex in `hashedQuitPassword`.
    pub quit_password: Option<String>,
}

/// Builds the SEB settings dictionary for an exam.
pub fn exam_settings(spec: &ExamSpec, opts: &SebOptions) -> Result<Dictionary, String> {
    let mut domains = effective_domains(spec)?;
    let start_url = match &opts.start_url_override {
        Some(u) => {
            let host = crate::seb::url_host(u)?;
            if !domains.iter().any(|d| host == *d || host.ends_with(&format!(".{d}"))) {
                domains.push(normalize_domain(&host)?);
            }
            u.trim().to_string()
        }
        None => spec.start_url.trim().to_string(),
    };

    let rules: Vec<Value> = domains
        .iter()
        .map(|d| {
            let mut r = Dictionary::new();
            r.insert("action".into(), Value::from(FILTER_ALLOW));
            r.insert("active".into(), Value::from(true));
            r.insert("expression".into(), Value::from(domain_rule(d)));
            r.insert("regex".into(), Value::from(true));
            Value::Dictionary(r)
        })
        .collect();

    let mut s = Dictionary::new();
    let mut put = |k: &str, v: Value| {
        s.insert(k.to_string(), v);
    };
    put("originatorVersion", ORIGINATOR.into());
    put("sebConfigPurpose", PURPOSE_START_EXAM.into());
    put("startURL", start_url.into());
    put("restartExamUseStartURL", true.into());

    // Handshake: send CK/BEK hashes in headers (and the JS API).
    put("sendBrowserExamKey", true.into());
    if !opts.exam_key_salt.is_empty() {
        put("examKeySalt", Value::Data(opts.exam_key_salt.clone()));
    }

    // Lockdown.
    put("URLFilterEnable", true.into());
    put("URLFilterEnableContentFilter", true.into());
    put("URLFilterRules", Value::Array(rules));
    put("allowQuit", spec.allow_quit.into());
    if let Some(pw) = opts.quit_password.as_deref().filter(|p| !p.is_empty()) {
        put("hashedQuitPassword", hex(&Sha256::digest(pw.as_bytes())).into());
    }
    put("downloadAndOpenSebConfig", false.into());
    put("allowDownUploads", false.into());
    put("allowDownloads", false.into());
    put("allowUploads", false.into());
    put("allowSpellCheck", false.into());
    put("allowDeveloperConsole", false.into());
    put("examSessionClearCookiesOnStart", true.into());
    put("examSessionClearCookiesOnEnd", true.into());

    // Window.
    put("enableBrowserWindowToolbar", true.into());
    put("browserWindowAllowAddressBar", false.into());
    put("browserWindowAllowReload", true.into());
    put("showReloadButton", true.into());
    Ok(s)
}

/// Settings as XML plist text (what goes inside the container).
pub fn to_xml(settings: &Dictionary) -> String {
    let mut out = Vec::new();
    Value::Dictionary(settings.clone()).to_writer_xml(&mut out).expect("writing to a Vec cannot fail");
    String::from_utf8(out).expect("plist writer emits UTF-8")
}

/// Parses XML plist settings (an unwrapped `.seb`).
pub fn from_xml(xml: &[u8]) -> Result<Dictionary, String> {
    match Value::from_reader_xml(std::io::Cursor::new(xml)).map_err(|e| format!("not a plist: {e}"))? {
        Value::Dictionary(d) => Ok(d),
        _ => Err("plist root is not a dictionary".into()),
    }
}

// ----- Config Key -------------------------------------------------------------

/// The SEB-JSON string the Config Key is hashed from.
pub fn seb_json(settings: &Dictionary) -> String {
    let mut root = settings.clone();
    root.remove("originatorVersion");
    let mut out = String::new();
    write_dict(&root, &mut out);
    out
}

/// Config Key: lowercase hex SHA256 of [`seb_json`].
pub fn config_key(settings: &Dictionary) -> String {
    hex(&Sha256::digest(seb_json(settings).as_bytes()))
}

/// Value SEB sends in `X-SafeExamBrowser-ConfigKeyHash` (and exposes as
/// `SafeExamBrowser.security.configKey`) for a request to `url`.
pub fn config_key_hash(url: &str, config_key: &str) -> String {
    let url = url.split('#').next().unwrap_or(url);
    hex(&Sha256::digest(format!("{url}{config_key}").as_bytes()))
}

/// Constant-time check of a received hash against the expected one.
pub fn verify_config_key_hash(url: &str, config_key: &str, received: &str) -> bool {
    let want = config_key_hash(url, config_key);
    let got = received.trim().to_ascii_lowercase();
    want.len() == got.len() && want.bytes().zip(got.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// Case-insensitive order; for keys equal ignoring case, lowercase first
/// (the docs' `allowWlan` before `allowWLAN`).
fn key_order(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| b.cmp(a))
}

fn is_empty_dict(v: &Value) -> bool {
    matches!(v, Value::Dictionary(d) if d.is_empty())
}

fn write_dict(d: &Dictionary, out: &mut String) {
    let mut keys: Vec<&String> = d.iter().filter(|(_, v)| !is_empty_dict(v)).map(|(k, _)| k).collect();
    keys.sort_by(|a, b| key_order(a, b));
    out.push('{');
    for (i, k) in keys.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_str(k, out);
        out.push(':');
        write_value(&d[k.as_str()], out);
    }
    out.push('}');
}

fn write_value(v: &Value, out: &mut String) {
    match v {
        Value::Dictionary(d) => write_dict(d, out),
        Value::Array(a) => {
            out.push('[');
            let mut first = true;
            for item in a.iter().filter(|v| !is_empty_dict(v)) {
                if !first {
                    out.push(',');
                }
                first = false;
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Boolean(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Integer(i) => match (i.as_signed(), i.as_unsigned()) {
            (Some(n), _) => out.push_str(&n.to_string()),
            (None, Some(n)) => out.push_str(&n.to_string()),
            _ => out.push('0'),
        },
        // Shortest round-trip form: <real>0.10000000000000001</real> -> 0.1.
        Value::Real(r) => out.push_str(&format_real(*r)),
        Value::String(s) => write_str(s, out),
        Value::Data(bytes) => write_str(&B64.encode(bytes), out),
        Value::Date(d) => write_str(&iso8601(d), out),
        Value::Uid(u) => out.push_str(&u.get().to_string()),
        _ => out.push_str("null"),
    }
}

fn format_real(r: f64) -> String {
    if r.is_finite() { format!("{r}") } else { "0".into() }
}

/// `2026-10-20T12:00:00+00:00`, like PHP's `DateTime::format('c')` in UTC.
fn iso8601(d: &plist::Date) -> String {
    let z = d.to_xml_format(); // "2026-10-20T12:00:00Z"
    z.strip_suffix('Z').map_or(z.clone(), |s| format!("{s}+00:00"))
}

/// JSON string without slash, backslash, or unicode escaping (per the SEB
/// docs). Quotes and control characters are still escaped so the output is
/// unambiguous; SEB settings never contain them in practice.
fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ----- container --------------------------------------------------------------

/// The `.seb` binary container.
///
/// ```text
/// plain:    gzip( "plnd" + gzip(xml) )
/// password: gzip( "pswd" + rncryptor_v2(password, gzip(xml)) )
/// ```
///
/// Password mode matches SEB for Windows and seb-linux: version 2, options 1,
/// PBKDF2-HMAC-SHA1 with 10,000 iterations for both keys, AES-256-CBC with
/// PKCS#7 padding, HMAC-SHA256 over header and ciphertext.
#[cfg(feature = "container")]
pub mod container {
    use std::io::{Read, Write};

    use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
    use flate2::{Compression, read::GzDecoder, write::GzEncoder};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    const ITERATIONS: u32 = 10_000;
    const VERSION: u8 = 2;
    const OPTIONS: u8 = 1;
    const HEADER: usize = 2 + 8 + 8 + 16;
    const MAC_LEN: usize = 32;

    pub fn gzip(data: &[u8]) -> Vec<u8> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).expect("writing to a Vec cannot fail");
        e.finish().expect("writing to a Vec cannot fail")
    }

    pub fn gunzip(data: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        GzDecoder::new(data).read_to_end(&mut out).map_err(|e| format!("gzip: {e}"))?;
        Ok(out)
    }

    fn random<const N: usize>() -> [u8; N] {
        let mut b = [0u8; N];
        getrandom::fill(&mut b).expect("OS random source");
        b
    }

    /// 32 random bytes for `SebOptions::exam_key_salt`.
    pub fn new_salt() -> Vec<u8> {
        random::<32>().to_vec()
    }

    fn derive(password: &str, salt: &[u8]) -> [u8; 32] {
        let mut k = [0u8; 32];
        pbkdf2::pbkdf2_hmac::<sha1::Sha1>(password.as_bytes(), salt, ITERATIONS, &mut k);
        k
    }

    pub fn encrypt(password: &str, plain: &[u8]) -> Vec<u8> {
        let (enc_salt, mac_salt, iv) = (random::<8>(), random::<8>(), random::<16>());
        let enc_key = derive(password, &enc_salt);
        let mac_key = derive(password, &mac_salt);
        let mut out = vec![VERSION, OPTIONS];
        out.extend_from_slice(&enc_salt);
        out.extend_from_slice(&mac_salt);
        out.extend_from_slice(&iv);
        let ct = cbc::Encryptor::<aes::Aes256>::new(&enc_key.into(), &iv.into()).encrypt_padded_vec_mut::<Pkcs7>(plain);
        out.extend_from_slice(&ct);
        let mut mac = Hmac::<Sha256>::new_from_slice(&mac_key).expect("any key length");
        mac.update(&out);
        out.extend_from_slice(&mac.finalize().into_bytes());
        out
    }

    pub fn decrypt(password: &str, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < HEADER + MAC_LEN || data[0] != VERSION || data[1] != OPTIONS {
            return Err("not SEB password-encrypted data".into());
        }
        let (body, tag) = data.split_at(data.len() - MAC_LEN);
        let mac_key = derive(password, &data[10..18]);
        let mut mac = Hmac::<Sha256>::new_from_slice(&mac_key).expect("any key length");
        mac.update(body);
        mac.verify_slice(tag).map_err(|_| "wrong password or corrupted file".to_string())?;
        let enc_key = derive(password, &data[2..10]);
        let iv: [u8; 16] = data[18..34].try_into().unwrap();
        cbc::Decryptor::<aes::Aes256>::new(&enc_key.into(), &iv.into())
            .decrypt_padded_vec_mut::<Pkcs7>(&body[HEADER..])
            .map_err(|_| "bad padding".to_string())
    }

    /// Wraps XML settings into `.seb` bytes, password-encrypted if given.
    pub fn encode(xml: &str, password: Option<&str>) -> Vec<u8> {
        let inner = gzip(xml.as_bytes());
        let mut framed = Vec::with_capacity(inner.len() + 64);
        match password.filter(|p| !p.is_empty()) {
            Some(pw) => {
                framed.extend_from_slice(b"pswd");
                framed.extend_from_slice(&encrypt(pw, &inner));
            }
            None => {
                framed.extend_from_slice(b"plnd");
                framed.extend_from_slice(&inner);
            }
        }
        gzip(&framed)
    }

    /// Opens `.seb` bytes (gzip container, or bare XML) back into XML.
    pub fn decode(bytes: &[u8], password: Option<&str>) -> Result<Vec<u8>, String> {
        let bytes = if bytes.starts_with(&[0x1f, 0x8b]) { gunzip(bytes)? } else { bytes.to_vec() };
        if bytes.starts_with(b"<?xml") || bytes.starts_with(b"<plist") {
            return Ok(bytes);
        }
        let (prefix, rest) = bytes.split_at(4.min(bytes.len()));
        match prefix {
            b"plnd" => gunzip(rest),
            b"pswd" | b"pwcc" => {
                let pw = password.ok_or("this file needs a password")?;
                gunzip(&decrypt(pw, rest)?)
            }
            b"pkhs" | b"phsk" => Err("certificate-encrypted .seb files are not supported".into()),
            _ => Err("unknown .seb format".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ExamSpec {
        ExamSpec {
            start_url: "https://ufl.instructure.com/courses/1/quizzes/9".into(),
            allowed_domains: vec!["ufl.edu".into(), "duosecurity.com".into()],
            allow_quit: false,
            ..Default::default()
        }
    }

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    /// Config Keys of files saved by SEB itself (vectors from Moodle's tests).
    #[test]
    fn config_key_matches_seb_generated_files() {
        let mac = from_xml(&fixture("unencrypted_mac_001.seb")).unwrap();
        assert_eq!(config_key(&mac), "4fa9af8ec8759eb7c680752ef4ee5eaf1a860628608fccae2715d519849f9292");
        let win = from_xml(&fixture("unencrypted_win_223.seb")).unwrap();
        assert_eq!(config_key(&win), "2534e4e9f3188f9f9133bf7cf7b4c5d898292bbd7e8d0230f39d1176636a1431");
    }

    #[test]
    fn empty_settings_key() {
        // sha256("{}"), same as Moodle for an empty config.
        assert_eq!(config_key(&Dictionary::new()), "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a");
    }

    #[test]
    fn originator_version_is_ignored() {
        let mut s = exam_settings(&spec(), &SebOptions::default()).unwrap();
        let a = config_key(&s);
        s.insert("originatorVersion".into(), "something else".into());
        assert_eq!(config_key(&s), a);
        s.insert("allowQuit".into(), true.into());
        assert_ne!(config_key(&s), a);
    }

    #[test]
    fn json_is_sorted_and_unescaped() {
        let mut d = Dictionary::new();
        d.insert("b".into(), "x/y\\.z".into());
        d.insert("allowWLAN".into(), true.into());
        d.insert("allowWlan".into(), false.into());
        d.insert("A".into(), Value::Dictionary(Dictionary::new()));
        d.insert("c".into(), Value::Data(vec![1, 2, 3]));
        assert_eq!(seb_json(&d), r#"{"allowWlan":false,"allowWLAN":true,"b":"x/y\.z","c":"AQID"}"#);
    }

    #[test]
    fn settings_roundtrip_through_xml_keep_their_key() {
        let s = exam_settings(&spec(), &SebOptions { exam_key_salt: vec![7; 32], ..Default::default() }).unwrap();
        let xml = to_xml(&s);
        assert!(xml.contains("<key>startURL</key>"));
        assert_eq!(config_key(&from_xml(xml.as_bytes()).unwrap()), config_key(&s));
    }

    #[test]
    fn salt_makes_keys_unique() {
        let a = exam_settings(&spec(), &SebOptions { exam_key_salt: vec![1; 32], ..Default::default() }).unwrap();
        let b = exam_settings(&spec(), &SebOptions { exam_key_salt: vec![2; 32], ..Default::default() }).unwrap();
        assert_ne!(config_key(&a), config_key(&b));
    }

    #[test]
    fn gate_override_is_allowed_and_used() {
        let opts = SebOptions { start_url_override: Some("https://gate.example.edu/gate/e1".into()), ..Default::default() };
        let s = exam_settings(&spec(), &opts).unwrap();
        assert_eq!(s["startURL"].as_string(), Some("https://gate.example.edu/gate/e1"));
        let rules = s["URLFilterRules"].as_array().unwrap();
        let exprs: Vec<_> = rules.iter().map(|r| r.as_dictionary().unwrap()["expression"].as_string().unwrap()).collect();
        assert!(exprs.iter().any(|e| e.contains(r"gate\.example\.edu")));
        assert!(exprs.iter().any(|e| e.contains(r"instructure\.com")));
    }

    #[test]
    fn request_hash() {
        let ck = "4fa9af8ec8759eb7c680752ef4ee5eaf1a860628608fccae2715d519849f9292";
        let h = config_key_hash("https://x.edu/gate/1", ck);
        assert_eq!(h, config_key_hash("https://x.edu/gate/1#frag", ck));
        assert!(verify_config_key_hash("https://x.edu/gate/1", ck, &h.to_uppercase()));
        assert!(!verify_config_key_hash("https://x.edu/gate/2", ck, &h));
        assert!(!verify_config_key_hash("https://x.edu/gate/1", ck, ""));
    }

    #[test]
    fn quit_password_is_hashed() {
        let s = exam_settings(&spec(), &SebOptions { quit_password: Some("pw".into()), ..Default::default() }).unwrap();
        assert_eq!(
            s["hashedQuitPassword"].as_string(),
            Some("30c952fab122c3f9759f02a6d95c3758b246b4fee239957b2d4fee46e26170c4")
        );
    }

    #[cfg(feature = "container")]
    #[test]
    fn container_roundtrip() {
        let xml = to_xml(&exam_settings(&spec(), &SebOptions::default()).unwrap());
        let plain = container::encode(&xml, None);
        assert_eq!(container::decode(&plain, None).unwrap(), xml.as_bytes());
        let locked = container::encode(&xml, Some("hunter2"));
        assert_eq!(container::decode(&locked, Some("hunter2")).unwrap(), xml.as_bytes());
        assert!(container::decode(&locked, Some("nope")).is_err());
        assert!(container::decode(&locked, None).is_err());
        // Bare XML (what the Moodle fixtures are) also opens.
        assert!(container::decode(&fixture("simpleunencrypted.seb"), None).is_ok());
    }
}
