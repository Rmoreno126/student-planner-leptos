//! Server functions: the browser calls these, the server runs them.

use crate::model::{
    DayData, History, NewBlock, NewTask, Schedule, ScheduleBlock, Split, Task, TaskUpdate,
};
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

/// Today's weekday name in the app time zone, for example `Monday`.
#[server]
pub async fn today_name() -> Result<String, ServerFnError> {
    crate::db::weekday(pool()?).await.map_err(fail)
}

/// Missed, finished, and archived tasks for the History tab.
#[server]
pub async fn list_history() -> Result<History, ServerFnError> {
    let pool = pool()?;
    let today = crate::db::today(pool).await.map_err(fail)?;
    Ok(History {
        missed: crate::db::list_missed(pool, &today).await.map_err(fail)?,
        done: crate::db::list_done(pool, &today).await.map_err(fail)?,
        archived: crate::db::list_archived(pool).await.map_err(fail)?,
    })
}

/// Archives every missed or finished task from before today. Returns how many moved.
#[server]
pub async fn archive_all() -> Result<u64, ServerFnError> {
    let pool = pool()?;
    let today = crate::db::today(pool).await.map_err(fail)?;
    crate::db::archive_past(pool, &today).await.map_err(fail)
}

/// Puts a task back on today's plan.
#[server]
pub async fn reschedule_today(id: i64) -> Result<Task, ServerFnError> {
    let pool = pool()?;
    let today = crate::db::today(pool).await.map_err(fail)?;
    let task = crate::db::reschedule_today(pool, id, &today)
        .await
        .map_err(fail)?;
    task.ok_or_else(missing)
}

/// The saved editor toolbar as comma-separated snippet ids.
#[server]
pub async fn get_toolbar() -> Result<String, ServerFnError> {
    let saved = crate::db::get_setting(pool()?, "toolbar")
        .await
        .map_err(fail)?;
    let ids = crate::snippets::parse(&saved.unwrap_or_default());
    Ok(crate::snippets::join(&crate::snippets::sanitize(&ids)))
}

/// Saves the editor toolbar after cleaning it up; returns what was saved.
#[server]
pub async fn save_toolbar(ids: String) -> Result<String, ServerFnError> {
    let clean = crate::snippets::join(&crate::snippets::sanitize(&crate::snippets::parse(&ids)));
    crate::db::set_setting(pool()?, "toolbar", &clean)
        .await
        .map_err(fail)?;
    Ok(clean)
}

/// The blocked windows, split times, and wake/bed times for one day.
/// `preview` (a weekday name) lets the page show another day's layout.
#[server]
pub async fn get_day(preview: Option<String>) -> Result<DayData, ServerFnError> {
    let pool = pool()?;
    let today_name = crate::db::weekday(pool).await.map_err(fail)?;
    let today = crate::db::today(pool).await.map_err(fail)?;
    let chosen = preview
        .filter(|name| crate::model::weekday_index(name).is_some())
        .unwrap_or_else(|| today_name.clone());
    let index = crate::model::weekday_index(&chosen).unwrap_or(0);
    let date = (chosen == today_name).then_some(today.as_str());
    let schedule = crate::db::get_schedule(pool).await.map_err(fail)?;
    Ok(schedule.day(&chosen, index, date))
}

/// The whole schedule for the Weekly Schedule tab.
#[server]
pub async fn get_schedule() -> Result<Schedule, ServerFnError> {
    crate::db::get_schedule(pool()?).await.map_err(fail)
}

/// Adds a weekly or one-time blocked window.
#[server]
pub async fn add_block(new: NewBlock) -> Result<ScheduleBlock, ServerFnError> {
    new.validate().map_err(fail)?;
    crate::db::insert_block(pool()?, &new).await.map_err(fail)
}

/// Changes a block's name and times.
#[server]
pub async fn update_block(
    id: i64,
    label: String,
    start: u32,
    end: u32,
) -> Result<ScheduleBlock, ServerFnError> {
    crate::model::check_block(&label, start, end).map_err(fail)?;
    let block = crate::db::update_block(pool()?, id, label.trim(), start, end)
        .await
        .map_err(fail)?;
    block.ok_or_else(|| fail("block not found"))
}

/// Removes a block.
#[server]
pub async fn delete_block(id: i64) -> Result<(), ServerFnError> {
    if crate::db::delete_block(pool()?, id).await.map_err(fail)? {
        Ok(())
    } else {
        Err(fail("block not found"))
    }
}

/// Adds a split time for a weekday.
#[server]
pub async fn add_split(weekday: u8, at: u32) -> Result<Split, ServerFnError> {
    if weekday > 6 || at > 1439 {
        return Err(fail("That split time is out of range"));
    }
    crate::db::insert_split(pool()?, weekday, at)
        .await
        .map_err(fail)
}

/// Removes a split time.
#[server]
pub async fn delete_split(id: i64) -> Result<(), ServerFnError> {
    if crate::db::delete_split(pool()?, id).await.map_err(fail)? {
        Ok(())
    } else {
        Err(fail("split not found"))
    }
}

/// Saves wake-up time and bedtime.
#[server]
pub async fn save_day_limits(wake: u32, sleep: u32) -> Result<(), ServerFnError> {
    if wake > 1439 || sleep > 1439 || wake == sleep {
        return Err(fail("Pick a wake-up time and a different bedtime"));
    }
    crate::db::save_day_limits(pool()?, wake, sleep)
        .await
        .map_err(fail)
}
