//! Board detail: rename, stats, start/stop, forget, history.

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_params_map};

use super::Shell;
use super::api::{board_action, error_text, forget_board, get_board, rename_board};
use super::fleet::ResultPanel;
use super::ui::*;
use crate::model::*;

#[component]
pub fn BoardPage() -> impl IntoView {
    let params = use_params_map();
    let host = move || params.with(|p| p.get("host").unwrap_or_default());
    let initial = Resource::new(host, get_board);
    view! {
        <Shell>
            <a class="back" href="/"><IconSvg icon=Icon::Back/>"Fleet"</a>
            <Suspense fallback=|| view! { <p class="loading">"Loading board…"</p> }>
                {move || Suspend::new(async move {
                    match initial.await {
                        Ok(Some(d)) => view! { <BoardBody initial=d/> }.into_any(),
                        Ok(None) => view! { <Gone/> }.into_any(),
                        Err(e) => view! { <ErrorNote text=error_text(&e)/> }.into_any(),
                    }
                })}
            </Suspense>
        </Shell>
    }
}

#[component]
fn Gone() -> impl IntoView {
    view! {
        <Title text="Unknown board · Exam Console"/>
        <EmptyState title="This board is not in the fleet">
            <p>"It may have been forgotten. It will reappear when its agent next checks in."</p>
            <a class="btn secondary" href="/">"Back to the fleet"</a>
        </EmptyState>
    }
}

