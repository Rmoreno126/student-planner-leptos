//! The History tab: missed, finished, and archived tasks, searchable, with an
//! "Archive all" button. Nothing here returns to the daily plan on its own.

use std::future::Future;

use leptos::prelude::*;

use crate::api::{archive_all, list_history, reschedule_today};
use crate::model::{History, Task};

/// Runs a server call, then reloads the history lists.
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

/// Does the task match the search text? Looks at title, notes, and date.
fn matches(task: &Task, needle: &str) -> bool {
    let needle = needle.trim().to_lowercase();
    needle.is_empty()
        || task.title.to_lowercase().contains(&needle)
        || task.notes.to_lowercase().contains(&needle)
        || task.due_date.contains(&needle)
}

/// The History tab.
#[component]
pub fn HistoryView() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let error = RwSignal::new(None::<String>);
    let search = RwSignal::new(String::new());
    let history = Resource::new(move || refresh.get(), |_| list_history());

    view! {
        <main class="tab-content active">
            <div class="header-row">
                <h2>"History"</h2>
                <button class="btn-primary" on:click=move |_| run(refresh, error, archive_all())>
                    "🗄 Archive all"
                </button>
            </div>
            <p class="settings-subtitle">
                "Missed, finished, and archived tasks. Nothing here returns to your plan unless you reschedule it."
            </p>
            <input
                type="text"
                class="input-field"
                placeholder="Search history..."
                prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))
            />
            {move || error.get().map(|message| view! { <p class="error">{message}</p> })}
            <Suspense fallback=|| view! { <p>"Loading history..."</p> }>
                {move || Suspend::new(async move {
                    match history.await {
                        Ok(data) => view! {
                            <HistoryBoard data=data search=search refresh=refresh error=error/>
                        }
                        .into_any(),
                        Err(e) => view! { <p class="error">{e.to_string()}</p> }.into_any(),
                    }
                })}
            </Suspense>
        </main>
    }
}

/// The three sections: missed, finished, archive.
#[component]
fn HistoryBoard(
    data: History,
    search: RwSignal<String>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let History {
        missed,
        done,
        archived,
    } = data;

    view! {
        <HistorySection
            title="⚠️ Missed"
            hint="Planned for an earlier day and never finished."
            tasks=missed
            search=search
            refresh=refresh
            error=error
            can_reschedule=true
        />
        <HistorySection
            title="✓ Finished"
            hint="Done on an earlier day. Archive all tidies these away."
            tasks=done
            search=search
            refresh=refresh
            error=error
            can_reschedule=false
        />
        <HistorySection
            title="🗄 Archive"
            hint="Everything you archived. Nothing is ever deleted automatically."
            tasks=archived
            search=search
            refresh=refresh
            error=error
            can_reschedule=true
        />
    }
}

/// One titled card of tasks, filtered by the search box.
#[component]
fn HistorySection(
    title: &'static str,
    hint: &'static str,
    tasks: Vec<Task>,
    search: RwSignal<String>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
    can_reschedule: bool,
) -> impl IntoView {
    let total = tasks.len();

    view! {
        <div class="slice-card">
            <div class="slice-header">
                <span class="slice-title">{title}</span>
                <span class="subtask-badge">{total}</span>
            </div>
            <p class="blocked-text">{hint}</p>
            <ul class="task-list">
                {move || {
                    let needle = search.get();
                    let rows: Vec<Task> = tasks
                        .iter()
                        .filter(|t| matches(t, &needle))
                        .cloned()
                        .collect();
                    let items: Vec<AnyView> = if rows.is_empty() {
                        vec![view! { <li class="blocked-text">"Nothing here."</li> }.into_any()]
                    } else {
                        rows.into_iter()
                            .map(|task| {
                                view! {
                                    <HistoryRow
                                        task=task
                                        can_reschedule=can_reschedule
                                        refresh=refresh
                                        error=error
                                    />
                                }
                                .into_any()
                            })
                            .collect()
                    };
                    items
                }}
            </ul>
        </div>
    }
}

/// One history row: title, date, priority, and a button to bring it back to today.
#[component]
fn HistoryRow(
    task: Task,
    can_reschedule: bool,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let id = task.id;
    let item_class = if task.completed {
        "task-item completed"
    } else {
        "task-item"
    };
    let badge_class = format!("priority-badge prio-{}", task.priority.code());
    let label = task.priority.label();
    let date = format!("📅 {}", task.due_date);
    let title = task.title;

    view! {
        <li class=item_class>
            <div class="task-item-main">
                <span class="task-title">{title}</span>
                <span class="task-time-badge">{date}</span>
                <span class=badge_class>{label}</span>
                <div class="task-actions">
                    {can_reschedule
                        .then(|| {
                            view! {
                                <button
                                    class="icon-btn"
                                    title="Back to today"
                                    on:click=move |_| run(refresh, error, reschedule_today(id))
                                >
                                    "↩"
                                </button>
                            }
                        })}
                </div>
            </div>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Priority;

    fn task(title: &str, notes: &str, date: &str) -> Task {
        Task {
            id: 1,
            title: title.into(),
            notes: notes.into(),
            priority: Priority::Medium,
            completed: false,
            rollover: false,
            due_date: date.into(),
            slice_id: None,
            start_time: None,
        }
    }

    #[test]
    fn search_matches_title_notes_and_date_ignoring_case() {
        let t = task("Lab 4 upload", "[ ] zip the files", "2026-10-02");
        assert!(matches(&t, ""));
        assert!(matches(&t, "LAB"));
        assert!(matches(&t, "zip"));
        assert!(matches(&t, "2026-10"));
        assert!(!matches(&t, "quiz"));
    }
}
