//! Shared components and formatting helpers. See `exam-env/DESIGN.md`.

use leptos::prelude::*;

use crate::model::{Actor, EventView, Level, Status};

// ----- icons --------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Grid,
    Doc,
    List,
    Exit,
    Warn,
    Play,
    Stop,
    Trash,
    Back,
    Info,
    Close,
}

/// 16px line icon drawn with `currentColor`. Decorative unless labelled.
#[component]
pub fn IconSvg(icon: Icon) -> impl IntoView {
    let body = match icon {
        Icon::Grid => view! { <path d="M2.5 2.5h4.5v4.5h-4.5zM9 2.5h4.5v4.5H9zM2.5 9h4.5v4.5h-4.5zM9 9h4.5v4.5H9z"/> }.into_any(),
        Icon::Doc => view! { <path d="M4 1.5h5.5L12.5 4.5v10h-8.5zM9.5 1.5v3h3M6 8h5M6 10.5h5"/> }.into_any(),
        Icon::List => view! { <path d="M5.5 4h8M5.5 8h8M5.5 12h8M2.5 4h.5M2.5 8h.5M2.5 12h.5"/> }.into_any(),
        Icon::Exit => view! { <path d="M6.5 2.5h-4v11h4M10.5 5l3 3-3 3M13.5 8h-7"/> }.into_any(),
        Icon::Warn => view! { <path d="M8 1.8l6.5 11.7h-13zM8 6.2v3.3M8 11.4v.4"/> }.into_any(),
        Icon::Play => view! { <path d="M4.5 2.8v10.4l8.5-5.2z"/> }.into_any(),
        Icon::Stop => view! { <path d="M3.5 3.5h9v9h-9z"/> }.into_any(),
        Icon::Trash => view! { <path d="M2.5 4h11M6 4V2.5h4V4M4 4l.7 9.5h6.6L12 4"/> }.into_any(),
        Icon::Back => view! { <path d="M10 3L5 8l5 5"/> }.into_any(),
        Icon::Info => view! { <path d="M8 1.8a6.2 6.2 0 1 0 0 12.4A6.2 6.2 0 0 0 8 1.8zM8 7.2v4M8 4.8v.4"/> }.into_any(),
        Icon::Close => view! { <path d="M3.5 3.5l9 9M12.5 3.5l-9 9"/> }.into_any(),
    };
    view! {
        <svg class="icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"
            fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            {body}
        </svg>
    }
}

#[component]
pub fn Logo() -> impl IntoView {
    // A board outline with a status dot: the product in one glyph.
    view! {
        <svg class="logo" viewBox="0 0 32 32" width="32" height="32" aria-hidden="true">
            <rect x="3" y="6" width="26" height="20" fill="none" stroke="currentColor" stroke-width="2"/>
            <path d="M9 2v4M16 2v4M23 2v4M9 26v4M16 26v4M23 26v4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <rect x="9" y="11" width="9" height="10" fill="currentColor" opacity=".18"/>
            <rect x="20" y="13" width="6" height="6" class="logo-dot"/>
        </svg>
    }
}

// ----- status ---------------------------------------------------------------------

/// Shape that encodes the status without color (see DESIGN.md).
#[component]
pub fn StatusIcon(status: Status) -> impl IntoView {
    let shape = match status {
        Status::Running => view! {
            <rect class="pulse" x="1.5" y="1.5" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1"/>
            <rect x="3.5" y="3.5" width="9" height="9" fill="currentColor"/>
        }
        .into_any(),
        Status::Starting => view! {
            <rect x="2.5" y="2.5" width="11" height="11" fill="none" stroke="currentColor" stroke-width="2" opacity=".25"/>
            <path class="spin" d="M2.5 2.5h11v11" fill="none" stroke="currentColor" stroke-width="2"/>
        }
        .into_any(),
        Status::Stopping => view! {
            <rect x="2" y="2" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5"/>
            <rect x="5.5" y="5.5" width="5" height="5" fill="currentColor"/>
        }
        .into_any(),
        Status::Idle => view! {
            <rect x="3" y="3" width="10" height="10" fill="none" stroke="currentColor" stroke-width="2"/>
        }
        .into_any(),
        Status::Error => view! {
            <path d="M8 1.8l6.6 11.9H1.4z" fill="currentColor"/>
            <path d="M8 6v3.6M8 11.3v.2" stroke="var(--surface)" stroke-width="1.8"/>
        }
        .into_any(),
        Status::Offline => view! {
            <rect x="2.5" y="2.5" width="11" height="11" fill="none" stroke="currentColor" stroke-width="1.6" stroke-dasharray="2.4 2.4"/>
        }
        .into_any(),
    };
    view! {
        <svg class="status-icon" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">{shape}</svg>
    }
}

