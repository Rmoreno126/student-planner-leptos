//! Server functions: the browser calls these, the server runs them.

use crate::model::{NewTask, Task, TaskUpdate};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
fn fail(msg: impl ToString) -> ServerFnError {
    ServerFnError::new(msg)
}

#[cfg(feature = "ssr")]
fn missing() -> ServerFnError {
    fail("task not found")
}

#[cfg(feature = "ssr")]
fn pool() -> Result<&'static sqlx::PgPool, ServerFnError> {
    crate::db::pool().ok_or_else(|| fail("database not initialised"))
}

/// Tasks due today or earlier.
#[server]
pub async fn list_daily() -> Result<Vec<Task>, ServerFnError> {
    let pool = pool()?;
    let today = crate::db::today(pool).await.map_err(fail)?;
    crate::db::list_daily(pool, &today).await.map_err(fail)
}

/// Tasks due in the next seven days.
#[server]
pub async fn list_planned() -> Result<Vec<Task>, ServerFnError> {
    let pool = pool()?;
    let today = crate::db::today(pool).await.map_err(fail)?;
    crate::db::list_planned(pool, &today).await.map_err(fail)
}

/// Creates a task after validating it.
#[server]
pub async fn add_task(new: NewTask) -> Result<Task, ServerFnError> {
    new.validate().map_err(fail)?;
    let pool = pool()?;
    let today = crate::db::today(pool).await.map_err(fail)?;
    crate::db::insert(pool, &new, &today).await.map_err(fail)
}

/// Updates title, notes, or priority.
#[server]
pub async fn update_task(id: i64, changes: TaskUpdate) -> Result<Task, ServerFnError> {
    let task = crate::db::update(pool()?, id, &changes)
        .await
        .map_err(fail)?;
    task.ok_or_else(missing)
}

/// Flips a task between done and not done.
#[server]
pub async fn toggle_complete(id: i64) -> Result<Task, ServerFnError> {
    let task = crate::db::toggle_complete(pool()?, id)
        .await
        .map_err(fail)?;
    task.ok_or_else(missing)
}

/// Flips a task's "roll over to tomorrow" flag.
#[server]
pub async fn toggle_rollover(id: i64) -> Result<Task, ServerFnError> {
    let task = crate::db::toggle_rollover(pool()?, id)
        .await
        .map_err(fail)?;
    task.ok_or_else(missing)
}

/// Moves a task into a slice with an optional start time.
#[server]
pub async fn move_task(
    id: i64,
    slice_id: Option<String>,
    start_time: Option<u32>,
) -> Result<Task, ServerFnError> {
    let task = crate::db::set_slice(pool()?, id, slice_id.as_deref(), start_time)
        .await
        .map_err(fail)?;
    task.ok_or_else(missing)
}

/// Deletes a task.
#[server]
pub async fn delete_task(id: i64) -> Result<(), ServerFnError> {
    if crate::db::delete(pool()?, id).await.map_err(fail)? {
        Ok(())
    } else {
        Err(missing())
    }
}
