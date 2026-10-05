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
    add_task, delete_task, list_daily, move_task, today_name, toggle_complete, toggle_rollover,
    update_task,
};
use crate::checklist::{checklist_progress, toggle_checkbox};
use crate::history_view::HistoryView;
use crate::model::{NewTask, Priority, Task, TaskUpdate};
use crate::notes::{render_markdown, split_notes, NoteBlock};
use crate::slices::{blocked_at, fmt_12h, open_slice_id_at, parse_hhmm};

/// Weekday labels, Monday first.
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

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

/// Minutes since midnight as `HH:MM`, the format a time input expects.
fn hhmm(minutes: u32) -> String {
    format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60)
}

/// Adds a markdown snippet to the end of the notes. Line-style snippets start a new line.
fn append_snippet(notes: RwSignal<String>, snippet: &'static str) {
    let line_style = ["[ ] ", "- ", "1. ", "### "].contains(&snippet);
    notes.update(|text| {
        if line_style && !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(snippet);
    });
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
                    <span class="tab-icon">"🗄"</span>
                    " History"
                </button>
            </nav>
            {move || match tab.get() {
                Tab::Daily => view! { <DailyView/> }.into_any(),
                Tab::Weekly => view! { <WeeklyView/> }.into_any(),
                Tab::History => view! { <HistoryView/> }.into_any(),
            }}
        </div>
    }
}

/// Looks up today's weekday (Pacific time), then shows the plan for it.
/// Add `?day=Monday` to the address to preview another weekday's layout.
#[component]
fn DailyView() -> impl IntoView {
    let query = use_query_map();
    let day = Resource::new(
        move || query.get().get("day"),
        |preview| async move {
            match preview {
                Some(name) if WEEKDAYS.contains(&name.as_str()) => {
                    Ok::<String, ServerFnError>(name)
                }
                _ => today_name().await,
            }
        },
    );

    view! {
        <main class="tab-content active">
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || Suspend::new(async move {
                    let name = day.await.unwrap_or_else(|_| "Monday".to_string());
                    view! { <DayPlan name=name/> }
                })}
            </Suspense>
        </main>
    }
}