#[component]
pub fn StatusBadge(#[prop(into)] status: Signal<Status>) -> impl IntoView {
    view! {
        <span class=move || format!("badge st-{}", status.get().slug())>
            {move || view! { <StatusIcon status=status.get()/> }}
            <span>{move || status.get().word()}</span>
        </span>
    }
}

// ----- formatting -----------------------------------------------------------------

pub fn fmt_ago(secs: u64) -> String {
    match secs {
        0..=4 => "just now".into(),
        5..=59 => format!("{secs} s ago"),
        60..=3599 => format!("{} min ago", secs / 60),
        3600..=86_399 => format!("{} h ago", secs / 3600),
        _ => format!("{} d ago", secs / 86_400),
    }
}

pub fn fmt_uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    if d > 0 {
        format!("{d} d {h} h")
    } else if h > 0 {
        format!("{h} h {m} min")
    } else {
        format!("{m} min")
    }
}

pub fn fmt_temp(c: f32) -> String {
    format!("{c:.1} °C")
}

pub fn fmt_kb(kb: u64) -> String {
    let gb = kb as f64 / 1_048_576.0;
    if gb >= 1.0 { format!("{gb:.1} GB") } else { format!("{} MB", kb / 1024) }
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// (year, month 1-12, day, h, m, s) in UTC. Howard Hinnant's civil_from_days.
fn utc_parts(ms: i64) -> (i64, usize, i64, i64, i64, i64) {
    let secs = ms.div_euclid(1000);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m as usize, d, rem / 3600, rem % 3600 / 60, rem % 60)
}

fn fmt_parts(mo: usize, d: i64, h: i64, mi: i64, s: i64, date: bool, zone: &str) -> String {
    let time = format!("{h:02}:{mi:02}:{s:02}{zone}");
    if date { format!("{} {d}, {time}", MONTHS[mo - 1]) } else { time }
}

pub fn fmt_utc(ms: i64, date: bool) -> String {
    let (_, mo, d, h, mi, s) = utc_parts(ms);
    fmt_parts(mo, d, h, mi, s, date, " UTC")
}

pub fn iso_utc(ms: i64) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(ms);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Renders UTC on the server, then switches to the browser's local time.
#[component]
pub fn LocalTime(at_ms: i64, #[prop(optional)] date: bool) -> impl IntoView {
    let text = RwSignal::new(fmt_utc(at_ms, date));
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let t = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(at_ms as f64));
        text.set(fmt_parts(
            t.get_month() as usize + 1,
            t.get_date() as i64,
            t.get_hours() as i64,
            t.get_minutes() as i64,
            t.get_seconds() as i64,
            date,
            "",
        ));
    });
    view! { <time class="mono" datetime=iso_utc(at_ms)>{move || text.get()}</time> }
}

// ----- small widgets ----------------------------------------------------------------

#[component]
pub fn Sparkline(values: Vec<f32>, #[prop(into)] label: String) -> impl IntoView {
    if values.len() < 2 {
        return view! { <span class="spark-empty">"Collecting samples…"</span> }.into_any();
    }
    let (w, h, pad) = (160.0_f32, 36.0_f32, 3.0_f32);
    let lo = values.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = (hi - lo).max(1.0);
    let step = w / (values.len() - 1) as f32;
    let pts: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{:.1},{:.1}", i as f32 * step, pad + (h - 2.0 * pad) * (1.0 - (v - lo) / span)))
        .collect();
    let last = pts.last().cloned().unwrap_or_default();
    let (lx, ly) = last.split_once(',').unwrap_or(("0", "0"));
    let aria = format!("{label}: low {lo:.1} °C, high {hi:.1} °C over the last {} samples", values.len());
    view! {
        <svg class="spark" viewBox=format!("0 0 {w} {h}") preserveAspectRatio="none" role="img" aria-label=aria>
            <polyline points=pts.join(" ") fill="none" stroke="currentColor" stroke-width="1.5" vector-effect="non-scaling-stroke"/>
            <circle cx=lx.to_string() cy=ly.to_string() r="2.5" fill="currentColor"/>
        </svg>
    }
    .into_any()
}

