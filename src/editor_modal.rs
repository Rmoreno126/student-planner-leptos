//! The task dialog (new and edit) and the IDE-style notes editor inside it.
//!
//! The editor is a real textarea with an identical, click-through "mirror" layer behind
//! it that repaints the text with the checkboxes highlighted. Typing, Tab, and clicks are
//! handled here; what the text should become is decided by `crate::editor`.

use leptos::prelude::*;

use crate::api::{add_task, get_toolbar, move_task, save_toolbar, update_task};
use crate::editor::{
    byte_to_utf16, insert_snippet, normalize_marker, segments, tab, toggle_at, utf16_to_byte, Edit,
    Segment,
};
use crate::model::{NewTask, Priority, Task, TaskUpdate};
use crate::slices::{blocked_at, open_slice_id_at, parse_hhmm, Slice, SliceKind};
use crate::snippets::{find, join, parse, sanitize, CATALOG, MAX_BUTTONS};

/// Minutes since midnight as `HH:MM`, the format a time input expects.
fn hhmm(minutes: u32) -> String {
    format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60)
}

/// A blank task that opens the dialog in "new task" mode (id 0).
pub fn blank_task() -> Task {
    Task {
        id: 0,
        title: String::new(),
        notes: String::new(),
        priority: Priority::High,
        completed: false,
        rollover: false,
        due_date: String::new(),
        slice_id: None,
        start_time: None,
    }
}

/// Reads the textarea's selection as byte offsets into `text`.
fn caret_pair(input: NodeRef<leptos::html::Textarea>, text: &str) -> (usize, usize) {
    match input.get() {
        Some(el) => {
            let start = el.selection_start().ok().flatten().unwrap_or(0);
            let end = el.selection_end().ok().flatten().unwrap_or(start);
            (
                utf16_to_byte(text, start as usize),
                utf16_to_byte(text, end as usize),
            )
        }
        None => (text.len(), text.len()),
    }
}

/// Writes an edit into the textarea first (so the caret survives), then into the signal.
fn apply(input: NodeRef<leptos::html::Textarea>, value: RwSignal<String>, edit: Edit) {
    if let Some(el) = input.get() {
        el.set_value(&edit.text);
        let caret = u32::try_from(byte_to_utf16(&edit.text, edit.caret)).unwrap_or(0);
        let _ = el.set_selection_range(caret, caret);
        let _ = el.focus();
    }
    value.set(edit.text);
}

/// Ticks or unticks the `index`th checkbox and keeps the caret where it was.
fn toggle_marker(input: NodeRef<leptos::html::Textarea>, value: RwSignal<String>, index: usize) {
    let text = value.get_untracked();
    let (caret, _) = caret_pair(input, &text);
    if let Some(edit) = toggle_at(&text, index, caret) {
        apply(input, value, edit);
    }
}

