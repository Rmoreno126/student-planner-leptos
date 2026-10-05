//! Server-only database access (Postgres through SQLx).
//! Every query function takes the pool as an argument so it can be tested directly.

use crate::model::{NewTask, Priority, Task, TaskUpdate};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use std::sync::OnceLock;

/// Time zone that decides what "today" means.
pub const APP_TZ: &str = "America/Los_Angeles";

const COLUMNS: &str = "id, title, notes, priority, completed, rollover, \
                       due_date::text AS due_date, slice_id, start_time";

static POOL: OnceLock<PgPool> = OnceLock::new();

/// Connects using `DATABASE_URL`, applies migrations, and stores the pool.
pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let _ = POOL.set(pool);
    Ok(())
}

/// The shared pool, once [`init`] has run.
pub fn pool() -> Option<&'static PgPool> {
    POOL.get()
}

#[derive(FromRow)]
struct TaskRow {
    id: i64,
    title: String,
    notes: String,
    priority: i16,
    completed: bool,
    rollover: bool,
    due_date: String,
    slice_id: Option<String>,
    start_time: Option<i32>,
}

impl From<TaskRow> for Task {
    fn from(r: TaskRow) -> Self {
        Task {
            id: r.id,
            title: r.title,
            notes: r.notes,
            priority: Priority::from_code(r.priority).unwrap_or(Priority::Medium),
            completed: r.completed,
            rollover: r.rollover,
            due_date: r.due_date,
            slice_id: r.slice_id,
            start_time: r.start_time.and_then(|t| u32::try_from(t).ok()),
        }
    }
}

async fn one(pool: &PgPool, sql: &str, id: i64) -> Result<Option<Task>, sqlx::Error> {
    let row = sqlx::query_as::<_, TaskRow>(sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Task::from))
}

/// Today's date (`YYYY-MM-DD`) in [`APP_TZ`].
pub async fn today(pool: &PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT (now() AT TIME ZONE $1::text)::date::text")
        .bind(APP_TZ)
        .fetch_one(pool)
        .await
}

/// Tasks due today or earlier (overdue tasks roll into today, like the original app).
pub async fn list_daily(pool: &PgPool, today: &str) -> Result<Vec<Task>, sqlx::Error> {
    let sql = format!("SELECT {COLUMNS} FROM tasks WHERE NOT archived AND (due_date = $1::date OR (due_date < $1::date AND rollover AND NOT completed)) ORDER BY id");
    let rows = sqlx::query_as::<_, TaskRow>(&sql)
        .bind(today)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(Task::from).collect())
}

/// Tasks due in the seven days after `today`.
pub async fn list_planned(pool: &PgPool, today: &str) -> Result<Vec<Task>, sqlx::Error> {
    let sql = format!(
        "SELECT {COLUMNS} FROM tasks \
         WHERE NOT archived AND due_date > $1::date AND due_date <= $1::date + 7 ORDER BY due_date, id"
    );
    let rows = sqlx::query_as::<_, TaskRow>(&sql)
        .bind(today)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(Task::from).collect())
}

/// Inserts a task; a missing due date becomes `today`.
pub async fn insert(pool: &PgPool, new: &NewTask, today: &str) -> Result<Task, sqlx::Error> {
    let due = new.due_date.as_deref().unwrap_or(today);
    let sql = format!(
        "INSERT INTO tasks (title, notes, priority, due_date, slice_id, start_time) \
         VALUES ($1, $2, $3, $4::date, $5, $6) RETURNING {COLUMNS}"
    );
    let row = sqlx::query_as::<_, TaskRow>(&sql)
        .bind(new.title.trim())
        .bind(&new.notes)
        .bind(new.priority.code())
        .bind(due)
        .bind(new.slice_id.as_deref())
        .bind(new.start_time.and_then(|t| i32::try_from(t).ok()))
        .fetch_one(pool)
        .await?;
    Ok(row.into())
}

/// Applies a partial update. Blank titles are ignored, like the original app.
pub async fn update(
    pool: &PgPool,
    id: i64,
    changes: &TaskUpdate,
) -> Result<Option<Task>, sqlx::Error> {
    let title = changes
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let sql = format!(
        "UPDATE tasks SET title = COALESCE($2, title), notes = COALESCE($3, notes), \
         priority = COALESCE($4, priority) WHERE id = $1 RETURNING {COLUMNS}"
    );
    let row = sqlx::query_as::<_, TaskRow>(&sql)
        .bind(id)
        .bind(title)
        .bind(changes.notes.as_deref())
        .bind(changes.priority.map(Priority::code))
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Task::from))
}