#[component]
pub fn Meter(pct: u8, #[prop(into)] label: String) -> impl IntoView {
    let level = match pct {
        0..=69 => "ok",
        70..=89 => "high",
        _ => "full",
    };
    view! {
        <span class=format!("meter {level}") role="img" aria-label=format!("{label}: {pct}% used")>
            <span class="meter-fill" style=format!("width:{}%", pct.min(100))></span>
        </span>
    }
}

#[component]
pub fn EmptyState(#[prop(into)] title: String, #[prop(optional)] children: Option<Children>) -> impl IntoView {
    view! {
        <section class="empty">
            <svg viewBox="0 0 120 80" width="120" height="80" aria-hidden="true" class="empty-art">
                <rect x="14" y="16" width="56" height="40" fill="none" stroke="currentColor" stroke-width="2"/>
                <path d="M26 8v8M42 8v8M58 8v8M26 56v8M42 56v8M58 56v8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                <circle cx="56" cy="36" r="5" fill="none" stroke="currentColor" stroke-width="2" stroke-dasharray="3 3"/>
                <path d="M78 40h10M92 40h4M100 40h4" stroke="currentColor" stroke-width="2" stroke-linecap="round" opacity=".6"/>
            </svg>
            <h2>{title}</h2>
            {children.map(|c| view! { <div class="empty-body">{c()}</div> })}
        </section>
    }
}

/// Two-step destructive button: the first click arms it for 4 seconds.
#[component]
pub fn ConfirmButton(
    #[prop(into)] label: Signal<String>,
    #[prop(into)] confirm: Signal<String>,
    on_confirm: Callback<()>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(default = Icon::Stop)] icon: Icon,
    #[prop(default = "btn danger")] class: &'static str,
) -> impl IntoView {
    let armed = RwSignal::new(false);
    let click = move |_| {
        if armed.get_untracked() {
            armed.set(false);
            on_confirm.run(());
        } else {
            armed.set(true);
            set_timeout(move || armed.set(false), std::time::Duration::from_secs(4));
        }
    };
    view! {
        <button type="button" class=move || if armed.get() { format!("{class} armed") } else { class.to_string() }
            disabled=move || disabled.get() on:click=click
            on:blur=move |_| armed.set(false)>
            <IconSvg icon/>
            <span>{move || if armed.get() { confirm.get() } else { label.get() }}</span>
        </button>
    }
}

#[component]
pub fn ErrorNote(#[prop(into)] text: String) -> impl IntoView {
    view! {
        <p class="error-note" role="alert"><IconSvg icon=Icon::Warn/><span>{text}</span></p>
    }
}

#[component]
pub fn Timeline(events: Vec<EventView>, #[prop(optional)] show_host: bool) -> impl IntoView {
    if events.is_empty() {
        return view! { <p class="muted">"Nothing recorded yet."</p> }.into_any();
    }
    view! {
        <ol class="timeline">
            {events.into_iter().map(|e| {
                let (actor, actor_class) = match e.actor {
                    Actor::Instructor => ("Instructor", "actor instructor"),
                    Actor::Board => ("Board", "actor board"),
                    Actor::System => ("System", "actor system"),
                };
                let level = match e.level { Level::Info => "info", Level::Warn => "warn", Level::Error => "error" };
                let level_word = match e.level { Level::Info => None, Level::Warn => Some("Warning: "), Level::Error => Some("Error: ") };
                let host = e.host.clone().filter(|_| show_host);
                view! {
                    <li class=format!("tl-row lv-{level}")>
                        <LocalTime at_ms=e.at_ms date=true/>
                        <span class=actor_class>{actor}</span>
                        <span class="tl-text">
                            {(e.level != Level::Info).then(|| view! { <IconSvg icon=Icon::Warn/> })}
                            <span class="sr-only">{level_word}</span>
                            {host.map(|h| view! { <a class="mono host-link" href=format!("/boards/{h}")>{h.clone()}</a> " " })}
                            {e.text}
                        </span>
                    </li>
                }
            }).collect_view()}
        </ol>
    }
    .into_any()
}

// ----- theme ----------------------------------------------------------------------

