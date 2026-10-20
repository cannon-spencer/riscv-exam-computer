//! Fleet: live card grid, summary counts, multi-select, start/stop.

use std::collections::BTreeSet;

use leptos::prelude::*;
use leptos_meta::Title;

use super::api::{board_action, error_text, get_fleet};
use super::ui::*;
use super::Shell;
use crate::model::*;

#[component]
pub fn FleetPage() -> impl IntoView {
    let initial = Resource::new(|| (), |_| get_fleet());
    view! {
        <Shell>
            <Title text="Fleet · Exam Console"/>
            <Suspense fallback=|| view! { <p class="loading">"Loading boards…"</p> }>
                {move || Suspend::new(async move {
                    match initial.await {
                        Ok(f) => view! { <FleetBody initial=f/> }.into_any(),
                        Err(e) => view! { <ErrorNote text=error_text(&e)/> }.into_any(),
                    }
                })}
            </Suspense>
        </Shell>
    }
}

#[component]
fn FleetBody(initial: FleetView) -> impl IntoView {
    let fleet = RwSignal::new(Some(initial));
    let lost = RwSignal::new(false);
    poll_every(2000, get_fleet, fleet, lost);

    let selected = RwSignal::new(BTreeSet::<String>::new());
    let filter = RwSignal::new(None::<Status>);
    let report = RwSignal::new(None::<Result<ActionReport, String>>);
    let exam = RwSignal::new(String::new());

    let boards = Memo::new(move |_| fleet.with(|f| f.as_ref().map(|f| f.boards.clone()).unwrap_or_default()));
    let exams = Memo::new(move |_| fleet.with(|f| f.as_ref().map(|f| f.exams.clone()).unwrap_or_default()));
    // Keep the exam picker pointing at an exam that exists.
    Effect::new(move |_| {
        let list = exams.get();
        let cur = exam.get_untracked();
        if !list.iter().any(|e| e.id == cur) {
            exam.set(list.first().map(|e| e.id.clone()).unwrap_or_default());
        }
    });
    // Forget selections of boards that were removed.
    Effect::new(move |_| {
        let hosts: BTreeSet<String> = boards.with(|b| b.iter().map(|b| b.host.clone()).collect());
        if selected.with_untracked(|s| s.iter().any(|h| !hosts.contains(h))) {
            selected.update(|s| s.retain(|h| hosts.contains(h)));
        }
    });

    let count = move |st: Status| boards.with(|b| b.iter().filter(|b| b.status == st).count());
    let visible = move || {
        let f = filter.get();
        let mut v: Vec<BoardView> = boards.get().into_iter().filter(|b| f.is_none_or(|s| b.status == s)).collect();
        v.sort_by_key(|b| b.title().to_lowercase());
        v
    };

    let act = Action::new(move |(verb, exam_id): &(String, String)| {
        let hosts = selected.get_untracked().into_iter().collect::<Vec<_>>().join(",");
        let (verb, exam_id) = (verb.clone(), exam_id.clone());
        async move { board_action(hosts, verb, exam_id).await.map_err(|e| error_text(&e)) }
    });
    Effect::new(move |_| {
        if let Some(r) = act.value().get() {
            report.set(Some(r));
            // Refresh right away instead of waiting for the next poll.
            leptos::task::spawn_local(async move {
                if let Ok(f) = get_fleet().await {
                    fleet.set(Some(f));
                }
            });
        }
    });

    let n_sel = move || selected.with(|s| s.len());
    let select_online = move |_| {
        let hosts: BTreeSet<String> = boards.with(|b| {
            b.iter().filter(|b| b.status != Status::Offline).map(|b| b.host.clone()).collect()
        });
        selected.set(hosts);
    };
    let total = move || boards.with(|b| b.len());
    let plural = |n: usize| if n == 1 { "" } else { "s" };

    view! {
        <header class="page-head">
            <div>
                <h1>"Fleet"</h1>
                <p class="sub">
                    {move || {
                        let online = total() - count(Status::Offline);
                        format!("{online} of {} board{} online", total(), plural(total()))
                    }}
                </p>
            </div>
            <ConnectionPill lost/>
        </header>

        <div class="summary" role="group" aria-label="Filter boards by status">
            <button type="button" class="tile all" aria-pressed=move || filter.get().is_none().to_string()
                on:click=move |_| filter.set(None)>
                <span class="tile-n mono">{total}</span>
                <span class="tile-label">"All boards"</span>
            </button>
            {Status::ALL.into_iter().map(|st| view! {
                <button type="button" class=format!("tile st-{}", st.slug())
                    class:zero=move || count(st) == 0
                    aria-pressed=move || (filter.get() == Some(st)).to_string()
                    on:click=move |_| filter.update(|f| *f = if *f == Some(st) { None } else { Some(st) })>
                    <span class="tile-n mono">{move || count(st)}</span>
                    <span class="tile-label"><StatusIcon status=st/>{st.word()}</span>
                </button>
            }).collect_view()}
        </div>

        <Show when=move || report.get().is_some()>
            <ResultPanel report on_close=Callback::new(move |_| report.set(None))/>
        </Show>

        <Show when=move || { total() > 0 }
            fallback=|| view! {
                <EmptyState title="No boards have checked in yet">
                    <p>"Boards appear here within a few seconds of their agent starting. "
                       "Check that the board is on the network and that its agent points at this server."</p>
                    <p class="muted">"No hardware nearby? Run the simulator: "
                       <code>"cargo run -p sim-agent -- --count 12"</code></p>
                </EmptyState>
            }>
            <div class="toolbar">
                <span class="muted">
                    {move || match n_sel() { 0 => "Select boards to start or stop an exam.".to_string(), n => format!("{n} selected") }}
                </span>
                <span class="toolbar-actions">
                    <button type="button" class="btn ghost small" on:click=select_online>"Select all online"</button>
                    <Show when=move || { n_sel() > 0 }>
                        <button type="button" class="btn ghost small" on:click=move |_| selected.set(BTreeSet::new())>"Clear selection"</button>
                    </Show>
                </span>
            </div>
            <ul class="grid" role="list">
                <For each=visible key=|b| b.host.clone() let:b>
                    <BoardCard host=b.host boards selected/>
                </For>
            </ul>
            <Show when=move || { total() > 0 && visible().is_empty() }>
                <p class="muted center">
                    {move || filter.get().map(|s| format!("No boards are {}.", s.word().trim_end_matches('…').to_lowercase()))}
                    " " <button type="button" class="link" on:click=move |_| filter.set(None)>"Show all boards"</button>
                </p>
            </Show>
        </Show>

        <Show when=move || { n_sel() > 0 }>
            <div class="selbar" role="region" aria-label="Actions for selected boards">
                <span class="selbar-count"><strong class="mono">{n_sel}</strong>" selected"</span>
                <label class="selbar-exam">
                    <span class="sr-only">"Exam to start"</span>
                    <Show when=move || !exams.with(|e| e.is_empty())
                        fallback=|| view! { <a class="link" href="/exams">"Create an exam to start"</a> }>
                        <select prop:value=move || exam.get() on:change=move |ev| exam.set(event_target_value(&ev))>
                            <For each=move || exams.get() key=|e| e.id.clone() let:e>
                                <option value=e.id.clone()>{e.name.clone()}</option>
                            </For>
                        </select>
                    </Show>
                </label>
                <button type="button" class="btn primary"
                    disabled=move || act.pending().get() || exam.with(|e| e.is_empty())
                    on:click=move |_| { act.dispatch(("start".into(), exam.get_untracked())); }>
                    <IconSvg icon=Icon::Play/>
                    {move || format!("Start exam on {} board{}", n_sel(), plural(n_sel()))}
                </button>
                <ConfirmButton
                    label=Signal::derive(move || format!("Stop {}", n_sel()))
                    confirm=Signal::derive(move || format!("Click again to end the exam on {}", n_sel()))
                    disabled=Signal::derive(move || act.pending().get())
                    on_confirm=Callback::new(move |_| { act.dispatch(("stop".into(), String::new())); })/>
            </div>
        </Show>
    }
}

