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

use crate::api::{add_task, delete_task, list_daily, today_name, toggle_complete, toggle_rollover};
use crate::model::{NewTask, Priority, Task};

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

/// The planner page: today's slices on one side, today's tasks on the other.
#[component]
fn HomePage() -> impl IntoView {
    let day = Resource::new(|| (), |_| today_name());

    view! {
        <main class="planner-shell">
            <header class="planner-header">
                <p class="eyebrow">"Daily rhythm"</p>
                <h1>"Student Planner"</h1>
            </header>
            <section class="planner-grid">
                <Suspense fallback=|| view! { <p>"Loading schedule..."</p> }>
                    {move || Suspend::new(async move {
                        let name = day.await.unwrap_or_else(|_| "Monday".to_string());
                        let slices = default_weekly_schedule().slices_for(&name);
                        view! { <SlicePanel day=name slices=slices/> }
                    })}
                </Suspense>
                <TaskPanel/>
            </section>
        </main>
    }
}

/// Lists the day's open and blocked slices.
#[component]
fn SlicePanel(day: String, slices: Vec<Slice>) -> impl IntoView {
    view! {
        <article class="planner-panel">
            <div class="panel-header">
                <h2>{day}</h2>
                <span class="badge">"Focus Slices"</span>
            </div>
            <ul class="slice-list">
                {slices
                    .into_iter()
                    .map(|slice| {
                        let class = if slice.kind == SliceKind::Blocked { "blocked" } else { "" };
                        view! { <li class=class><span class="label">{slice.label}</span></li> }
                    })
                    .collect::<Vec<_>>()}
            </ul>
        </article>
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

/// Today's tasks, loaded from the database.
#[component]
fn TaskPanel() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let error = RwSignal::new(None::<String>);
    let tasks = Resource::new(move || refresh.get(), |_| list_daily());

    view! {
        <article class="planner-panel">
            <div class="panel-header">
                <h2>"Today's Tasks"</h2>
            </div>
            <AddTaskForm refresh=refresh error=error/>
            {move || error.get().map(|message| view! { <p class="error">{message}</p> })}
            <Suspense fallback=|| view! { <p>"Loading tasks..."</p> }>
                {move || Suspend::new(async move {
                    match tasks.await {
                        Ok(list) if list.is_empty() => {
                            view! { <p class="empty">"Nothing planned yet."</p> }.into_any()
                        }
                        Ok(list) => view! {
                            <ul class="task-list">
                                {list
                                    .into_iter()
                                    .map(|task| view! { <TaskRow task=task refresh=refresh error=error/> })
                                    .collect::<Vec<_>>()}
                            </ul>
                        }
                        .into_any(),
                        Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                    }
                })}
            </Suspense>
        </article>
    }
}

/// Title box, priority picker, and the Add button.
#[component]
fn AddTaskForm(refresh: RwSignal<u32>, error: RwSignal<Option<String>>) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let priority = RwSignal::new(Priority::High);

    let submit = move |_| {
        let text = title.get_untracked();
        if text.trim().is_empty() {
            return;
        }
        let new = NewTask {
            title: text,
            notes: String::new(),
            priority: priority.get_untracked(),
            due_date: None,
            slice_id: None,
            start_time: None,
        };
        title.set(String::new());
        run(refresh, error, add_task(new));
    };

    view! {
        <div class="task-entry">
            <input
                type="text"
                placeholder="What needs to be done?"
                prop:value=move || title.get()
                on:input=move |ev| title.set(event_target_value(&ev))
            />
            <select on:change=move |ev| {
                let chosen = match event_target_value(&ev).as_str() {
                    "1" => Priority::Low,
                    "2" => Priority::Medium,
                    _ => Priority::High,
                };
                priority.set(chosen);
            }>
                <option value="3">"High"</option>
                <option value="2">"Medium"</option>
                <option value="1">"Low"</option>
            </select>
            <button on:click=submit>"Add Task"</button>
        </div>
    }
}

/// One task: checkbox, title, priority badge, rollover and delete buttons.
#[component]
fn TaskRow(task: Task, refresh: RwSignal<u32>, error: RwSignal<Option<String>>) -> impl IntoView {
    let id = task.id;
    let completed = task.completed;
    let item_class = format!(
        "task-item {} {}",
        if task.completed { "completed" } else { "" },
        if task.rollover { "is-rollover" } else { "" }
    );
    let rollover_class = if task.rollover {
        "icon-btn rollover-btn active"
    } else {
        "icon-btn rollover-btn"
    };
    let badge_class = format!("priority-badge prio-{}", task.priority.code());
    let label = task.priority.label();
    let title = task.title;

    view! {
        <li class=item_class>
            <input
                type="checkbox"
                checked=completed
                on:change=move |_| run(refresh, error, toggle_complete(id))
            />
            <span class="task-title">{title}</span>
            <span class=badge_class>{label}</span>
            <button
                class=rollover_class
                title="Roll over to tomorrow"
                on:click=move |_| run(refresh, error, toggle_rollover(id))
            >
                "➔"
            </button>
            <button
                class="icon-btn"
                title="Delete"
                on:click=move |_| run(refresh, error, delete_task(id))
            >
                "✖"
            </button>
        </li>
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
