//! The Weekly Schedule tab: wake and bedtime, weekly blocks (classes, shifts), split
//! times, and one-time blocks for a single date. Every change saves right away.

use std::future::Future;

use leptos::prelude::*;

use crate::api::{
    add_block, add_split, delete_block, delete_split, get_schedule, save_day_limits, update_block,
};
use crate::model::{NewBlock, Schedule, ScheduleBlock, Split, WEEKDAY_NAMES};
use crate::slices::{fmt_12h, parse_hhmm};

/// Minutes since midnight as `HH:MM`, the format a time input expects.
fn hhmm(minutes: u32) -> String {
    format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60)
}

/// Runs a server call, then reloads the schedule. Errors are shown, not swallowed.
fn run<T, F>(refresh: RwSignal<u32>, error: RwSignal<Option<String>>, call: F)
where
    T: 'static,
    F: Future<Output = Result<T, ServerFnError>> + 'static,
{
    leptos::task::spawn_local(async move {
        match call.await {
            Ok(_) => {
                error.set(None);
                refresh.update(|n| *n += 1);
            }
            Err(e) => error.set(Some(e.to_string())),
        }
    });
}

/// The Weekly Schedule tab.
#[component]
pub fn ScheduleEditor() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let error = RwSignal::new(None::<String>);
    let schedule = Resource::new(move || refresh.get(), |_| get_schedule());

    view! {
        <main class="tab-content active">
            <div class="header-row">
                <h2>"Weekly Schedule"</h2>
            </div>
            <p class="settings-subtitle">
                "Time you can't use for tasks: classes, shifts, appointments. Changes save as you go, and the Daily tab updates on its next load."
            </p>
            {move || error.get().map(|message| view! { <p class="error">{message}</p> })}
            <Suspense fallback=|| view! { <p>"Loading schedule..."</p> }>
                {move || Suspend::new(async move {
                    match schedule.await {
                        Ok(data) => view! {
                            <ScheduleBoard data=data refresh=refresh error=error/>
                        }
                        .into_any(),
                        Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                    }
                })}
            </Suspense>
        </main>
    }
}

/// Day limits, one-time blocks, then one card per weekday.
#[component]
fn ScheduleBoard(
    data: Schedule,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let Schedule {
        wake,
        sleep,
        blocks,
        splits,
    } = data;
    let weekly: Vec<ScheduleBlock> = blocks
        .iter()
        .filter(|b| b.weekday.is_some())
        .cloned()
        .collect();
    let one_time: Vec<ScheduleBlock> = blocks.into_iter().filter(|b| b.on_date.is_some()).collect();

    let cards = WEEKDAY_NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let weekday = u8::try_from(index).unwrap_or(0);
            let day_blocks: Vec<ScheduleBlock> = weekly
                .iter()
                .filter(|b| b.weekday == Some(weekday))
                .cloned()
                .collect();
            let day_splits: Vec<Split> = splits
                .iter()
                .filter(|s| s.weekday == weekday)
                .cloned()
                .collect();
            view! {
                <DayCard
                    name=*name
                    weekday=weekday
                    blocks=day_blocks
                    splits=day_splits
                    refresh=refresh
                    error=error
                />
            }
        })
        .collect::<Vec<_>>();

    view! {
        <DayLimits wake=wake sleep=sleep refresh=refresh error=error/>
        <OneTimeCard blocks=one_time refresh=refresh error=error/>
        {cards}
    }
}

/// Wake-up time and bedtime.
#[component]
fn DayLimits(
    wake: u32,
    sleep: u32,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let wake_text = RwSignal::new(hhmm(wake));
    let sleep_text = RwSignal::new(hhmm(sleep));

    let save = move |_| {
        let (Some(w), Some(s)) = (
            parse_hhmm(&wake_text.get_untracked()),
            parse_hhmm(&sleep_text.get_untracked()),
        ) else {
            error.set(Some("Enter both a wake-up time and a bedtime".to_string()));
            return;
        };
        run(refresh, error, save_day_limits(w, s));
    };

    view! {
        <div class="settings-card">
            <div class="slice-header">
                <span class="slice-title">"Your day"</span>
            </div>
            <div class="sched-row">
                <label>
                    "Wake up"
                    <input
                        type="time"
                        class="input-field"
                        prop:value=move || wake_text.get()
                        on:input=move |ev| wake_text.set(event_target_value(&ev))
                    />
                </label>
                <label>
                    "Bedtime"
                    <input
                        type="time"
                        class="input-field"
                        prop:value=move || sleep_text.get()
                        on:input=move |ev| sleep_text.set(event_target_value(&ev))
                    />
                </label>
                <button type="button" class="btn-primary" on:click=save>"Save"</button>
            </div>
        </div>
    }
}

/// One-time blocks: shifts and appointments tied to a single date.
#[component]
fn OneTimeCard(
    blocks: Vec<ScheduleBlock>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let rows = blocks
        .into_iter()
        .map(|block| view! { <BlockRow block=block refresh=refresh error=error/> })
        .collect::<Vec<_>>();
    let empty = rows.is_empty();

    view! {
        <div class="settings-card">
            <div class="slice-header">
                <span class="slice-title">"One-time blocks (shifts, appointments)"</span>
            </div>
            <p class="settings-subtitle">
                "Things that happen once, on one date. They block that day only."
            </p>
            {empty.then(|| view! { <p class="settings-subtitle">"No one-time blocks yet."</p> })}
            <ul class="task-list">{rows}</ul>
            <AddBlockForm weekday=None refresh=refresh error=error/>
        </div>
    }
}

