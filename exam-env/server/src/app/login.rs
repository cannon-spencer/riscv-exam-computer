//! Sign-in page. The form posts to `/auth/login` (a plain Axum handler).

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;

use super::ui::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let query = use_query_map();
    let error = move || match query.with(|q| q.get("e")).as_deref() {
        Some("bad") => Some("That password is not correct."),
        Some("locked") => Some("Too many attempts. Wait five minutes, then try again."),
        _ => None,
    };
    view! {
        <Title text="Sign in · Exam Console"/>
        <main class="login" id="main">
            <div class="login-card">
                <div class="brand login-brand">
                    <Logo/>
                    <span class="brand-text">
                        <span class="brand-name">"Exam Console"</span>
                        <span class="brand-sub">"RISC-V exam boards"</span>
                    </span>
                </div>
                <h1>"Sign in"</h1>
                <p class="muted">"For instructors and proctors running an exam session."</p>
                <form method="post" action="/auth/login" class="form">
                    <label class="field">
                        <span class="field-label">"Instructor password"</span>
                        <input type="password" name="password" required autocomplete="current-password" autofocus
                            aria-invalid=move || error().is_some().to_string()/>
                    </label>
                    {move || error().map(|e| view! { <ErrorNote text=e/> })}
                    <button type="submit" class="btn primary block">"Sign in"</button>
                </form>
                <p class="field-hint login-hint">
                    "The password is set with " <code>"EXAM_ADMIN_PASSWORD"</code>
                    " on the server. If it is not set, the server prints a new one in its log each time it starts."
                </p>
            </div>
        </main>
    }
}