/// A VS Code-style notes editor: real typing, live checkbox markers, snippet buttons.
#[component]
fn IdeEditor(value: RwSignal<String>, toolbar: RwSignal<Vec<String>>) -> impl IntoView {
    let input = NodeRef::<leptos::html::Textarea>::new();
    let customizing = RwSignal::new(false);

    let insert = move |snippet: &'static str| {
        let text = value.get_untracked();
        let (_, end) = caret_pair(input, &text);
        apply(input, value, insert_snippet(&text, end, snippet));
    };

    let buttons = move || {
        toolbar
            .get()
            .into_iter()
            .filter_map(|id| find(&id))
            .map(|snip| {
                view! {
                    <button type="button" class="ide-btn" on:click=move |_| insert(snip.text)>
                        {snip.label}
                    </button>
                }
            })
            .collect::<Vec<_>>()
    };

    let mirror = move || {
        let text = value.get();
        let mut parts: Vec<AnyView> = segments(&text)
            .into_iter()
            .map(|seg| match seg {
                Segment::Plain(t) => view! { <span>{t}</span> }.into_any(),
                Segment::Done(t) => view! { <span class="ide-done">{t}</span> }.into_any(),
                Segment::Marker { index, checked, text } => {
                    let class = if checked { "cb cb-on" } else { "cb cb-off" };
                    view! { <span class=class on:mousedown=move |ev| ev.prevent_default() on:click=move |_| toggle_marker(input, value, index)>{text}</span> }.into_any()
                }
            })
            .collect();
        parts.push(view! { <span>"\u{200b}"</span> }.into_any());
        parts
    };

    view! {
        <div class="ide">
            <div class="ide-toolbar">
                {buttons}
                <button
                    type="button"
                    class="ide-btn"
                    title="Choose and reorder your buttons"
                    on:click=move |_| customizing.update(|open| *open = !*open)
                >
                    "⚙"
                </button>
            </div>
            {move || {
                customizing
                    .get()
                    .then(|| view! { <ToolbarSettings toolbar=toolbar customizing=customizing/> })
            }}
            <div class="ide-stack">
                <pre class="ide-mirror" aria-hidden="true">{mirror}</pre>
                <textarea
                    class="ide-input"
                    node_ref=input
                    placeholder="Type here. [] then space makes a checkbox. Click it to tick it."
                    prop:value=move || value.get()
                    on:input=move |ev| {
                        let new_text = event_target_value(&ev);
                        let grew_by_one = new_text.len() == value.get_untracked().len() + 1;
                        let (_, caret) = caret_pair(input, &new_text);
                        match normalize_marker(&new_text, caret) {
                            Some(edit) if grew_by_one => apply(input, value, edit),
                            _ => value.set(new_text),
                        }
                    }
                    on:keydown=move |ev| {
                        match ev.key().as_str() {
                            "Tab" if !ev.shift_key() => {
                                ev.prevent_default();
                                let text = value.get_untracked();
                                let (start, end) = caret_pair(input, &text);
                                apply(input, value, tab(&text, start, end));
                            }
                            "Escape" => {
                                if let Some(el) = input.get() {
                                    let _ = el.blur();
                                }
                            }
                            _ => {}
                        }
                    }
                ></textarea>
            </div>
        </div>
    }
}