/// One weekday: its blocks, an add form, and its split times.
#[component]
fn DayCard(
    name: &'static str,
    weekday: u8,
    blocks: Vec<ScheduleBlock>,
    splits: Vec<Split>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let rows = blocks
        .into_iter()
        .map(|block| view! { <BlockRow block=block refresh=refresh error=error/> })
        .collect::<Vec<_>>();
    let empty = rows.is_empty();

    view! {
        <div class="settings-card">
            <div class="slice-header">
                <span class="slice-title">{name}</span>
            </div>
            {empty
                .then(|| {
                    view! { <p class="settings-subtitle">"Nothing blocked. The whole day is open."</p> }
                })}
            <ul class="task-list">{rows}</ul>
            <AddBlockForm weekday=Some(weekday) refresh=refresh error=error/>
            <SplitRow weekday=weekday splits=splits refresh=refresh error=error/>
        </div>
    }
}

/// One blocked window with editable name and times. It saves when you leave a field.
#[component]
fn BlockRow(
    block: ScheduleBlock,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let id = block.id;
    let date_badge = block
        .on_date
        .clone()
        .map(|d| view! { <span class="task-time-badge">{format!("📅 {d}")}</span> });
    let label = RwSignal::new(block.label);
    let start = RwSignal::new(hhmm(block.start));
    let end = RwSignal::new(hhmm(block.end));

    let save = move || {
        let (Some(s), Some(e)) = (
            parse_hhmm(&start.get_untracked()),
            parse_hhmm(&end.get_untracked()),
        ) else {
            error.set(Some("Enter a start and an end time".to_string()));
            return;
        };
        run(
            refresh,
            error,
            update_block(id, label.get_untracked(), s, e),
        );
    };

    view! {
        <li class="task-item">
            <div class="sched-row">
                {date_badge}
                <input
                    type="text"
                    class="input-field"
                    prop:value=move || label.get()
                    on:input=move |ev| label.set(event_target_value(&ev))
                    on:change=move |_| save()
                />
                <input
                    type="time"
                    class="input-field"
                    prop:value=move || start.get()
                    on:input=move |ev| start.set(event_target_value(&ev))
                    on:change=move |_| save()
                />
                <input
                    type="time"
                    class="input-field"
                    prop:value=move || end.get()
                    on:input=move |ev| end.set(event_target_value(&ev))
                    on:change=move |_| save()
                />
                <button
                    type="button"
                    class="icon-btn delete-btn"
                    title="Remove"
                    on:click=move |_| run(refresh, error, delete_block(id))
                >
                    "✖"
                </button>
            </div>
        </li>
    }
}

/// Add a weekly block (when `weekday` is set) or a one-time block (when it is `None`).
#[component]
fn AddBlockForm(
    weekday: Option<u8>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let label = RwSignal::new(String::new());
    let date = RwSignal::new(String::new());
    let start = RwSignal::new(String::new());
    let end = RwSignal::new(String::new());

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let (Some(s), Some(e)) = (
            parse_hhmm(&start.get_untracked()),
            parse_hhmm(&end.get_untracked()),
        ) else {
            error.set(Some("Enter a start and an end time".to_string()));
            return;
        };
        let on_date = if weekday.is_some() {
            None
        } else {
            Some(date.get_untracked())
        };
        let new = NewBlock {
            label: label.get_untracked(),
            weekday,
            on_date,
            start: s,
            end: e,
        };
        label.set(String::new());
        run(refresh, error, add_block(new));
    };

    view! {
        <form class="sched-add" on:submit=submit>
            <input
                type="text"
                class="input-field"
                placeholder="Class, shift, appointment..."
                prop:value=move || label.get()
                on:input=move |ev| label.set(event_target_value(&ev))
            />
            {weekday
                .is_none()
                .then(|| {
                    view! {
                        <input
                            type="date"
                            class="input-field"
                            prop:value=move || date.get()
                            on:input=move |ev| date.set(event_target_value(&ev))
                        />
                    }
                })}
            <input
                type="time"
                class="input-field"
                prop:value=move || start.get()
                on:input=move |ev| start.set(event_target_value(&ev))
            />
            <input
                type="time"
                class="input-field"
                prop:value=move || end.get()
                on:input=move |ev| end.set(event_target_value(&ev))
            />
            <button type="submit" class="btn-primary">"+ Add"</button>
        </form>
    }
}

/// Split times: they cut a day's open time into separate slices.
#[component]
fn SplitRow(
    weekday: u8,
    splits: Vec<Split>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let time = RwSignal::new(String::new());

    let add = move |_| {
        let Some(at) = parse_hhmm(&time.get_untracked()) else {
            error.set(Some("Pick a split time first".to_string()));
            return;
        };
        time.set(String::new());
        run(refresh, error, add_split(weekday, at));
    };

    let chips = splits
        .into_iter()
        .map(|split| {
            let id = split.id;
            view! {
                <span class="ide-chip on">
                    {fmt_12h(split.at)}
                    <button
                        type="button"
                        title="Remove"
                        on:click=move |_| run(refresh, error, delete_split(id))
                    >
                        "✖"
                    </button>
                </span>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <div class="sched-splits">
            <span class="settings-subtitle">"Split open time at:"</span>
            {chips}
            <input
                type="time"
                class="input-field"
                prop:value=move || time.get()
                on:input=move |ev| time.set(event_target_value(&ev))
            />
            <button type="button" class="btn-secondary" on:click=add>"+ Split"</button>
        </div>
    }
}
