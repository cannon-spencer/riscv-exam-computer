//! Exams: create (with a live preview of the seb-linux JSON), list, delete.

use leptos::prelude::*;
use leptos_meta::Title;
use proto::seb::{ExamSpec, effective_domains, parse_domain_list, seb_config, url_host};

use super::Shell;
use super::api::{create_exam, delete_exam, error_text, list_exams};
use super::ui::*;
use crate::model::ExamView;

/// Starting points for UF Canvas quizzes. Verify against a real exam: the
/// filter blocks every request that no rule allows.
const SUGGESTED: [(&str, &str); 5] = [
    ("instructure.com", "Canvas"),
    ("inscloudgate.net", "Canvas files"),
    ("canvas-user-content.com", "Canvas user content"),
    ("ufl.edu", "UF login"),
    ("duosecurity.com", "Duo two-factor"),
];

#[component]
pub fn ExamsPage() -> impl IntoView {
    let initial = Resource::new(|| (), |_| list_exams());
    view! {
        <Shell>
            <Title text="Exams · Exam Console"/>
            <header class="page-head">
                <div>
                    <h1>"Exams"</h1>
                    <p class="sub">"An exam is a start page plus the sites students may reach. Boards receive it as a Safe Exam Browser config when you start them."</p>
                </div>
            </header>
            <Suspense fallback=|| view! { <p class="loading">"Loading exams…"</p> }>
                {move || Suspend::new(async move {
                    match initial.await {
                        Ok(list) => view! { <ExamsBody initial=list/> }.into_any(),
                        Err(e) => view! { <ErrorNote text=error_text(&e)/> }.into_any(),
                    }
                })}
            </Suspense>
        </Shell>
    }
}

