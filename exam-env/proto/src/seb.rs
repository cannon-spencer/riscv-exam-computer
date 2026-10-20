//! Generate seb-linux JSON configurations from an instructor's exam setup.
//!
//! seb-linux's request filter evaluates block rules, then allow rules, and
//! blocks everything else. Rules are matched against the whole decoded URL.
//! "simplified" rules turn `*` into `.*`, so `*://*.example.com/*` would also
//! match `https://evil.test/?.example.com/`. We therefore emit anchored
//! `regex` rules whose host part cannot contain `/ ? # @`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const MAX_DOMAINS: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExamSpec {
    pub start_url: String,
    pub allowed_domains: Vec<String>,
    pub allow_quit: bool,
}

/// Returns the lowercase host of an `http(s)` URL, rejecting user info and
/// anything that is not a plain DNS name.
pub fn url_host(url: &str) -> Result<String, String> {
    let url = url.trim();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or("Start URL must begin with https://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return Err("Start URL must not contain a user name or password".into());
    }
    let host = match authority.rsplit_once(':') {
        Some((h, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => h,
        Some(_) => return Err("Start URL has an invalid port".into()),
        None => authority,
    };
    normalize_domain(host).map_err(|e| format!("Start URL host: {e}"))
}

/// Accepts `example.com`, `*.example.com`, `.example.com` or a pasted URL and
/// returns the bare lowercase domain.
pub fn normalize_domain(input: &str) -> Result<String, String> {
    let mut d = input.trim();
    if d.contains("://") {
        return url_host(d);
    }
    d = d.trim_start_matches("*.").trim_start_matches('.').trim_end_matches('.');
    let d = d.to_ascii_lowercase();
    if d.is_empty() {
        return Err("empty domain".into());
    }
    if d.len() > 253 {
        return Err(format!("{d}: too long"));
    }
    for label in d.split('.') {
        let ok = !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        if !ok {
            return Err(format!("{d}: not a valid domain name"));
        }
    }
    if !d.contains('.') && d != "localhost" {
        return Err(format!("{d}: needs a dot, like example.com"));
    }
    Ok(d)
}

/// Regex that allows `domain` and its subdomains over http(s) and ws(s).
pub fn domain_rule(domain: &str) -> String {
    let escaped = domain.replace('.', r"\.");
    format!(r"^(https?|wss?)://([^/?#@]*\.)?{escaped}(:[0-9]+)?([/?#].*)?$")
}

/// Parses a textarea of domains (one per line, or comma separated) into a
/// deduplicated list, reporting the first invalid entry.
pub fn parse_domain_list(text: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split(['\n', ',', ' ', '\t']) {
        if raw.trim().is_empty() {
            continue;
        }
        let d = normalize_domain(raw)?;
        if !out.contains(&d) {
            out.push(d);
        }
    }
    if out.len() > MAX_DOMAINS {
        return Err(format!("at most {MAX_DOMAINS} domains"));
    }
    Ok(out)
}

/// Domains the generated config allows: the start URL host first, then the
/// listed ones, skipping any already covered by a parent domain.
pub fn effective_domains(spec: &ExamSpec) -> Result<Vec<String>, String> {
    let mut all = vec![url_host(&spec.start_url)?];
    for d in &spec.allowed_domains {
        all.push(normalize_domain(d)?);
    }
    let covered = |d: &str, by: &str| d == by || d.ends_with(&format!(".{by}"));
    let mut out: Vec<String> = Vec::new();
    for d in all {
        if out.iter().any(|kept| covered(&d, kept)) {
            continue;
        }
        out.retain(|kept| !covered(kept, &d));
        out.push(d);
    }
    Ok(out)
}

/// Builds the seb-linux structured JSON configuration for an exam.
pub fn seb_config(spec: &ExamSpec) -> Result<Value, String> {
    let start_url = spec.start_url.trim();
    let rules: Vec<Value> = effective_domains(spec)?
        .iter()
        .map(|d| json!({ "expression": domain_rule(d), "result": "allow", "type": "regex" }))
        .collect();
    Ok(json!({
        "browser": {
            "startUrl": start_url,
            "useStartUrlAsHomeUrl": true,
            "allowDownloads": false,
            "allowUploads": false,
            "allowConfigurationDownloads": false,
            "deleteCookiesOnStartup": true,
            "deleteCookiesOnShutdown": true,
            "filter": {
                "processMainRequests": true,
                "processContentRequests": true,
                "rules": rules
            },
            "mainWindow": {
                "showToolbar": true,
                "allowAddressBar": false,
                "allowReloading": true,
                "showReloadButton": true,
                "alwaysOnTop": true,
                "allowMinimize": false
            }
        },
        "security": {
            "allowTermination": spec.allow_quit
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors seb-linux `RequestFilter::evaluate` with only allow rules:
    /// case-insensitive regex match against the URL, default block.
    fn allowed(config: &Value, url: &str) -> bool {
        config["browser"]["filter"]["rules"].as_array().unwrap().iter().any(|r| {
            assert_eq!(r["type"], "regex");
            regex::RegexBuilder::new(r["expression"].as_str().unwrap())
                .case_insensitive(true)
                .build()
                .unwrap()
                .is_match(url)
        })
    }

    fn spec(url: &str, domains: &[&str]) -> ExamSpec {
        ExamSpec {
            start_url: url.into(),
            allowed_domains: domains.iter().map(|s| s.to_string()).collect(),
            allow_quit: false,
        }
    }

    #[test]
    fn host_extraction() {
        assert_eq!(url_host("https://UFL.instructure.com/courses/1/quizzes/2").unwrap(), "ufl.instructure.com");
        assert_eq!(url_host("https://a.b.edu:8443/x").unwrap(), "a.b.edu");
        assert!(url_host("ftp://x.com").is_err());
        assert!(url_host("https://user@x.com/").is_err());
        assert!(url_host("javascript:alert(1)").is_err());
        assert!(url_host("https://exa mple.com/").is_err());
    }

    #[test]
    fn domain_normalization() {
        assert_eq!(normalize_domain(" *.Instructure.com ").unwrap(), "instructure.com");
        assert_eq!(normalize_domain(".ufl.edu").unwrap(), "ufl.edu");
        assert_eq!(normalize_domain("https://login.ufl.edu/idp").unwrap(), "login.ufl.edu");
        assert!(normalize_domain("bad_domain.com").is_err());
        assert!(normalize_domain("nodot").is_err());
    }

    #[test]
    fn generated_rules_allow_host_and_subdomains_only() {
        let cfg = seb_config(&spec("https://ufl.instructure.com/courses/1/quizzes/9", &["duosecurity.com"])).unwrap();
        assert_eq!(cfg["browser"]["startUrl"], "https://ufl.instructure.com/courses/1/quizzes/9");
        assert_eq!(cfg["browser"]["filter"]["processMainRequests"], true);
        assert_eq!(cfg["security"]["allowTermination"], false);

        assert!(allowed(&cfg, "https://ufl.instructure.com/"));
        assert!(allowed(&cfg, "https://cdn.ufl.instructure.com/a.js"));
        assert!(allowed(&cfg, "https://api-1234.duosecurity.com/frame"));
        assert!(allowed(&cfg, "https://duosecurity.com"));

        assert!(!allowed(&cfg, "https://evil.test/?u=.duosecurity.com/"));
        assert!(!allowed(&cfg, "https://duosecurity.com@evil.test/"));
        assert!(!allowed(&cfg, "https://notduosecurity.com/"));
        assert!(!allowed(&cfg, "https://ufl.instructure.com.evil.test/"));
        assert!(!allowed(&cfg, "https://evil.test/ufl.instructure.com/"));
        assert!(!allowed(&cfg, "https://instructure.com/"));
        assert!(!allowed(&cfg, "file:///etc/passwd"));
    }

    #[test]
    fn rule_text_is_anchored_regex() {
        assert_eq!(
            domain_rule("ufl.edu"),
            r"^(https?|wss?)://([^/?#@]*\.)?ufl\.edu(:[0-9]+)?([/?#].*)?$"
        );
    }

    #[test]
    fn covered_domains_are_collapsed() {
        let d = effective_domains(&spec("https://ufl.instructure.com/x", &["instructure.com", "a.instructure.com", "ufl.edu"])).unwrap();
        assert_eq!(d, vec!["instructure.com", "ufl.edu"]);
    }

    #[test]
    fn domain_list_parsing() {
        let d = parse_domain_list("instructure.com\n  *.ufl.edu, duosecurity.com\n\ninstructure.com").unwrap();
        assert_eq!(d, vec!["instructure.com", "ufl.edu", "duosecurity.com"]);
        assert!(parse_domain_list("good.com\nbad domain!").is_err());
    }

    #[test]
    fn allow_quit_maps_to_allow_termination() {
        let mut s = spec("https://x.edu/", &[]);
        s.allow_quit = true;
        assert_eq!(seb_config(&s).unwrap()["security"]["allowTermination"], true);
    }
}