/// Pick and order the editor's snippet buttons (up to 7), then save them.
#[component]
fn ToolbarSettings(toolbar: RwSignal<Vec<String>>, customizing: RwSignal<bool>) -> impl IntoView {
    let draft = RwSignal::new(toolbar.get_untracked());
    let message = RwSignal::new(None::<String>);

    let chosen = move || {
        draft
            .get()
            .into_iter()
            .enumerate()
            .filter_map(|(i, id)| find(&id).map(|snip| (i, snip)))
            .map(|(i, snip)| {
                view! {
                    <span class="ide-chip on">
                        {snip.label}
                        <button
                            type="button"
                            title="Move left"
                            on:click=move |_| {
                                draft.update(|d| {
                                    if i > 0 {
                                        d.swap(i, i - 1);
                                    }
                                })
                            }
                        >
                            "←"
                        </button>
                        <button
                            type="button"
                            title="Move right"
                            on:click=move |_| {
                                draft.update(|d| {
                                    if i + 1 < d.len() {
                                        d.swap(i, i + 1);
                                    }
                                })
                            }
                        >
                            "→"
                        </button>
                        <button
                            type="button"
                            title="Remove"
                            on:click=move |_| {
                                draft.update(|d| {
                                    if i < d.len() {
                                        d.remove(i);
                                    }
                                })
                            }
                        >
                            "✖"
                        </button>
                    </span>
                }
            })
            .collect::<Vec<_>>()
    };

    let available = move || {
        let current = draft.get();
        CATALOG
            .iter()
            .filter(|s| !current.iter().any(|id| id == s.id))
            .map(|snip| {
                view! {
                    <button
                        type="button"
                        class="ide-chip"
                        on:click=move |_| {
                            draft.update(|d| {
                                if d.len() < MAX_BUTTONS {
                                    d.push(snip.id.to_string());
                                }
                            })
                        }
                    >
                        {format!("+ {}", snip.label)}
                    </button>
                }
            })
            .collect::<Vec<_>>()
    };

    let save = move |_| {
        let ids = join(&draft.get_untracked());
        leptos::task::spawn_local(async move {
            match save_toolbar(ids).await {
                Ok(saved) => {
                    toolbar.set(sanitize(&parse(&saved)));
                    customizing.set(false);
                }
                Err(e) => message.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <div class="ide-settings">
            <p class="settings-subtitle">
                "Pick and order up to 7 buttons. Each one inserts on a new line."
            </p>
            <div class="ide-settings-row">{chosen}</div>
            <div class="ide-settings-row">{available}</div>
            {move || message.get().map(|text| view! { <p class="error">{text}</p> })}
            <div class="ide-settings-row">
                <button type="button" class="btn-primary" on:click=save>"Save buttons"</button>
                <button type="button" class="btn-secondary" on:click=move |_| customizing.set(false)>
                    "Cancel"
                </button>
            </div>
        </div>
    }
}

/// The task dialog: a new task when `task.id == 0`, otherwise an edit.
#[component]
pub fn EditModal(
    task: Task,
    slices: Vec<Slice>,
    wake: u32,
    editing: RwSignal<Option<Task>>,
    refresh: RwSignal<u32>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let id = task.id;
    let heading = if id == 0 { "New Task" } else { "Edit Task" };
    let open_ids: Vec<String> = slices
        .iter()
        .filter(|s| s.kind == SliceKind::Open)
        .map(|s| s.id.clone())
        .collect();
    let title = RwSignal::new(task.title);
    let notes = RwSignal::new(task.notes);
    let priority = RwSignal::new(task.priority);
    let time = RwSignal::new(task.start_time.map(hhmm).unwrap_or_default());
    let slice_sel = RwSignal::new(
        task.slice_id
            .filter(|s| open_ids.contains(s))
            .unwrap_or_default(),
    );

    let toolbar = RwSignal::new(sanitize(&[]));
    leptos::task::spawn_local(async move {
        if let Ok(csv) = get_toolbar().await {
            toolbar.set(sanitize(&parse(&csv)));
        }
    });

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
        let notes_text = notes.get_untracked();
        let level = priority.get_untracked();
        leptos::task::spawn_local(async move {
            let saved: Result<Task, ServerFnError> = if id == 0 {
                add_task(NewTask {
                    title: new_title,
                    notes: notes_text,
                    priority: level,
                    due_date: None,
                    slice_id,
                    start_time: minutes,
                })
                .await
            } else {
                async {
                    update_task(
                        id,
                        TaskUpdate {
                            title: Some(new_title),
                            notes: Some(notes_text),
                            priority: Some(level),
                        },
                    )
                    .await?;
                    move_task(id, slice_id, minutes).await
                }
                .await
            };
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
                    <h3>{heading}</h3>
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
                        <div class="edit-field-group flex-1">
                            <label>"Priority"</label>
                            <select
                                class="input-field"
                                on:change=move |ev| {
                                    let chosen = match event_target_value(&ev).as_str() {
                                        "1" => Priority::Low,
                                        "2" => Priority::Medium,
                                        _ => Priority::High,
                                    };
                                    priority.set(chosen);
                                }
                            >
                                <option value="3" prop:selected=move || priority.get() == Priority::High>
                                    "High"
                                </option>
                                <option value="2" prop:selected=move || priority.get() == Priority::Medium>
                                    "Med"
                                </option>
                                <option value="1" prop:selected=move || priority.get() == Priority::Low>
                                    "Low"
                                </option>
                            </select>
                        </div>
                    </div>
                    <label class="ide-label">"Notes (type like an editor)"</label>
                    <IdeEditor value=notes toolbar=toolbar/>
                </div>
                <div class="modal-footer">
                    <button type="button" class="btn-secondary" on:click=close>"Cancel"</button>
                    <button type="button" class="btn-primary" on:click=save>"Save"</button>
                </div>
            </div>
        </div>
    }
}