fn data_url(json: &str) -> String {
    let mut out = String::from("data:application/json;charset=utf-8,");
    for b in json.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~:,{}[]\"".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn file_name(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");
    format!("{}.json", if slug.is_empty() { "exam" } else { &slug })
}

#[component]
fn ExamsBody(initial: Vec<ExamView>) -> impl IntoView {
    let exams = RwSignal::new(initial);
    let refresh = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = list_exams().await {
                exams.set(list);
            }
        });
    };

    let name = RwSignal::new(String::new());
    let url = RwSignal::new(String::new());
    let domains = RwSignal::new(String::new());
    let quit = RwSignal::new(false);
    let touched = RwSignal::new(false);

    let url_error = Memo::new(move |_| {
        let u = url.get();
        if u.trim().is_empty() { None } else { url_host(&u).err() }
    });
    let domain_error = Memo::new(move |_| parse_domain_list(&domains.get()).err());
    let spec = Memo::new(move |_| -> Result<ExamSpec, String> {
        Ok(ExamSpec { start_url: url.get().trim().to_string(), allowed_domains: parse_domain_list(&domains.get())?, allow_quit: quit.get() })
    });
    let preview = Memo::new(move |_| {
        let s = spec.get()?;
        let v = seb_config(&s)?;
        Ok::<_, String>((effective_domains(&s)?, serde_json::to_string_pretty(&v).unwrap_or_default()))
    });

    let create = Action::new(move |(n, u, d, q): &(String, String, String, bool)| {
        let (n, u, d, q) = (n.clone(), u.clone(), d.clone(), *q);
        async move { create_exam(n, u, d, q).await.map_err(|e| error_text(&e)) }
    });
    let result = RwSignal::new(None::<Result<String, String>>);
    Effect::new(move |_| {
        if let Some(r) = create.value().get() {
            if r.is_ok() {
                name.set(String::new());
                url.set(String::new());
                domains.set(String::new());
                quit.set(false);
                touched.set(false);
                refresh();
            }
            result.set(Some(r.map(|_| "Exam created. Choose it on the Fleet page to start it.".into())));
        }
    });
    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        touched.set(true);
        if name.get_untracked().trim().is_empty() || url.get_untracked().trim().is_empty() || preview.get_untracked().is_err() {
            return;
        }
        create.dispatch((name.get_untracked(), url.get_untracked(), domains.get_untracked(), quit.get_untracked()));
    };
    let add_domain = move |d: &'static str| {
        domains.update(|text| {
            if !text.lines().any(|l| l.trim() == d) {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(d);
                text.push('\n');
            }
        });
    };

    let delete = Action::new(move |id: &String| {
        let id = id.clone();
        async move { delete_exam(id).await.map_err(|e| error_text(&e)) }
    });
    let delete_err = RwSignal::new(None::<String>);
    Effect::new(move |_| match delete.value().get() {
        Some(Ok(())) => refresh(),
        Some(Err(e)) => delete_err.set(Some(e)),
        None => {}
    });

    view! {
        <div class="exams-layout">
            <section class="panel">
                <h2>"New exam"</h2>
                <form class="form" on:submit=submit novalidate>
                    <label class="field">
                        <span class="field-label">"Name"</span>
                        <input type="text" maxlength="80" required autocomplete="off" placeholder="COP 3502 Quiz 4"
                            prop:value=move || name.get() on:input=move |ev| name.set(event_target_value(&ev))
                            aria-invalid=move || (touched.get() && name.with(|n| n.trim().is_empty())).to_string()/>
                        <Show when=move || touched.get() && name.with(|n| n.trim().is_empty())>
                            <span class="field-error"><IconSvg icon=Icon::Warn/>"Give the exam a name."</span>
                        </Show>
                    </label>

                    <label class="field">
                        <span class="field-label">"Start URL"</span>
                        <input type="url" required autocomplete="off" spellcheck="false" class="mono"
                            placeholder="https://ufl.instructure.com/courses/123/quizzes/456"
                            prop:value=move || url.get() on:input=move |ev| url.set(event_target_value(&ev))
                            aria-invalid=move || url_error.with(|e| e.is_some()).to_string()/>
                        <span class="field-hint">"Paste the Canvas quiz URL. Its site is always allowed, including subdomains."</span>
                        {move || url_error.get().map(|e| view! { <span class="field-error"><IconSvg icon=Icon::Warn/>{e}</span> })}
                        <Show when=move || touched.get() && url.with(|u| u.trim().is_empty())>
                            <span class="field-error"><IconSvg icon=Icon::Warn/>"Paste the quiz URL."</span>
                        </Show>
                    </label>

                    <label class="field">
                        <span class="field-label">"Other allowed sites"</span>
                        <textarea rows="5" spellcheck="false" class="mono" placeholder="ufl.edu\nduosecurity.com"
                            prop:value=move || domains.get() on:input=move |ev| domains.set(event_target_value(&ev))
                            aria-invalid=move || domain_error.with(|e| e.is_some()).to_string()></textarea>
                        <span class="field-hint">"One domain per line. Subdomains are included. Everything else is blocked, including images and scripts from other sites."</span>
                        {move || domain_error.get().map(|e| view! { <span class="field-error"><IconSvg icon=Icon::Warn/>{e}</span> })}
                    </label>
                    <div class="chips" role="group" aria-label="Suggested domains">
                        <span class="muted small">"Common for UF Canvas:"</span>
                        {SUGGESTED.into_iter().map(|(d, what)| view! {
                            <button type="button" class="chip" title=what on:click=move |_| add_domain(d)>
                                "+ "<span class="mono">{d}</span>
                            </button>
                        }).collect_view()}
                    </div>

                    <label class="check-field">
                        <input type="checkbox" prop:checked=move || quit.get() on:change=move |ev| quit.set(event_target_checked(&ev))/>
                        <span>
                            <span class="field-label">"Students may quit Safe Exam Browser"</span>
                            <span class="field-hint">"When off, only Stop in this console ends the exam on a board."</span>
                        </span>
                    </label>

                    <div class="form-actions">
                        <button type="submit" class="btn primary" disabled=move || create.pending().get()>"Create exam"</button>
                        {move || result.get().map(|r| match r {
                            Ok(m) => view! { <p class="ok-note" role="status">{m}</p> }.into_any(),
                            Err(e) => view! { <ErrorNote text=e/> }.into_any(),
                        })}
                    </div>
                </form>
            </section>

            <section class="panel preview">
                <h2>"What boards will receive"</h2>
                {move || match preview.get() {
                    _ if url.with(|u| u.trim().is_empty()) => view! {
                        <p class="muted">"Paste a start URL to preview the generated Safe Exam Browser config."</p>
                    }.into_any(),
                    Err(e) => view! { <ErrorNote text=e/> }.into_any(),
                    Ok((allowed, json)) => view! {
                        <p class="small muted">"Allowed sites (and their subdomains):"</p>
                        <ul class="chips static">
                            {allowed.into_iter().map(|d| view! { <li class="chip mono">{d}</li> }).collect_view()}
                        </ul>
                        <p class="small">
                            {move || if quit.get() { "Students can quit SEB themselves." } else { "Students cannot quit SEB." }}
                        </p>
                        <pre class="json" tabindex="0" aria-label="Generated configuration"><code>{json}</code></pre>
                    }.into_any(),
                }}
            </section>
        </div>

        <section class="exam-list" aria-labelledby="saved-exams">
            <h2 id="saved-exams">"Saved exams"</h2>
            {move || delete_err.get().map(|e| view! { <ErrorNote text=e/> })}
            <Show when=move || !exams.with(|e| e.is_empty())
                fallback=|| view! { <p class="muted">"No exams yet. Create one above."</p> }>
                <ul class="exam-cards" role="list">
                    <For each=move || exams.get() key=|e| e.id.clone() let:e>
                        <li class="panel exam-card">
                            <div class="exam-card-head">
                                <h3>{e.name.clone()}</h3>
                                <span class="muted small">"Created "<LocalTime at_ms=e.created_ms date=true/></span>
                            </div>
                            <p class="mono small break">{e.start_url.clone()}</p>
                            <ul class="chips static">
                                {e.allowed_domains.iter().map(|d| view! { <li class="chip mono">{d.clone()}</li> }).collect_view()}
                            </ul>
                            <p class="small">
                                {if e.allow_quit { "Students may quit SEB" } else { "Students cannot quit SEB" }}
                            </p>
                            <details>
                                <summary>"Generated SEB config"</summary>
                                <pre class="json" tabindex="0"><code>{e.config_json.clone()}</code></pre>
                            </details>
                            <div class="exam-card-actions">
                                <a class="btn ghost small" download=file_name(&e.name) href=data_url(&e.config_json)>"Download JSON"</a>
                                <ConfirmButton label="Delete" confirm="Click again to delete" icon=Icon::Trash class="btn danger small"
                                    on_confirm={
                                        let id = e.id.clone();
                                        Callback::new(move |_| { delete.dispatch(id.clone()); })
                                    }/>
                            </div>
                        </li>
                    </For>
                </ul>
            </Show>
        </section>
    }
}
