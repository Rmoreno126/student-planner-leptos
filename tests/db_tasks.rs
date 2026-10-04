//! Database tests. They need a running Postgres:
//!   DATABASE_URL=postgres://postgres:dev@localhost:5432/planner \
//!   cargo test --test db_tasks --features ssr
#![cfg(feature = "ssr")]

use sqlx::PgPool;
use student_planner_leptos::db;
use student_planner_leptos::model::{is_iso_date, NewTask, Priority, TaskUpdate};

fn new_task(title: &str) -> NewTask {
    NewTask {
        title: title.into(),
        notes: String::new(),
        priority: Priority::High,
        due_date: None,
        slice_id: None,
        start_time: None,
    }
}

#[sqlx::test]
async fn today_looks_like_a_date(pool: PgPool) -> sqlx::Result<()> {
    assert!(is_iso_date(&db::today(&pool).await?));
    Ok(())
}

#[sqlx::test]
async fn insert_then_list_daily(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let task = db::insert(&pool, &new_task("write essay"), &today).await?;
    assert_eq!(task.title, "write essay");
    assert_eq!(task.priority, Priority::High);
    assert!(!task.completed && !task.rollover);
    assert_eq!(task.due_date, today);
    assert_eq!(db::list_daily(&pool, &today).await?, vec![task]);
    Ok(())
}

#[sqlx::test]
async fn overdue_is_daily_and_next_week_is_planned(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let in_three: String = sqlx::query_scalar("SELECT ($1::date + 3)::text")
        .bind(&today)
        .fetch_one(&pool)
        .await?;
    let mut overdue = new_task("overdue");
    overdue.due_date = Some("2000-01-01".into());
    let mut soon = new_task("soon");
    soon.due_date = Some(in_three);
    db::insert(&pool, &overdue, &today).await?;
    db::insert(&pool, &soon, &today).await?;

    let daily = db::list_daily(&pool, &today).await?;
    let planned = db::list_planned(&pool, &today).await?;
    assert_eq!(daily.len(), 1);
    assert_eq!(daily[0].title, "overdue");
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].title, "soon");
    Ok(())
}

#[sqlx::test]
async fn toggles_flip_and_unknown_ids_return_none(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let task = db::insert(&pool, &new_task("toggle me"), &today).await?;

    let done = db::toggle_complete(&pool, task.id)
        .await?
        .expect("task exists");
    assert!(done.completed);
    let undone = db::toggle_complete(&pool, task.id)
        .await?
        .expect("task exists");
    assert!(!undone.completed);
    let rolled = db::toggle_rollover(&pool, task.id)
        .await?
        .expect("task exists");
    assert!(rolled.rollover);
    assert!(db::toggle_complete(&pool, -1).await?.is_none());
    Ok(())
}

#[sqlx::test]
async fn update_ignores_blank_title(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let task = db::insert(&pool, &new_task("write essay"), &today).await?;
    let changes = TaskUpdate {
        title: Some("   ".into()),
        notes: Some("see syllabus".into()),
        priority: Some(Priority::Low),
    };
    let changed = db::update(&pool, task.id, &changes)
        .await?
        .expect("task exists");
    assert_eq!(changed.title, "write essay");
    assert_eq!(changed.notes, "see syllabus");
    assert_eq!(changed.priority, Priority::Low);
    Ok(())
}

#[sqlx::test]
async fn move_to_slice_and_delete(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let task = db::insert(&pool, &new_task("move me"), &today).await?;
    let moved = db::set_slice(&pool, task.id, Some("slice_2"), Some(810))
        .await?
        .expect("task exists");
    assert_eq!(moved.slice_id.as_deref(), Some("slice_2"));
    assert_eq!(moved.start_time, Some(810));
    assert!(db::delete(&pool, task.id).await?);
    assert!(!db::delete(&pool, task.id).await?);
    Ok(())
}

#[sqlx::test]
async fn weekday_is_a_day_name(pool: PgPool) -> sqlx::Result<()> {
    let day = db::weekday(&pool).await?;
    let days = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    assert!(days.contains(&day.as_str()));
    Ok(())
}
