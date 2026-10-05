use std::collections::BTreeMap;

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

use crate::slices::{compute_slices, Block, DaySchedule, Slice, SliceKind};

/// Default weekly schedule used by the student planner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeeklySchedule {
    pub wake_time: u32,
    pub sleep_time: u32,
    pub days: BTreeMap<String, DaySchedule>,
}

impl WeeklySchedule {
    /// Returns the day schedule for a weekday label.
    pub fn day_for(&self, name: &str) -> DaySchedule {
        self.days.get(name).cloned().unwrap_or_default()
    }

    /// Returns the open and blocked slices for a given day.
    pub fn slices_for(&self, name: &str) -> Vec<Slice> {
        let schedule = self.day_for(name);
        compute_slices(self.wake_time, self.sleep_time, &schedule)
    }
}

/// Creates the default planner schedule from the earlier draft.
pub fn default_weekly_schedule() -> WeeklySchedule {
    let mut days = BTreeMap::new();
    days.insert(
        "Monday".to_string(),
        DaySchedule {
            blocks: vec![
                Block {
                    label: "Class".to_string(),
                    start: 10 * 60,
                    end: 13 * 60,
                },
                Block {
                    label: "Class".to_string(),
                    start: 16 * 60,
                    end: 17 * 60,
                },
            ],
            divisions: vec![],
        },
    );
    days.insert(
        "Tuesday".to_string(),
        DaySchedule {
            blocks: vec![
                Block {
                    label: "Class".to_string(),
                    start: 11 * 60,
                    end: 12 * 60 + 30,
                },
                Block {
                    label: "Class".to_string(),
                    start: 13 * 60,
                    end: 14 * 60 + 30,
                },
            ],
            divisions: vec![],
        },
    );
    days.insert(
        "Wednesday".to_string(),
        DaySchedule {
            blocks: vec![Block {
                label: "Class".to_string(),
                start: 10 * 60,
                end: 13 * 60,
            }],
            divisions: vec![],
        },
    );
    days.insert(
        "Thursday".to_string(),
        DaySchedule {
            blocks: vec![
                Block {
                    label: "Class".to_string(),
                    start: 11 * 60,
                    end: 12 * 60 + 30,
                },
                Block {
                    label: "Work".to_string(),
                    start: 14 * 60,
                    end: 18 * 60,
                },
            ],
            divisions: vec![],
        },
    );
    days.insert(
        "Friday".to_string(),
        DaySchedule {
            blocks: vec![Block {
                label: "Work".to_string(),
                start: 9 * 60,
                end: 13 * 60,
            }],
            divisions: vec![],
        },
    );
    days.insert(
        "Saturday".to_string(),
        DaySchedule {
            blocks: vec![],
            divisions: vec![13 * 60],
        },
    );
    days.insert(
        "Sunday".to_string(),
        DaySchedule {
            blocks: vec![],
            divisions: vec![],
        },
    );

    WeeklySchedule {
        wake_time: 8 * 60,
        sleep_time: 22 * 60,
        days,
    }
}

use std::future::Future;

use leptos_router::hooks::use_query_map;

use crate::api::{
    add_task, delete_task, get_day, list_daily, toggle_complete, toggle_rollover, update_task,
};
use crate::checklist::{checklist_progress, toggle_checkbox};
use crate::editor_modal::{blank_task, EditModal};
use crate::history_view::HistoryView;
use crate::model::{DayData, NewTask, Priority, Task, TaskUpdate};
use crate::notes::{render_markdown, split_notes, NoteBlock};
use crate::schedule_view::ScheduleEditor;
use crate::slices::{blocked_at, fmt_12h, open_slice_id_at, parse_hhmm};
use crate::timeline::BlockedTimeline;

/// Which tab is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Daily,
    Weekly,
    History,
}

/// CSS class for a tab button.
fn tab_class(active: bool) -> &'static str {
    if active {
        "tab-btn active"
    } else {
        "tab-btn"
    }
}

/// HTML shell wrapped around the app on the server.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Root component: page chrome and routes only. It must never render itself.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/student-planner-leptos.css"/>
        <Title text="Student Planner"/>
        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=HomePage/>
            </Routes>
        </Router>
    }
}