#[component]
fn BoardCard(host: String, boards: Memo<Vec<BoardView>>, selected: RwSignal<BTreeSet<String>>) -> impl IntoView {
    let h = host.clone();
    let board = Memo::new(move |_| boards.with(|bs| bs.iter().find(|b| b.host == h).cloned()));
    let status = Signal::derive(move || board.with(|b| b.as_ref().map_or(Status::Offline, |b| b.status)));
    let h = host.clone();
    let is_sel = Memo::new(move |_| selected.with(|s| s.contains(&h)));
    let h = host.clone();
    let toggle = move |_| {
        let h = h.clone();
        selected.update(|s| {
            if !s.remove(&h) {
                s.insert(h);
            }
        });
    };
    let field = move |f: fn(&BoardView) -> Option<String>| move || board.with(|b| b.as_ref().and_then(f));
    let title = move || board.with(|b| b.as_ref().map(|b| b.title().to_string()).unwrap_or_default());
    let has_label = move || board.with(|b| b.as_ref().is_some_and(|b| b.label.is_some()));
    let check_id = format!("sel-{host}");
    let href = format!("/boards/{host}");

    view! {
        <li class=move || format!("card st-{}", status.get().slug()) class:selected=move || is_sel.get()>
            <div class="card-head">
                <input type="checkbox" class="check" id=check_id.clone() prop:checked=move || is_sel.get() on:change=toggle/>
                <label for=check_id class="card-title">
                    <span class="card-name">{title}</span>
                    <Show when=has_label>
                        <span class="card-host mono">{host.clone()}</span>
                    </Show>
                </label>
                <StatusBadge status/>
            </div>
            <p class="card-detail">
                {move || board.with(|b| {
                    let b = b.as_ref()?;
                    match (b.status, &b.detail, &b.exam_name) {
                        (_, Some(d), _) => Some(d.clone()),
                        (Status::Running, None, Some(e)) => Some(e.clone()),
                        (Status::Running, None, None) => Some("Exam in progress".into()),
                        (Status::Idle, None, _) => Some("Ready for an exam".into()),
                        _ => None,
                    }
                })}
            </p>
            <dl class="card-stats">
                <Show when=move || board.with(|b| b.as_ref().is_some_and(|b| !b.legacy_agent))
                    fallback=|| view! { <p class="muted small">"Old agent: update it to see board stats."</p> }>
                    <div><dt>"Temp"</dt><dd class="mono">{field(|b| b.cpu_temp_c.map(fmt_temp))}</dd></div>
                    <div><dt>"Load"</dt><dd class="mono">{field(|b| b.load1.map(|l| format!("{l:.2}")))}</dd></div>
                    <div><dt>"Memory"</dt><dd class="mono">{field(|b| b.mem_used_pct.map(|m| format!("{m}%")))}</dd></div>
                </Show>
            </dl>
            <div class="card-foot">
                <span class="muted small">
                    {move || board.with(|b| match b.as_ref().and_then(|b| b.seen_secs) {
                        Some(s) => format!("Seen {}", fmt_ago(s)),
                        None => "Not seen since server start".into(),
                    })}
                </span>
                <a class="link small" href=href>"Details"<span class="sr-only">" for "{title}</span></a>
            </div>
        </li>
    }
}