/// Flips the completed flag.
pub async fn toggle_complete(pool: &PgPool, id: i64) -> Result<Option<Task>, sqlx::Error> {
    let sql =
        format!("UPDATE tasks SET completed = NOT completed WHERE id = $1 RETURNING {COLUMNS}");
    one(pool, &sql, id).await
}

/// Flips the rollover flag.
pub async fn toggle_rollover(pool: &PgPool, id: i64) -> Result<Option<Task>, sqlx::Error> {
    let sql = format!("UPDATE tasks SET rollover = NOT rollover WHERE id = $1 RETURNING {COLUMNS}");
    one(pool, &sql, id).await
}

/// Moves a task into a slice (or out of all slices with `None`).
pub async fn set_slice(
    pool: &PgPool,
    id: i64,
    slice_id: Option<&str>,
    start_time: Option<u32>,
) -> Result<Option<Task>, sqlx::Error> {
    let sql = format!(
        "UPDATE tasks SET slice_id = $2, start_time = $3 WHERE id = $1 RETURNING {COLUMNS}"
    );
    let row = sqlx::query_as::<_, TaskRow>(&sql)
        .bind(id)
        .bind(slice_id)
        .bind(start_time.and_then(|t| i32::try_from(t).ok()))
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Task::from))
}

/// Deletes a task; `true` if one was removed.
pub async fn delete(pool: &PgPool, id: i64) -> Result<bool, sqlx::Error> {
    let done = sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// Weekday name (for example `Monday`) in [`APP_TZ`].
pub async fn weekday(pool: &PgPool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT to_char(now() AT TIME ZONE $1::text, 'FMDay')")
        .bind(APP_TZ)
        .fetch_one(pool)
        .await
}

async fn fetch_dated(pool: &PgPool, sql: &str, today: &str) -> Result<Vec<Task>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TaskRow>(sql)
        .bind(today)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(Task::from).collect())
}

/// Planned for a past day, never finished, and not set to roll over.
pub async fn list_missed(pool: &PgPool, today: &str) -> Result<Vec<Task>, sqlx::Error> {
    let sql = format!(
        "SELECT {COLUMNS} FROM tasks WHERE NOT archived AND NOT completed AND NOT rollover \
         AND due_date < $1::date ORDER BY due_date DESC, id"
    );
    fetch_dated(pool, &sql, today).await
}

/// Finished tasks from a past day that are not archived yet.
pub async fn list_done(pool: &PgPool, today: &str) -> Result<Vec<Task>, sqlx::Error> {
    let sql = format!(
        "SELECT {COLUMNS} FROM tasks WHERE NOT archived AND completed \
         AND due_date < $1::date ORDER BY due_date DESC, id"
    );
    fetch_dated(pool, &sql, today).await
}

/// Everything the user archived, newest first.
pub async fn list_archived(pool: &PgPool) -> Result<Vec<Task>, sqlx::Error> {
    let sql = format!(
        "SELECT {COLUMNS} FROM tasks WHERE archived ORDER BY due_date DESC, id DESC LIMIT 500"
    );
    let rows = sqlx::query_as::<_, TaskRow>(&sql).fetch_all(pool).await?;
    Ok(rows.into_iter().map(Task::from).collect())
}

/// Archives every finished or missed task from before `today`. Returns how many moved.
pub async fn archive_past(pool: &PgPool, today: &str) -> Result<u64, sqlx::Error> {
    let done = sqlx::query(
        "UPDATE tasks SET archived = TRUE WHERE NOT archived AND due_date < $1::date \
         AND (completed OR NOT rollover)",
    )
    .bind(today)
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

/// Brings a task back to `today`: un-archived, not done, and without a slice yet.
pub async fn reschedule_today(
    pool: &PgPool,
    id: i64,
    today: &str,
) -> Result<Option<Task>, sqlx::Error> {
    let sql = format!(
        "UPDATE tasks SET due_date = $2::date, archived = FALSE, completed = FALSE, \
         slice_id = NULL, start_time = NULL WHERE id = $1 RETURNING {COLUMNS}"
    );
    let row = sqlx::query_as::<_, TaskRow>(&sql)
        .bind(id)
        .bind(today)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Task::from))
}