#[component]
fn BoardBody(initial: BoardDetail) -> impl IntoView {
    let host = initial.board.host.clone();
    let data = RwSignal::new(Some(Some(initial.clone())));
    let lost = RwSignal::new(false);
    {
        let host = host.clone();
        poll_every(2000, move || get_board(host.clone()), data, lost);
    }
    let detail = Memo::new(move |_| data.get().flatten());
    let b = move || detail.with(|d| d.as_ref().map(|d| d.board.clone()));
    let status = Signal::derive(move || b().map_or(Status::Offline, |b| b.status));
    let report = RwSignal::new(None::<Result<ActionReport, String>>);
    let exam = RwSignal::new(initial.exams.first().map(|e| e.id.clone()).unwrap_or_default());

    let h = host.clone();
    let act = Action::new(move |(verb, exam_id): &(String, String)| {
        let (h, verb, exam_id) = (h.clone(), verb.clone(), exam_id.clone());
        async move { board_action(h, verb, exam_id).await.map_err(|e| error_text(&e)) }
    });
    let h = host.clone();
    Effect::new(move |_| {
        if let Some(r) = act.value().get() {
            report.set(Some(r));
            let h = h.clone();
            leptos::task::spawn_local(async move {
                if let Ok(d) = get_board(h).await {
                    data.set(Some(d));
                }
            });
        }
    });

    // Rename.
    let label = RwSignal::new(initial.board.label.clone().unwrap_or_default());
    let rename_msg = RwSignal::new(None::<Result<(), String>>);
    let h = host.clone();
    let rename = Action::new(move |l: &String| {
        let (h, l) = (h.clone(), l.clone());
        async move { rename_board(h, l).await.map_err(|e| error_text(&e)) }
    });
    Effect::new(move |_| {
        if let Some(r) = rename.value().get() {
            rename_msg.set(Some(r));
        }
    });

    // Forget.
    let h = host.clone();
    let forget = Action::new(move |_: &()| {
        let h = h.clone();
        async move { forget_board(h).await.map_err(|e| error_text(&e)) }
    });
    let forget_err = RwSignal::new(None::<String>);
    let navigate = use_navigate();
    Effect::new(move |_| match forget.value().get() {
        Some(Ok(())) => navigate("/", Default::default()),
        Some(Err(e)) => forget_err.set(Some(e)),
        None => {}
    });

    let title = move || b().map(|b| b.title().to_string()).unwrap_or_default();
    let page_title = move || format!("{} · Exam Console", title());
    let stat = move |f: fn(&BoardView) -> Option<String>| move || b().as_ref().and_then(f).unwrap_or_else(|| "—".into());
    let exams = move || detail.with(|d| d.as_ref().map(|d| d.exams.clone()).unwrap_or_default());

    view! {
        <Title text=page_title/>
        <header class="page-head board-head">
            <div>
                <h1>{title}</h1>
                <p class="sub mono">{host.clone()}</p>
            </div>
            <div class="board-head-status">
                <StatusBadge status/>
                <ConnectionPill lost/>
            </div>
        </header>

        {move || b().and_then(|b| b.detail.clone().or(b.exam_name.clone().map(|e| format!("Running {e}")))).map(|d| view! {
            <p class=move || format!("status-line st-{}", status.get().slug())>{d}</p>
        })}

        <Show when=move || report.get().is_some()>
            <ResultPanel report on_close=Callback::new(move |_| report.set(None))/>
        </Show>

        <div class="board-layout">
            <section class="panel">
                <h2>"Exam"</h2>
                <div class="action-row">
                    <Show when=move || !exams().is_empty()
                        fallback=|| view! { <p class="muted">"No exams yet. " <a class="link" href="/exams">"Create one"</a></p> }>
                        <label class="field inline">
                            <span class="field-label">"Exam"</span>
                            <select prop:value=move || exam.get() on:change=move |ev| exam.set(event_target_value(&ev))>
                                <For each=exams key=|e| e.id.clone() let:e>
                                    <option value=e.id.clone()>{e.name.clone()}</option>
                                </For>
                            </select>
                        </label>
                        <button type="button" class="btn primary" disabled=move || act.pending().get()
                            on:click=move |_| { act.dispatch(("start".into(), exam.get_untracked())); }>
                            <IconSvg icon=Icon::Play/>"Start exam"
                        </button>
                    </Show>
                    <ConfirmButton label="Stop exam" confirm="Click again to end this exam"
                        disabled=Signal::derive(move || act.pending().get())
                        on_confirm=Callback::new(move |_| { act.dispatch(("stop".into(), String::new())); })/>
                </div>
                <Show when=move || b().is_some_and(|b| b.legacy_agent)>
                    <p class="hint"><IconSvg icon=Icon::Info/>
                        "This board runs an old agent. It reports no stats and starts SEB with its local config instead of the chosen exam."</p>
                </Show>
            </section>

            <section class="panel">
                <h2>"Name"</h2>
                <form class="action-row" on:submit=move |ev| { ev.prevent_default(); rename.dispatch(label.get_untracked()); }>
                    <label class="field grow">
                        <span class="field-label">"Display name"</span>
                        <input type="text" maxlength="48" placeholder=host.clone() autocomplete="off"
                            prop:value=move || label.get() on:input=move |ev| { label.set(event_target_value(&ev)); rename_msg.set(None); }/>
                        <span class="field-hint">"Shown on the fleet instead of the host name, e.g. “Row 2, seat 4”. Leave empty to use the host name."</span>
                    </label>
                    <button type="submit" class="btn secondary" disabled=move || rename.pending().get()>"Save name"</button>
                </form>
                {move || rename_msg.get().map(|r| match r {
                    Ok(()) => view! { <p class="ok-note" role="status">"Saved."</p> }.into_any(),
                    Err(e) => view! { <ErrorNote text=e/> }.into_any(),
                })}
            </section>
        </div>

        <section class="panel">
            <h2>"Board"</h2>
            <dl class="stats">
                <div class="stat wide">
                    <dt>"CPU temperature"</dt>
                    <dd>
                        <span class="stat-v mono">{stat(|b| b.cpu_temp_c.map(fmt_temp))}</span>
                        {move || detail.with(|d| d.as_ref().map(|d| d.temp_history.clone())).map(|v| view! {
                            <Sparkline values=v label="CPU temperature"/>
                        })}
                    </dd>
                </div>
                <div class="stat wide">
                    <dt>"Memory"</dt>
                    <dd>
                        {move || detail.with(|d| {
                            let d = d.as_ref()?;
                            let (total, avail, pct) = (d.mem_total_kb?, d.mem_available_kb?, d.board.mem_used_pct?);
                            Some(view! {
                                <span class="stat-v mono">{format!("{} of {}", fmt_kb(total.saturating_sub(avail)), fmt_kb(total))}</span>
                                <Meter pct label="Memory"/>
                            }.into_any())
                        }).unwrap_or_else(|| view! { <span class="stat-v mono">"—"</span> }.into_any())}
                    </dd>
                </div>
                <div class="stat"><dt>"Load (1 / 5 / 15 min)"</dt>
                    <dd class="stat-v mono">{move || detail.with(|d| d.as_ref().and_then(|d| d.load)
                        .map(|l| format!("{:.2} / {:.2} / {:.2}", l[0], l[1], l[2])).unwrap_or_else(|| "—".into()))}</dd></div>
                <div class="stat"><dt>"Uptime"</dt><dd class="stat-v mono">{stat(|b| b.uptime_s.map(fmt_uptime))}</dd></div>
                <div class="stat"><dt>"Last seen"</dt><dd class="stat-v mono">{stat(|b| b.seen_secs.map(fmt_ago))}</dd></div>
                <div class="stat"><dt>"IP address"</dt><dd class="stat-v mono">{stat(|b| b.ip.clone())}</dd></div>
                <div class="stat"><dt>"Agent version"</dt><dd class="stat-v mono">{stat(|b| b.agent_version.clone())}</dd></div>
                <div class="stat"><dt>"SEB version"</dt><dd class="stat-v mono">{stat(|b| b.seb_version.clone())}</dd></div>
                <div class="stat"><dt>"OS image"</dt><dd class="stat-v mono">{stat(|b| b.image_hash.clone())}</dd></div>
                <div class="stat"><dt>"Last SEB exit code"</dt><dd class="stat-v mono">{stat(|b| b.last_exit_code.map(|c| c.to_string()))}</dd></div>
                <div class="stat"><dt>"First seen"</dt><dd class="stat-v"><LocalTime at_ms=initial.first_seen_ms date=true/></dd></div>
            </dl>
        </section>

        <section class="panel">
            <h2>"History"</h2>
            {move || detail.with(|d| d.as_ref().map(|d| d.history.clone())).map(|events| view! { <Timeline events/> })}
        </section>

        <section class="panel danger-zone">
            <h2>"Forget this board"</h2>
            <p class="muted">"Removes the board, its name, and its place in the fleet. If its agent is still running, it will check in again as a new board."</p>
            <ConfirmButton label="Forget board" confirm="Click again to forget" icon=Icon::Trash class="btn danger"
                on_confirm=Callback::new(move |_| { forget.dispatch(()); })/>
            {move || forget_err.get().map(|e| view! { <ErrorNote text=e/> })}
        </section>
    }
}
