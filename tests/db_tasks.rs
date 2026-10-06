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
        duration_minutes: 15,
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
async fn overdue_is_missed_not_daily_and_next_week_is_planned(pool: PgPool) -> sqlx::Result<()> {
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
    assert!(daily.is_empty());
    assert_eq!(db::list_missed(&pool, &today).await?.len(), 1);
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
        duration_minutes: None,
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

#[sqlx::test]
async fn rollover_tasks_stay_on_the_daily_plan_until_done(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let mut new = new_task("carry me");
    new.due_date = Some("2000-01-01".into());
    let task = db::insert(&pool, &new, &today).await?;
    assert!(db::list_daily(&pool, &today).await?.is_empty());
    db::toggle_rollover(&pool, task.id).await?;
    assert_eq!(db::list_daily(&pool, &today).await?.len(), 1);
    db::toggle_complete(&pool, task.id).await?;
    assert!(db::list_daily(&pool, &today).await?.is_empty());
    assert_eq!(db::list_done(&pool, &today).await?.len(), 1);
    Ok(())
}

#[sqlx::test]
async fn archive_all_moves_missed_and_done_but_deletes_nothing(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let mut old = new_task("missed one");
    old.due_date = Some("2000-01-01".into());
    db::insert(&pool, &old, &today).await?;
    let mut finished = new_task("finished one");
    finished.due_date = Some("2000-01-02".into());
    let finished = db::insert(&pool, &finished, &today).await?;
    db::toggle_complete(&pool, finished.id).await?;
    db::insert(&pool, &new_task("today's task"), &today).await?;

    assert_eq!(db::list_missed(&pool, &today).await?.len(), 1);
    assert_eq!(db::list_done(&pool, &today).await?.len(), 1);
    assert_eq!(db::archive_past(&pool, &today).await?, 2);
    assert!(db::list_missed(&pool, &today).await?.is_empty());
    assert!(db::list_done(&pool, &today).await?.is_empty());
    assert_eq!(db::list_archived(&pool).await?.len(), 2);
    assert_eq!(db::list_daily(&pool, &today).await?.len(), 1);
    Ok(())
}

#[sqlx::test]
async fn reschedule_brings_an_archived_task_back_to_today(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let mut old = new_task("second chance");
    old.due_date = Some("2000-01-01".into());
    old.slice_id = Some("slice_1".into());
    old.start_time = Some(600);
    let old = db::insert(&pool, &old, &today).await?;
    db::archive_past(&pool, &today).await?;
    assert_eq!(db::list_archived(&pool).await?.len(), 1);

    let back = db::reschedule_today(&pool, old.id, &today)
        .await?
        .expect("task exists");
    assert_eq!(back.due_date, today);
    assert_eq!(back.slice_id, None);
    assert_eq!(back.start_time, None);
    assert!(db::list_archived(&pool).await?.is_empty());
    assert_eq!(db::list_daily(&pool, &today).await?.len(), 1);
    Ok(())
}

#[sqlx::test]
async fn settings_round_trip(pool: PgPool) -> sqlx::Result<()> {
    assert_eq!(db::get_setting(&pool, "toolbar").await?, None);
    db::set_setting(&pool, "toolbar", "task,list").await?;
    db::set_setting(&pool, "toolbar", "task,bold").await?;
    assert_eq!(
        db::get_setting(&pool, "toolbar").await?.as_deref(),
        Some("task,bold")
    );
    Ok(())
}

use student_planner_leptos::model::NewBlock;

#[sqlx::test]
async fn the_real_timetable_is_preloaded(pool: PgPool) -> sqlx::Result<()> {
    let blocks = db::list_blocks(&pool).await?;
    let count = |day: u8| blocks.iter().filter(|b| b.weekday == Some(day)).count();
    assert_eq!(
        [count(0), count(1), count(2), count(3), count(4), count(5)],
        [3, 3, 2, 2, 2, 0]
    );
    let monday: Vec<&str> = blocks
        .iter()
        .filter(|b| b.weekday == Some(0))
        .map(|b| b.label.as_str())
        .collect();
    assert_eq!(
        monday,
        vec!["MATH 107 lecture", "CS 374 activity", "CS 453 lecture"]
    );
    Ok(())
}

#[sqlx::test]
async fn blocks_can_be_added_edited_and_removed(pool: PgPool) -> sqlx::Result<()> {
    let shift = db::insert_block(
        &pool,
        &NewBlock {
            label: "Barista shift".into(),
            weekday: None,
            on_date: Some("2026-10-08".into()),
            start: 840,
            end: 1080,
        },
    )
    .await?;
    assert_eq!(shift.on_date.as_deref(), Some("2026-10-08"));
    assert_eq!(shift.weekday, None);
    let moved = db::update_block(&pool, shift.id, "Closing shift", 900, 1140)
        .await?
        .expect("block exists");
    assert_eq!(
        (moved.label.as_str(), moved.start, moved.end),
        ("Closing shift", 900, 1140)
    );
    assert!(db::delete_block(&pool, shift.id).await?);
    assert!(!db::delete_block(&pool, shift.id).await?);
    Ok(())
}

#[sqlx::test]
async fn splits_are_unique_per_day_and_removable(pool: PgPool) -> sqlx::Result<()> {
    let first = db::insert_split(&pool, 5, 780).await?;
    let again = db::insert_split(&pool, 5, 780).await?;
    assert_eq!(first.id, again.id);
    db::insert_split(&pool, 5, 900).await?;
    assert_eq!(db::list_splits(&pool).await?.len(), 2);
    assert!(db::delete_split(&pool, first.id).await?);
    assert_eq!(db::list_splits(&pool).await?.len(), 1);
    Ok(())
}

#[sqlx::test]
async fn day_limits_default_then_save(pool: PgPool) -> sqlx::Result<()> {
    assert_eq!(db::day_limits(&pool).await?, (480, 1320));
    db::save_day_limits(&pool, 420, 1380).await?;
    assert_eq!(db::day_limits(&pool).await?, (420, 1380));
    let schedule = db::get_schedule(&pool).await?;
    assert_eq!((schedule.wake, schedule.sleep), (420, 1380));
    Ok(())
}

#[sqlx::test]
async fn duration_saves_and_updates(pool: PgPool) -> sqlx::Result<()> {
    let today = db::today(&pool).await?;
    let task = db::insert(&pool, &new_task("stretch"), &today).await?;
    assert_eq!(task.duration_minutes, 15);
    let changes = TaskUpdate {
        duration_minutes: Some(45),
        ..TaskUpdate::default()
    };
    let changed = db::update(&pool, task.id, &changes)
        .await?
        .expect("task exists");
    assert_eq!(changed.duration_minutes, 45);
    assert_eq!(changed.title, "stretch");
    Ok(())
}

#[sqlx::test]
async fn now_is_a_minute_of_the_day(pool: PgPool) -> sqlx::Result<()> {
    assert!(db::now_minutes(&pool).await? <= 1440);
    Ok(())
}