#[component]
pub fn ResultPanel(report: RwSignal<Option<Result<ActionReport, String>>>, on_close: Callback<()>) -> impl IntoView {
    let body = move || match report.get() {
        None => ().into_any(),
        Some(Err(e)) => view! { <ErrorNote text=e/> }.into_any(),
        Some(Ok(r)) => {
            let n = r.queued.len();
            let verb = if r.verb == "Start" { "Start" } else { "Stop" };
            let head = match (n, r.skipped.len()) {
                (0, s) => format!("{verb} was not sent to any board ({s} skipped)"),
                (n, 0) => format!("{verb} sent to {n} board{}", if n == 1 { "" } else { "s" }),
                (n, s) => format!("{verb} sent to {n} board{}, {s} skipped", if n == 1 { "" } else { "s" }),
            };
            view! {
                <h2 class="result-head">{head}</h2>
                {(!r.queued.is_empty()).then(|| view! {
                    <p class="small"><span class="muted">"Queued: "</span>
                        <span class="mono">{r.queued.join(", ")}</span></p>
                })}
                {(!r.skipped.is_empty()).then(|| view! {
                    <ul class="reasons">
                        {r.skipped.into_iter().map(|(h, why)| view! {
                            <li><span class="mono">{h}</span>" — "{why}</li>
                        }).collect_view()}
                    </ul>
                })}
                {(!r.notes.is_empty()).then(|| view! {
                    <ul class="reasons notes">
                        {r.notes.into_iter().map(|(h, note)| view! {
                            <li><IconSvg icon=Icon::Info/><span class="mono">{h}</span>" — "{note}</li>
                        }).collect_view()}
                    </ul>
                })}
            }
            .into_any()
        }
    };
    view! {
        <section class="result" aria-live="polite">
            <div class="result-body">{body}</div>
            <button type="button" class="icon-btn" aria-label="Dismiss" on:click=move |_| on_close.run(())>
                <IconSvg icon=Icon::Close/>
            </button>
        </section>
    }
}
