//! Activity: every instructor action and board event, newest first.

use leptos::prelude::*;
use leptos_meta::Title;

use super::Shell;
use super::api::{error_text, get_activity};
use super::ui::*;
use crate::model::EventView;

const FILTERS: [(&str, &str); 4] = [("all", "All"), ("instructor", "Instructor"), ("board", "Boards"), ("system", "System")];

#[component]
pub fn ActivityPage() -> impl IntoView {
    let initial = Resource::new(|| (), |_| get_activity("all".into()));
    view! {
        <Shell>
            <Title text="Activity · Exam Console"/>
            <Suspense fallback=|| view! { <p class="loading">"Loading activity…"</p> }>
                {move || Suspend::new(async move {
                    match initial.await {
                        Ok(events) => view! { <ActivityBody initial=events/> }.into_any(),
                        Err(e) => view! { <ErrorNote text=error_text(&e)/> }.into_any(),
                    }
                })}
            </Suspense>
        </Shell>
    }
}

#[component]
fn ActivityBody(initial: Vec<EventView>) -> impl IntoView {
    let filter = RwSignal::new("all");
    let events = RwSignal::new(Some(initial));
    let lost = RwSignal::new(false);
    poll_every(3000, move || get_activity(filter.get_untracked().into()), events, lost);
    let choose = move |f: &'static str| {
        filter.set(f);
        leptos::task::spawn_local(async move {
            if let Ok(list) = get_activity(f.into()).await {
                events.set(Some(list));
            }
        });
    };
    view! {
        <header class="page-head">
            <div>
                <h1>"Activity"</h1>
                <p class="sub">"Instructor actions and board events, newest first. The last 5,000 entries are kept."</p>
            </div>
            <ConnectionPill lost/>
        </header>
        <div class="seg" role="group" aria-label="Show">
            {FILTERS.into_iter().map(|(value, label)| view! {
                <button type="button" aria-pressed=move || (filter.get() == value).to_string() on:click=move |_| choose(value)>
                    {label}
                </button>
            }).collect_view()}
        </div>
        <section class="panel">
            {move || events.get().map(|events| view! { <Timeline events show_host=true/> })}
        </section>
    }
}