/// Header, add-task form, slice board, and edit dialog for one weekday.
#[component]
fn DayPlan(name: String) -> impl IntoView {
    let schedule = default_weekly_schedule();
    let slices = schedule.slices_for(&name);
    let wake = schedule.wake_time;
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
                        <p class="blocked-text">"🔒 Tasks cannot be assigned during this period."</p>
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

/// The edit dialog: title, slice, start time, and markdown notes.
#[component]
fn EditModal(
    task: Task,
    slices: Vec<Slice>,
    wake: u32,
    editing: RwSignal<Option<Task>>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let id = task.id;
    let open_ids: Vec<String> = slices
        .iter()
        .filter(|s| s.kind == SliceKind::Open)
        .map(|s| s.id.clone())
        .collect();
    let title = RwSignal::new(task.title);
    let notes = RwSignal::new(task.notes);
    let time = RwSignal::new(task.start_time.map(hhmm).unwrap_or_default());
    let slice_sel = RwSignal::new(
        task.slice_id
            .filter(|s| open_ids.contains(s))
            .unwrap_or_default(),
    );

    let slices_for_time = slices.clone();
    let slices_for_save = slices.clone();
    let slice_options = slices
        .iter()
        .filter(|s| s.kind == SliceKind::Open)
        .map(|s| {
            let value = s.id.clone();
            let current = s.id.clone();
            let label = s.label.clone();
            view! {
                <option value=value prop:selected=move || slice_sel.get() == current>
                    {label}
                </option>
            }
        })
        .collect::<Vec<_>>();

    let close = move |_| editing.set(None);

    let save = move |_| {
        let new_title = title.get_untracked();
        if new_title.trim().is_empty() {
            error.set(Some("Task title cannot be empty".to_string()));
            return;
        }
        let minutes = parse_hhmm(&time.get_untracked());
        if let Some(t) = minutes {
            if let Some(block) = blocked_at(&slices_for_save, t, wake) {
                error.set(Some(format!(
                    "That time falls in a blocked window: {}",
                    block.label
                )));
                return;
            }
        }
        let chosen = slice_sel.get_untracked();
        let slice_id = if chosen.is_empty() {
            None
        } else {
            Some(chosen)
        };
        let changes = TaskUpdate {
            title: Some(new_title),
            notes: Some(notes.get_untracked()),
            priority: None,
        };
        leptos::task::spawn_local(async move {
            let saved: Result<Task, ServerFnError> = async {
                update_task(id, changes).await?;
                move_task(id, slice_id, minutes).await
            }
            .await;
            match saved {
                Ok(_) => {
                    error.set(None);
                    refresh.update(|n| *n += 1);
                    editing.set(None);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <div class="modal-overlay">
            <div class="modal-content edit-modal-large">
                <div class="modal-header">
                    <h3>"Edit Task"</h3>
                    <button type="button" class="icon-btn" on:click=close>"✖"</button>
                </div>
                <div class="modal-body">
                    {move || error.get().map(|message| view! { <p class="error">{message}</p> })}
                    <div class="edit-field-group">
                        <label>"Task Title"</label>
                        <input
                            type="text"
                            class="input-field"
                            prop:value=move || title.get()
                            on:input=move |ev| title.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="edit-row">
                        <div class="edit-field-group flex-1">
                            <label>"Assigned Time Slice"</label>
                            <select
                                class="input-field"
                                on:change=move |ev| slice_sel.set(event_target_value(&ev))
                            >
                                <option value="" prop:selected=move || slice_sel.get().is_empty()>
                                    "Unassigned"
                                </option>
                                {slice_options}
                            </select>
                        </div>
                        <div class="edit-field-group flex-1">
                            <label>"Start Time"</label>
                            <input
                                type="time"
                                class="input-field"
                                prop:value=move || time.get()
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    if let Some(found) = parse_hhmm(&value)
                                        .and_then(|t| open_slice_id_at(&slices_for_time, t, wake))
                                    {
                                        slice_sel.set(found.to_string());
                                    }
                                    time.set(value);
                                }
                            />
                        </div>
                    </div>
                    <div class="edit-field-group">
                        <label>"Additional Notes (Markdown Supported)"</label>
                        <div class="md-toolbar">
                            <button type="button" on:click=move |_| append_snippet(notes, "[ ] ")>
                                "☑ Task"
                            </button>
                            <button type="button" on:click=move |_| append_snippet(notes, "- ")>
                                "• List"
                            </button>
                            <button type="button" on:click=move |_| append_snippet(notes, "1. ")>
                                "1. Number"
                            </button>
                            <button type="button" on:click=move |_| append_snippet(notes, "### ")>
                                "H3"
                            </button>
                            <button
                                type="button"
                                on:click=move |_| append_snippet(notes, "**bold**")
                            >
                                "Bold"
                            </button>
                        </div>
                        <textarea
                            placeholder="Add subtasks or notes here using Markdown..."
                            prop:value=move || notes.get()
                            on:input=move |ev| notes.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                </div>
                <div class="modal-footer">
                    <button type="button" class="btn-secondary" on:click=close>"Cancel"</button>
                    <button type="button" class="btn-primary" on:click=save>"Save Changes"</button>
                </div>
            </div>
        </div>
    }
}

/// Read-only view of the weekly schedule (the editor comes later).
#[component]
fn WeeklyView() -> impl IntoView {
    let schedule = default_weekly_schedule();
    let summary = format!(
        "Wake {} · Bedtime {}.",
        fmt_12h(schedule.wake_time),
        fmt_12h(schedule.sleep_time)
    );
    let cards = WEEKDAYS
        .iter()
        .map(|name| {
            let day = schedule.day_for(name);
            let mut lines: Vec<String> = day
                .blocks
                .iter()
                .map(|b| format!("{}: {} - {}", b.label, fmt_12h(b.start), fmt_12h(b.end)))
                .collect();
            lines.extend(
                day.divisions
                    .iter()
                    .map(|d| format!("Split at {}", fmt_12h(*d))),
            );
            let body = if lines.is_empty() {
                view! { <p class="blocked-text">"Open all day"</p> }.into_any()
            } else {
                view! {
                    <ul class="task-list">
                        {lines
                            .into_iter()
                            .map(|line| {
                                view! {
                                    <li class="task-item">
                                        <div class="task-item-main">
                                            <span class="task-title">{line}</span>
                                        </div>
                                    </li>
                                }
                            })
                            .collect::<Vec<_>>()}
                    </ul>
                }
                .into_any()
            };
            view! {
                <div class="settings-card">
                    <div class="slice-header">
                        <span class="slice-title">{*name}</span>
                    </div>
                    {body}
                </div>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <main class="tab-content active">
            <div class="header-row">
                <h2>"Weekly Recurring Schedule"</h2>
            </div>
            <p class="settings-subtitle">{summary}" The editor is coming next."</p>
            <div class="slices-wrapper">{cards}</div>
        </main>
    }
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
