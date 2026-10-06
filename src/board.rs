//! The Daily board as one Google Calendar-style day: a single timeline from wake to bed,
//! classes as gray blocks, tasks as blue blocks, the past dimmed, and a red
//! "You are here" line. `plan.rs` decides where things go; this file only draws them.

use leptos::prelude::*;

use crate::api::{delete_task, move_task, toggle_complete, toggle_rollover};
use crate::app::run;
use crate::checklist::checklist_progress;
use crate::model::Task;
use crate::plan::{
    capacity_fits, capacity_label, fmt_duration, overflow_label, slice_when, Placed, SlicePlan,
    When,
};
use crate::slices::{fmt_12h, Slice, SliceKind};
use crate::timeline::{mark_label, marks, PX_PER_MIN};

/// Shortest a task block is drawn, so its title stays readable.
const MIN_TASK_PX: f32 = 22.0;
/// Shortest a class block is drawn.
const MIN_BLOCK_PX: f32 = 22.0;
/// How far above the now line the page scrolls to, so the line sits partway down the screen.
const SCROLL_LEAD_PX: f32 = 160.0;

/// Scrolls `target` into view the first time the page shows it, not after every change.
fn scroll_once(target: NodeRef<leptos::html::Div>, scrolled: RwSignal<bool>) {
    Effect::new(move |_| {
        if scrolled.get_untracked() {
            return;
        }
        if let Some(el) = target.get() {
            scrolled.set(true);
            el.scroll_into_view();
        }
    });
}

/// CSS class suffix for something that is over, happening now, or still ahead.
fn when_class(when: When) -> &'static str {
    match when {
        When::Past => " past",
        When::Now => " current",
        When::Later => "",
    }
}

/// The whole day on one set of hour lines.
#[allow(clippy::too_many_arguments)]
#[component]
pub fn DayTimeline(
    slices: Vec<Slice>,
    plans: Vec<SlicePlan>,
    clock: Option<u32>,
    target: Option<(String, u32)>,
    scrolled: RwSignal<bool>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
    editing: RwSignal<Option<Task>>,
) -> impl IntoView {
    let day_start = slices.first().map_or(0, |s| s.start);
    let day_end = slices.last().map_or(day_start, |s| s.end);
    let task_end = plans
        .iter()
        .flat_map(|p| p.tasks.iter().map(|t| t.end))
        .fold(day_end, u32::max);
    let height = (task_end - day_start) as f32 * PX_PER_MIN;
    let px = move |minute: u32| minute.saturating_sub(day_start) as f32 * PX_PER_MIN;

    let lines = marks(day_start, day_end)
        .into_iter()
        .map(|minute| {
            let style = format!("top:{}px", px(minute));
            view! { <div class="cal-hour" style=style><span>{mark_label(minute)}</span></div> }
        })
        .collect::<Vec<_>>();

    // A darker tint over the part of the day that has already gone by.
    let shade = clock.filter(|&c| c > day_start).map(|c| {
        let style = format!("height:{}px", px(c.min(task_end)));
        view! { <div class="past-shade" style=style></div> }
    });

    let blocks = slices
        .iter()
        .filter(|s| s.kind == SliceKind::Blocked)
        .map(|s| {
            let class = format!("day-block{}", when_class(slice_when(s, clock)));
            let tall = ((s.end - s.start) as f32 * PX_PER_MIN).max(MIN_BLOCK_PX);
            let style = format!("top:{}px;height:{tall}px", px(s.start));
            view! { <div class=class style=style>{s.label.clone()}</div> }
        })
        .collect::<Vec<_>>();

    // Free gaps: an invisible band (blue border when it's the current one) with the
    // capacity line at its top when there's room, or as a hover tooltip when there isn't.
    let gaps = plans
        .iter()
        .map(|plan| {
            let s = &plan.slice;
            let class = format!("day-gap{}", when_class(slice_when(s, clock)));
            let tall = (s.end - s.start) as f32 * PX_PER_MIN;
            let style = format!("top:{}px;height:{tall}px", px(s.start));
            let text = capacity_label(plan);
            let tip = text.clone();
            let label =
                capacity_fits(plan).then(|| view! { <span class="gap-capacity">{text}</span> });
            view! { <div class=class style=style title=tip>{label}</div> }
        })
        .collect::<Vec<_>>();

    // Tasks in a slice that is over keep their spot, dimmed, with a Move button.
    let tasks = plans
        .into_iter()
        .flat_map(|plan| {
            let past = slice_when(&plan.slice, clock) == When::Past;
            let mover = if past { target.clone() } else { None };
            plan.tasks
                .into_iter()
                .map(move |placed| (placed, past, mover.clone()))
        })
        .map(|(placed, past, mover)| {
            view! {
                <EventBlock
                    placed=placed
                    origin=day_start
                    past=past
                    mover=mover
                    refresh=refresh
                    error=error
                    editing=editing
                />
            }
        })
        .collect::<Vec<_>>();

    // The red line, plus an invisible anchor above it that the page scrolls to on load.
    let anchor = NodeRef::<leptos::html::Div>::new();
    let now_line = clock.filter(|c| (day_start..day_end).contains(c)).map(|c| {
        let top = px(c);
        let line_style = format!("top:{top}px");
        let lead_style = format!("top:{}px", (top - SCROLL_LEAD_PX).max(0.0));
        let label = format!("📍 You are here · {}", mark_label(c));
        scroll_once(anchor, scrolled);
        view! {
            <div class="scroll-anchor" style=lead_style node_ref=anchor></div>
            <div class="now-line" style=line_style>
                <span class="now-label">{label}</span>
            </div>
        }
    });

    let style = format!("height:{height}px");
    view! {
        <div class="slice-card day-card">
            <div class="day-timeline" style=style>
                {shade}
                {lines}
                {gaps}
                {blocks}
                {tasks}
                {now_line}
            </div>
        </div>
    }
}