/// The page frame: tab bar plus whichever tab is active.
#[component]
fn HomePage() -> impl IntoView {
    let tab = RwSignal::new(Tab::Daily);

    view! {
        <div class="app-container">
            <nav class="nav-tabs">
                <button
                    class=move || tab_class(tab.get() == Tab::Daily)
                    on:click=move |_| tab.set(Tab::Daily)
                >
                    <span class="tab-icon">"📋"</span>
                    " Daily Tasks"
                </button>
                <button
                    class=move || tab_class(tab.get() == Tab::Weekly)
                    on:click=move |_| tab.set(Tab::Weekly)
                >
                    <span class="tab-icon blue-icon">"⚙️"</span>
                    " Weekly Schedule"
                </button>
                <button
                    class=move || tab_class(tab.get() == Tab::History)
                    on:click=move |_| tab.set(Tab::History)
                >
                    <span class="tab-icon">"📦"</span>
                    " History"
                </button>
            </nav>
            {move || match tab.get() {
                Tab::Daily => view! { <DailyView/> }.into_any(),
                Tab::Weekly => view! { <ScheduleEditor/> }.into_any(),
                Tab::History => view! { <HistoryView/> }.into_any(),
            }}
        </div>
    }
}

/// Loads the day's blocks and wake/bed times, then shows the plan.
/// Add `?day=Monday` to the address to preview another weekday's layout.
#[component]
fn DailyView() -> impl IntoView {
    let query = use_query_map();
    let day = Resource::new(move || query.get().get("day"), get_day);

    view! {
        <main class="tab-content active">
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || Suspend::new(async move {
                    match day.await {
                        Ok(data) => view! { <DayPlan data=data/> }.into_any(),
                        Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                    }
                })}
            </Suspense>
        </main>
    }
}

/// Header, add-task form, slice board, and edit dialog for one weekday.
#[component]
fn DayPlan(data: DayData) -> impl IntoView {
    let wake = data.wake;
    let day_schedule = DaySchedule {
        blocks: data
            .blocks
            .iter()
            .map(|b| Block {
                label: b.label.clone(),
                start: b.start,
                end: b.end,
            })
            .collect(),
        divisions: data.divisions.clone(),
    };
    let slices = compute_slices(data.wake, data.sleep, &day_schedule);
    let name = data.name;
    let form_slices = slices.clone();
    let modal_slices = slices.clone();

    let refresh = RwSignal::new(0u32);
    let error = RwSignal::new(None::<String>);
    let editing = RwSignal::new(None::<Task>);
    let tasks = Resource::new(move || refresh.get(), |_| list_daily());

    view! {
        <div class="header-row">
            <h2>"Today's Plan"</h2>
            <span class="day-badge">{name}</span>
        </div>
        <AddTaskForm slices=form_slices wake=wake refresh=refresh error=error/>
        <button type="button" class="btn-secondary" on:click=move |_| editing.set(Some(blank_task()))>
            "+ Add with notes (editor)"
        </button>
        {move || error.get().map(|message| view! { <p class="error">{message}</p> })}
        <Suspense fallback=|| view! { <p>"Loading tasks..."</p> }>
            {move || {
                let slices = slices.clone();
                Suspend::new(async move {
                    match tasks.await {
                        Ok(list) => view! {
                            <SliceBoard
                                slices=slices
                                tasks=list
                                refresh=refresh
                                error=error
                                editing=editing
                            />
                        }
                        .into_any(),
                        Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                    }
                })
            }}
        </Suspense>
        {move || {
            editing.get().map(|task| {
                view! {
                    <EditModal
                        task=task
                        slices=modal_slices.clone()
                        wake=wake
                        editing=editing
                        refresh=refresh
                        error=error
                    />
                }
            })
        }}
    }
}

/// Runs a server call in the background, then reloads the task list.
/// Errors are shown above the list instead of being swallowed.
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

