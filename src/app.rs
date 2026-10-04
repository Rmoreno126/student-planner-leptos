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
        self.days
            .get(name)
            .cloned()
            .unwrap_or_default()
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

/// Returns the current day label used by the planner.
pub fn current_day_name() -> &'static str {
    "Monday"
}

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

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let schedule = default_weekly_schedule();
    let today = current_day_name();
    let slices = schedule.slices_for(today);
    let (tasks, set_tasks) = signal(vec![
        "Review chemistry notes".to_string(),
        "Finish reading assignment".to_string(),
        "Plan tomorrow's priorities".to_string(),
    ]);
    let (draft, set_draft) = signal(String::new());

    let add_task = move |_| {
        let item = draft.get().trim().to_string();
        if !item.is_empty() {
            set_tasks.update(|list| list.push(item));
            set_draft.set(String::new());
        }
    };

    view! {
        <Stylesheet id="leptos" href="/pkg/student-planner-leptos.css"/>
        <Title text="Student Planner"/>
        <Router>
            <main class="planner-shell">
                <header class="planner-header">
                    <p class="eyebrow">"Daily rhythm"</p>
                    <h1>"Student Planner"</h1>
                    <p class="subtitle">"Automated focus windows for classes, work, and personal study."</p>
                </header>

                <section class="planner-grid">
                    <article class="planner-panel">
                        <div class="panel-header">
                            <h2>{today}</h2>
                            <span class="badge">"Focus Slices"</span>
                        </div>
                        <ul class="slice-list">
                            <For each=move || slices.clone() key=|slice| slice.id.clone() let:slice>
                                <li class:blocked={move || slice.kind == SliceKind::Blocked}>
                                    <span class="label">{slice.label.clone()}</span>
                                </li>
                            </For>
                        </ul>
                    </article>

                    <article class="planner-panel">
                        <div class="panel-header">
                            <h2>"Tasks"</h2>
                            <span class="badge">{move || tasks.get().len()}</span>
                        </div>

                        <div class="task-entry">
                            <input
                                type="text"
                                placeholder="Add a task"
                                prop:value=draft
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_draft.set(value);
                                }
                            />
                            <button on:click=add_task>"Add"</button>
                        </div>

                        <ul class="task-list">
                            <For each=move || tasks.get() key=|task| task.clone() let:task>
                                <li>{task}</li>
                            </For>
                        </ul>
                    </article>
                </section>
            </main>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=HomePage/>
            </Routes>
        </Router>
    }
}

/// Renders the page content for the planner landing page.
#[component]
fn HomePage() -> impl IntoView {
    App()
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