/// One task as a block: its top comes from the start time, its height from the duration.
/// Click the body to open the editor.
#[component]
fn EventBlock(
    placed: Placed,
    origin: u32,
    past: bool,
    mover: Option<(String, u32)>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
    editing: RwSignal<Option<Task>>,
) -> impl IntoView {
    let top = placed.start.saturating_sub(origin) as f32 * PX_PER_MIN;
    let tall = (placed.end - placed.start) as f32 * PX_PER_MIN;
    let style = format!("top:{top}px;height:{}px", tall.max(MIN_TASK_PX));
    let task = placed.task;
    let id = task.id;
    let class = format!(
        "event-block prio-{}{}",
        task.priority.code(),
        if past { " past" } else { "" }
    );
    let range = format!(
        "{} – {} · {}",
        fmt_12h(placed.start),
        fmt_12h(placed.end),
        fmt_duration(task.duration_minutes)
    );
    let progress = checklist_progress(&task.notes).map(|(done, total)| {
        view! { <span class="subtask-badge">{format!("{done}/{total}")}</span> }
    });
    let pushed = placed
        .pushed
        .then(|| view! { <span class="event-note">"pushed down"</span> });
    let overflow = overflow_label(placed.overflow)
        .map(|text| view! { <span class="event-overflow">{text}</span> });
    let move_btn = mover.map(|(slice_id, start)| {
        view! {
            <button
                type="button"
                class="move-btn"
                title="Carry this task forward"
                on:click=move |_| {
                    run(refresh, error, move_task(id, Some(slice_id.clone()), Some(start)))
                }
            >
                {format!("Move to {}", mark_label(start))}
            </button>
        }
    });
    let rollover_class = if task.rollover {
        "icon-btn rollover-btn active"
    } else {
        "icon-btn rollover-btn"
    };
    let title = task.title.clone();

    view! {
        <div class=class style=style>
            <input
                type="checkbox"
                class="checkbox"
                on:change=move |_| run(refresh, error, toggle_complete(id))
            />
            <div class="event-body" on:click=move |_| editing.set(Some(task.clone()))>
                <span class="event-title">{title}</span>
                <span class="event-time">{range}</span>
                {progress}
                {pushed}
                {overflow}
            </div>
            <div class="task-actions">
                {move_btn}
                <button
                    class=rollover_class
                    title="Rollover to tomorrow"
                    on:click=move |_| run(refresh, error, toggle_rollover(id))
                >
                    "➔"
                </button>
                <button
                    class="icon-btn delete-btn"
                    title="Delete"
                    on:click=move |_| run(refresh, error, delete_task(id))
                >
                    "✖"
                </button>
            </div>
        </div>
    }
}