/// Title box, start time, priority picker, and the Add button.
#[component]
fn AddTaskForm(
    slices: Vec<Slice>,
    wake: u32,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let start = RwSignal::new(String::new());
    let priority = RwSignal::new(Priority::High);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let text = title.get_untracked();
        if text.trim().is_empty() {
            return;
        }
        let minutes = parse_hhmm(&start.get_untracked());
        if let Some(t) = minutes {
            if let Some(block) = blocked_at(&slices, t, wake) {
                error.set(Some(format!(
                    "That time falls in a blocked window: {}",
                    block.label
                )));
                return;
            }
        }
        let slice_id = minutes
            .and_then(|t| open_slice_id_at(&slices, t, wake))
            .map(str::to_string);
        let new = NewTask {
            title: text,
            notes: String::new(),
            priority: priority.get_untracked(),
            due_date: None,
            slice_id,
            start_time: minutes,
        };
        title.set(String::new());
        run(refresh, error, add_task(new));
    };

    view! {
        <form class="task-form" on:submit=submit>
            <input
                type="text"
                class="task-input"
                placeholder="What needs to be done?"
                prop:value=move || title.get()
                on:input=move |ev| title.set(event_target_value(&ev))
            />
            <div class="form-controls">
                <div class="time-picker-wrapper">
                    <label class="input-inline-label">"Start Time:"</label>
                    <input
                        type="time"
                        class="task-select time-input"
                        prop:value=move || start.get()
                        on:input=move |ev| start.set(event_target_value(&ev))
                    />
                </div>
                <select
                    class="task-select"
                    on:change=move |ev| {
                        let chosen = match event_target_value(&ev).as_str() {
                            "1" => Priority::Low,
                            "2" => Priority::Medium,
                            _ => Priority::High,
                        };
                        priority.set(chosen);
                    }
                >
                    <option value="3">"High"</option>
                    <option value="2">"Med"</option>
                    <option value="1">"Low"</option>
                </select>
                <button type="submit" class="btn-primary add-btn">"Add Task"</button>
            </div>
        </form>
    }
}

/// One card per slice, then unassigned tasks, then the completed card.
#[component]
fn SliceBoard(
    slices: Vec<Slice>,
    tasks: Vec<Task>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
    editing: RwSignal<Option<Task>>,
) -> impl IntoView {
    let (mut done, mut active): (Vec<Task>, Vec<Task>) =
        tasks.into_iter().partition(|t| t.completed);
    active.sort_by_key(|t| (t.start_time.unwrap_or(u32::MAX), t.id));
    done.sort_by_key(|t| (t.start_time.unwrap_or(u32::MAX), t.id));

    let open_ids: Vec<String> = slices
        .iter()
        .filter(|s| s.kind == SliceKind::Open)
        .map(|s| s.id.clone())
        .collect();
    // A task belongs to its slice only if that slice exists today.
    let target = |t: &Task| -> Option<String> {
        match &t.slice_id {
            Some(id) if open_ids.contains(id) => Some(id.clone()),
            _ => None,
        }
    };

    let cards = slices
        .iter()
        .map(|slice| {
            if slice.kind == SliceKind::Blocked {
                view! {
                    <div class="slice-card blocked">
                        <div class="slice-header">
                            <span class="slice-title">{slice.label.clone()}</span>
                        </div>
                        <BlockedTimeline start=slice.start end=slice.end/>
                    </div>
                }
                .into_any()
            } else {
                let mine: Vec<Task> = active
                    .iter()
                    .filter(|t| target(t).as_deref() == Some(slice.id.as_str()))
                    .cloned()
                    .collect();
                view! {
                    <div class="slice-card">
                        <div class="slice-header">
                            <span class="slice-title">{slice.label.clone()}</span>
                        </div>
                        <TaskList tasks=mine refresh=refresh error=error editing=editing/>
                    </div>
                }
                .into_any()
            }
        })
        .collect::<Vec<_>>();

    let loose: Vec<Task> = active
        .iter()
        .filter(|t| target(t).is_none())
        .cloned()
        .collect();
    let loose_card = (!loose.is_empty()).then(|| {
        view! {
            <div class="slice-card">
                <div class="slice-header">
                    <span class="slice-title">"📌 Unassigned Tasks"</span>
                </div>
                <TaskList tasks=loose refresh=refresh error=error editing=editing/>
            </div>
        }
    });

    let done_card = (!done.is_empty()).then(|| {
        view! {
            <div class="completed-card">
                <h3 class="completed-header">"✓ Completed Tasks"</h3>
                <TaskList tasks=done refresh=refresh error=error editing=editing/>
            </div>
        }
    });

    view! {
        <div class="slices-wrapper">{cards}{loose_card}</div>
        {done_card}
    }
}

/// A list of task rows.
#[component]
fn TaskList(
    tasks: Vec<Task>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
    editing: RwSignal<Option<Task>>,
) -> impl IntoView {
    view! {
        <ul class="task-list">
            {tasks
                .into_iter()
                .map(|task| {
                    view! { <TaskRow task=task refresh=refresh error=error editing=editing/> }
                })
                .collect::<Vec<_>>()}
        </ul>
    }
}