/// `light`, `dark`, or `None` for "follow the system".
pub fn theme_cookie() -> Option<String> {
    #[cfg(feature = "ssr")]
    let raw: Option<String> = use_context::<axum::http::request::Parts>()
        .and_then(|p| p.headers.get(axum::http::header::COOKIE)?.to_str().ok().map(String::from));
    #[cfg(feature = "hydrate")]
    let raw: Option<String> = {
        use wasm_bindgen::JsCast;
        document().dyn_into::<web_sys::HtmlDocument>().ok().and_then(|d| d.cookie().ok())
    };
    #[cfg(not(any(feature = "ssr", feature = "hydrate")))]
    let raw: Option<String> = None;
    raw?.split(';').find_map(|kv| {
        let (k, v) = kv.trim().split_once('=')?;
        (k == "theme" && matches!(v, "light" | "dark")).then(|| v.to_string())
    })
}

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let theme = RwSignal::new(theme_cookie().unwrap_or_else(|| "auto".into()));
    let set = move |t: &'static str| {
        theme.set(t.to_string());
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;
            let doc = document();
            if let Some(root) = doc.document_element() {
                let _ = if t == "auto" { root.remove_attribute("data-theme") } else { root.set_attribute("data-theme", t) };
            }
            if let Ok(html) = doc.dyn_into::<web_sys::HtmlDocument>() {
                let cookie = if t == "auto" {
                    "theme=; Path=/; Max-Age=0; SameSite=Strict".to_string()
                } else {
                    format!("theme={t}; Path=/; Max-Age=31536000; SameSite=Strict")
                };
                let _ = html.set_cookie(&cookie);
            }
        }
    };
    let opt = move |value: &'static str, label: &'static str| {
        view! {
            <button type="button" aria-pressed=move || (theme.get() == value).to_string() on:click=move |_| set(value)>
                {label}
            </button>
        }
    };
    view! {
        <div class="seg theme" role="group" aria-label="Color theme">
            {opt("auto", "Auto")}
            {opt("light", "Light")}
            {opt("dark", "Dark")}
        </div>
    }
}

// ----- polling --------------------------------------------------------------------

/// Calls `fetch` every `ms` milliseconds in the browser and stores successful
/// results in `sink`. `lost` is set while requests fail. No-op on the server.
pub fn poll_every<T, F, Fut>(ms: u64, fetch: F, sink: RwSignal<Option<T>>, lost: RwSignal<bool>)
where
    T: Send + Sync + 'static,
    F: Fn() -> Fut + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    #[cfg(feature = "hydrate")]
    {
        use std::cell::Cell;
        use std::rc::Rc;
        let fetch = Rc::new(fetch);
        let busy = Rc::new(Cell::new(false));
        Effect::new(move |_| {
            let fetch = fetch.clone();
            let busy = busy.clone();
            let handle = set_interval_with_handle(
                move || {
                    if busy.get() {
                        return;
                    }
                    busy.set(true);
                    let fut = fetch();
                    let busy = busy.clone();
                    leptos::task::spawn_local(async move {
                        match fut.await {
                            Ok(v) => {
                                sink.set(Some(v));
                                lost.set(false);
                            }
                            Err(_) => lost.set(true),
                        }
                        busy.set(false);
                    });
                },
                std::time::Duration::from_millis(ms),
            )
            .ok();
            on_cleanup(move || {
                if let Some(h) = handle {
                    h.clear();
                }
            });
        });
    }
    #[cfg(not(feature = "hydrate"))]
    let _ = (ms, fetch, sink, lost);
}

#[component]
pub fn ConnectionPill(lost: RwSignal<bool>) -> impl IntoView {
    view! {
        <span class="live-pill" class:lost=move || lost.get() role="status">
            <span class="live-dot" aria-hidden="true"></span>
            {move || if lost.get() { "Reconnecting… reload if this persists" } else { "Live" }}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formatting() {
        // 2026-10-19T14:02:11Z
        assert_eq!(iso_utc(1_792_418_531_000), "2026-10-19T14:02:11Z");
        assert_eq!(fmt_utc(1_792_418_531_000, true), "Oct 19, 14:02:11 UTC");
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(951_782_400_000), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn durations() {
        assert_eq!(fmt_ago(3), "just now");
        assert_eq!(fmt_ago(75), "1 min ago");
        assert_eq!(fmt_uptime(93_784), "1 d 2 h");
        assert_eq!(fmt_uptime(3_700), "1 h 1 min");
        assert_eq!(fmt_kb(4_000_000), "3.8 GB");
    }
}
