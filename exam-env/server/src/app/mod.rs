//! The instructor console UI (Leptos, server-rendered and hydrated).

pub mod activity;
pub mod api;
pub mod board;
pub mod exams;
pub mod fleet;
pub mod login;
pub mod ui;

use leptos::prelude::*;
use leptos_meta::{MetaTags, Title, provide_meta_context};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_location;
use leptos_router::path;

use crate::model::SessionInfo;

/// The HTML document. Server only; the body is hydrated by [`App`].
pub fn shell(options: LeptosOptions) -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        leptos::nonce::provide_nonce();
        if let (Some(nonce), Some(res)) = (leptos::nonce::use_nonce(), use_context::<leptos_axum::ResponseOptions>()) {
            let csp = crate::backend::http::page_csp(&nonce);
            res.insert_header(
                axum::http::header::CONTENT_SECURITY_POLICY,
                axum::http::HeaderValue::from_str(&csp).expect("ascii csp"),
            );
        }
    }
    let theme = ui::theme_cookie();
    view! {
        <!DOCTYPE html>
        <html lang="en" data-theme=theme>
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="color-scheme" content="dark"/>
                <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>
                <link rel="preload" href="/fonts/ibm-plex-mono-latin-400-normal.woff2" r#as="font" r#type="font/woff2" crossorigin="anonymous"/>
                <link rel="stylesheet" href="/pkg/console.css"/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/") view=fleet::FleetPage/>
                <Route path=path!("/boards/:host") view=board::BoardPage/>
                <Route path=path!("/exams") view=exams::ExamsPage/>
                <Route path=path!("/activity") view=activity::ActivityPage/>
                <Route path=path!("/login") view=login::LoginPage/>
            </Routes>
        </Router>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(res) = use_context::<leptos_axum::ResponseOptions>() {
        res.set_status(axum::http::StatusCode::NOT_FOUND);
    }
    view! {
        <Shell>
            <Title text="Not found Â· Exam Console"/>
            <ui::EmptyState title="There is nothing at this address">
                <a class="btn secondary" href="/">"Go to the fleet"</a>
            </ui::EmptyState>
        </Shell>
    }
}

#[component]
fn NavLink(href: &'static str, label: &'static str, icon: ui::Icon) -> impl IntoView {
    let location = use_location();
    let current = move || {
        let p = location.pathname.get();
        let on = if href == "/" { p == "/" || p.starts_with("/boards/") } else { p.starts_with(href) };
        on.then_some("page")
    };
    view! {
        <a class="nav-link" href=href aria-current=current>
            <ui::IconSvg icon/>
            <span>{label}</span>
        </a>
    }
}

/// Page frame: sidebar (top bar on narrow screens) and content column.
#[component]
pub fn Shell(children: Children) -> impl IntoView {
    let info = Resource::new(|| (), |_| api::get_session_info());
    view! {
        <a class="skip" href="#main">"Skip to content"</a>
        <div class="shell">
            <aside class="side">
                <a class="brand" href="/">
                    <ui::Logo/>
                    <span class="brand-text">
                        <span class="brand-name">"Exam Console"</span>
                        <span class="brand-sub">"RISC-V exam boards"</span>
                    </span>
                </a>
                <nav class="nav" aria-label="Main">
                    <NavLink href="/" label="Fleet" icon=ui::Icon::Grid/>
                    <NavLink href="/exams" label="Exams" icon=ui::Icon::Doc/>
                    <NavLink href="/activity" label="Activity" icon=ui::Icon::List/>
                </nav>
                <div class="side-foot">
                    <form method="post" action="/auth/logout">
                        <button class="btn ghost small" type="submit">
                            <ui::IconSvg icon=ui::Icon::Exit/>
                            "Sign out"
                        </button>
                    </form>
                </div>
            </aside>
            <main class="main" id="main" tabindex="-1">
                <Suspense fallback=|| ()>
                    {move || Suspend::new(async move {
                        let info: Option<SessionInfo> = info.await.ok();
                        info.filter(|i| i.agent_token_missing).map(|_| view! { <TokenBanner/> })
                    })}
                </Suspense>
                {children()}
            </main>
        </div>
    }
}

#[component]
fn TokenBanner() -> impl IntoView {
    view! {
        <div class="banner warn" role="alert">
            <ui::IconSvg icon=ui::Icon::Warn/>
            <div>
                <strong>"Boards are not authenticated."</strong>
                " Anyone who can reach this server can pose as a board, receive exam settings, or fill the fleet with fake boards. "
                "Set " <code>"EXAM_AGENT_TOKEN"</code> " on the server and in each board's agent environment, then restart both."
            </div>
        </div>
    }
}