/// One task: checkbox, title, badges, action buttons, and its notes.
#[component]
fn TaskRow(
    task: Task,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
    editing: RwSignal<Option<Task>>,
) -> impl IntoView {
    let for_edit = task.clone();
    let id = task.id;
    let completed = task.completed;
    let item_class = format!(
        "task-item{}{}",
        if task.completed { " completed" } else { "" },
        if task.rollover { " is-rollover" } else { "" }
    );
    let rollover_class = if task.rollover {
        "icon-btn rollover-btn active"
    } else {
        "icon-btn rollover-btn"
    };
    let badge_class = format!("priority-badge prio-{}", task.priority.code());
    let label = task.priority.label();
    let time_badge = task.start_time.map(|t| format!("🕒 {}", fmt_12h(t)));
    let progress_badge = checklist_progress(&task.notes).map(|(done, total)| {
        let class = if done == total {
            "subtask-badge all-done"
        } else {
            "subtask-badge"
        };
        view! { <span class=class>{format!("{done}/{total}")}</span> }
    });
    let notes_view = (!task.notes.trim().is_empty()).then(|| {
        view! { <NotesView id=id notes=task.notes.clone() refresh=refresh error=error/> }
    });
    let title = task.title;

    view! {
        <li class=item_class>
            <div class="task-item-main">
                <input
                    type="checkbox"
                    class="checkbox"
                    checked=completed
                    on:change=move |_| run(refresh, error, toggle_complete(id))
                />
                <span class="task-title">{title}</span>
                {time_badge.map(|text| view! { <span class="task-time-badge">{text}</span> })}
                {progress_badge}
                <span class=badge_class>{label}</span>
                <div class="task-actions">
                    <button
                        class=rollover_class
                        title="Rollover to tomorrow"
                        on:click=move |_| run(refresh, error, toggle_rollover(id))
                    >
                        "➔"
                    </button>
                    <button
                        class="icon-btn edit-btn"
                        title="Edit"
                        on:click=move |_| editing.set(Some(for_edit.clone()))
                    >
                        "✎"
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
            {notes_view}
        </li>
    }
}

/// A task's notes: markdown text, with checklist lines as tickable boxes.
#[component]
fn NotesView(
    id: i64,
    notes: String,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let rows = split_notes(&notes)
        .into_iter()
        .map(|block| match block {
            NoteBlock::Text(text) => {
                view! { <div inner_html=render_markdown(&text)></div> }.into_any()
            }
            NoteBlock::Check {
                index,
                checked,
                label,
            } => {
                let source = notes.clone();
                view! {
                    <label class="note-check">
                        <input
                            type="checkbox"
                            checked=checked
                            on:change=move |_| {
                                if let Some(updated) = toggle_checkbox(&source, index) {
                                    let changes = TaskUpdate {
                                        notes: Some(updated),
                                        ..TaskUpdate::default()
                                    };
                                    run(refresh, error, update_task(id, changes));
                                }
                            }
                        />
                        <span>{label}</span>
                    </label>
                }
                .into_any()
            }
        })
        .collect::<Vec<_>>();

    view! { <div class="task-notes-rendered">{rows}</div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_schedule_matches_weekday_blocks() {
        let schedule = default_weekly_schedule();
        assert_eq!(schedule.wake_time, 8 * 60);
        assert_eq!(schedule.sleep_time, 22 * 60);
        assert_eq!(schedule.day_for("Monday").blocks.len(), 2);
        assert_eq!(schedule.day_for("Saturday").divisions, vec![13 * 60]);
    }

    #[test]
    fn monday_has_expected_slice_sequence() {
        let schedule = default_weekly_schedule();
        let slices = schedule.slices_for("Monday");
        let kinds: Vec<_> = slices.iter().map(|slice| slice.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                SliceKind::Open,
                SliceKind::Blocked,
                SliceKind::Open,
                SliceKind::Blocked,
                SliceKind::Open,
            ]
        );
        assert_eq!(slices[0].label, "Slice 1: 08:00 AM - 10:00 AM");
        assert_eq!(slices[1].label, "🚫 Class (10:00 AM - 01:00 PM)");
    }
}
