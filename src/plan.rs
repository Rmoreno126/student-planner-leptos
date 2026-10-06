//! Pure layout for tasks inside open slices: where each task is drawn, what runs past
//! its slice, and how much time is left. No UI and no database, so it is unit-tested
//! here and wired to the Daily tab in `app.rs`.

use crate::model::Task;
use crate::slices::{Slice, SliceKind};

const DAY: u32 = 1440;

/// The day runs from 4 AM to 4 AM (matches `DAY_CUTOFF` in `db.rs`).
pub const DAY_CUTOFF_MIN: u32 = 240;

/// Where a slice sits relative to the current time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    Past,
    Now,
    Later,
}

/// One task as drawn inside a slice. Times use the slice's clock (may pass 1440).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub task: Task,
    pub start: u32,
    pub end: u32,
    /// It wanted an earlier start, but the task before it was still running.
    pub pushed: bool,
    /// Minutes it runs past the end of its slice.
    pub overflow: u32,
}

/// One open slice with its tasks laid out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlicePlan {
    pub slice: Slice,
    pub tasks: Vec<Placed>,
    /// Length of the slice.
    pub free: u32,
    /// Sum of the task durations.
    pub planned: u32,
    /// Free time not covered by any task.
    pub left: u32,
}

/// How many untimed tasks fit in the time left today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayFit {
    pub fits: usize,
    pub total: usize,
    pub left: u32,
}

/// The whole Daily board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayLayout {
    pub plans: Vec<SlicePlan>,
    pub unassigned: Vec<Task>,
    pub fit: DayFit,
}

/// Times before wake belong to the night after (00:30 with an 08:00 wake is 24:30).
pub fn day_clock(t: u32, wake: u32) -> u32 {
    if t < wake {
        t + DAY
    } else {
        t
    }
}

/// The current time on the day clock: 2:30 AM is still the night of the day before.
pub fn now_clock(minutes: u32) -> u32 {
    if minutes < DAY_CUTOFF_MIN {
        minutes + DAY
    } else {
        minutes
    }
}

/// Past, running now, or still ahead. With no clock (a preview day), everything is Later.
pub fn slice_when(slice: &Slice, now: Option<u32>) -> When {
    match now {
        Some(n) if n >= slice.end => When::Past,
        Some(n) if n >= slice.start => When::Now,
        _ => When::Later,
    }
}

/// Where "Move to next slice" sends an unfinished task: the slice running now, at the
/// current minute, or between slices, the start of the next one. `None` once the day is over.
pub fn move_target(slices: &[Slice], now: u32) -> Option<(String, u32)> {
    slices
        .iter()
        .find(|s| s.kind == SliceKind::Open && s.end > now)
        .map(|s| (s.id.clone(), now.max(s.start)))
}

/// Minutes two time ranges share.
fn overlap(a_start: u32, a_end: u32, b_start: u32, b_end: u32) -> u32 {
    a_end.min(b_end).saturating_sub(a_start.max(b_start))
}

/// Index of the open slice a task belongs to. A timed task goes by its start time;
/// an untimed one keeps its stored slice id, if that slice exists today.
pub fn slice_index(task: &Task, slices: &[Slice], wake: u32) -> Option<usize> {
    let open = |s: &Slice| s.kind == SliceKind::Open;
    match task.start_time {
        Some(t) => {
            let t = day_clock(t, wake);
            slices
                .iter()
                .position(|s| open(s) && (s.start..s.end).contains(&t))
        }
        None => {
            let id = task.slice_id.as_deref()?;
            slices.iter().position(|s| open(s) && s.id == id)
        }
    }
}

/// Lays out one slice. Timed tasks sit at their start time; one that would overlap the
/// task before it is pushed down to start when that one ends. Untimed tasks then stack
/// back to back from the slice start, flowing around the timed ones.
pub fn plan_slice(slice: &Slice, tasks: Vec<Task>, wake: u32) -> SlicePlan {
    let (mut timed, mut untimed): (Vec<Task>, Vec<Task>) =
        tasks.into_iter().partition(|t| t.start_time.is_some());
    timed.sort_by_key(|t| (day_clock(t.start_time.unwrap_or(0), wake), t.id));
    untimed.sort_by_key(|t| t.id);

    let mut placed: Vec<Placed> = Vec::new();
    let mut cursor = slice.start;
    for task in timed {
        let wanted = day_clock(task.start_time.unwrap_or(slice.start), wake);
        let start = wanted.max(cursor);
        let end = start + task.duration_minutes;
        cursor = end;
        placed.push(Placed {
            pushed: start > wanted,
            overflow: end.saturating_sub(slice.end),
            start,
            end,
            task,
        });
    }

    let busy: Vec<(u32, u32)> = placed.iter().map(|p| (p.start, p.end)).collect();
    let mut cursor = slice.start;
    for task in untimed {
        let length = task.duration_minutes;
        while let Some(&(_, busy_end)) = busy
            .iter()
            .find(|&&(s, e)| cursor < e && cursor + length > s)
        {
            cursor = busy_end;
        }
        let end = cursor + length;
        placed.push(Placed {
            pushed: false,
            overflow: end.saturating_sub(slice.end),
            start: cursor,
            end,
            task,
        });
        cursor = end;
    }
    placed.sort_by_key(|p| (p.start, p.task.id));

    let free = slice.end - slice.start;
    let planned: u32 = placed.iter().map(|p| p.task.duration_minutes).sum();
    let covered: u32 = placed
        .iter()
        .map(|p| overlap(p.start, p.end, slice.start, slice.end))
        .sum();
    SlicePlan {
        slice: slice.clone(),
        tasks: placed,
        free,
        planned,
        left: free.saturating_sub(covered),
    }
}

/// Untimed tasks, oldest first, counted until the time left after `now` runs out.
/// `now` is already on the day clock.
pub fn day_fit(plans: &[SlicePlan], unassigned: &[Task], now: Option<u32>) -> DayFit {
    let mut left = 0;
    let mut untimed: Vec<&Task> = unassigned
        .iter()
        .filter(|t| t.start_time.is_none())
        .collect();
    for plan in plans {
        let (start, end) = (plan.slice.start, plan.slice.end);
        let from = now.map_or(start, |n| n.max(start)).min(end);
        let timed: u32 = plan
            .tasks
            .iter()
            .filter(|p| p.task.start_time.is_some())
            .map(|p| overlap(p.start, p.end, from, end))
            .sum();
        left += (end - from).saturating_sub(timed);
        untimed.extend(
            plan.tasks
                .iter()
                .filter(|p| p.task.start_time.is_none())
                .map(|p| &p.task),
        );
    }
    untimed.sort_by_key(|t| t.id);
    let mut used = 0;
    let fits = untimed
        .iter()
        .take_while(|t| {
            used += t.duration_minutes;
            used <= left
        })
        .count();
    DayFit {
        fits,
        total: untimed.len(),
        left,
    }
}

/// Sorts tasks into slices, lays out each open slice, and counts what fits.
pub fn layout_day(slices: &[Slice], tasks: Vec<Task>, wake: u32, now: Option<u32>) -> DayLayout {
    let mut buckets: Vec<Vec<Task>> = vec![Vec::new(); slices.len()];
    let mut unassigned = Vec::new();
    for task in tasks {
        match slice_index(&task, slices, wake) {
            Some(i) => buckets[i].push(task),
            None => unassigned.push(task),
        }
    }
    let plans: Vec<SlicePlan> = slices
        .iter()
        .zip(buckets)
        .filter(|(slice, _)| slice.kind == SliceKind::Open)
        .map(|(slice, tasks)| plan_slice(slice, tasks, wake))
        .collect();
    let fit = day_fit(&plans, &unassigned, now.map(now_clock));
    DayLayout {
        plans,
        unassigned,
        fit,
    }
}

/// 45 -> "45m", 120 -> "2h", 190 -> "3h 10m".
pub fn fmt_duration(minutes: u32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// "3h 10m free · 45m planned · 2h 25m left"
pub fn capacity_label(plan: &SlicePlan) -> String {
    format!(
        "{} free · {} planned · {} left",
        fmt_duration(plan.free),
        fmt_duration(plan.planned),
        fmt_duration(plan.left)
    )
}

/// "10 min past this slice", or `None` when the task ends in time.
pub fn overflow_label(minutes: u32) -> Option<String> {
    match minutes {
        0 => None,
        m if m < 60 => Some(format!("{m} min past this slice")),
        m => Some(format!("{} past this slice", fmt_duration(m))),
    }
}

/// "3 of 5 to-dos fit in the 2h 10m left", or `None` with no untimed tasks.
pub fn fit_label(fit: DayFit) -> Option<String> {
    (fit.total > 0).then(|| {
        format!(
            "{} of {} to-dos fit in the {} left",
            fit.fits,
            fit.total,
            fmt_duration(fit.left)
        )
    })
}

/// Minutes of empty space a free gap needs at its top to show its capacity line.
pub const LABEL_MINUTES: u32 = 20;

/// True when the first `LABEL_MINUTES` of a slice are empty, so the capacity line
/// never sits on top of a task. Otherwise the UI shows it as a hover tooltip.
pub fn capacity_fits(plan: &SlicePlan) -> bool {
    plan.free >= LABEL_MINUTES
        && plan
            .tasks
            .iter()
            .all(|p| p.start >= plan.slice.start + LABEL_MINUTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Priority;
    use crate::slices::{compute_slices, parse_hhmm, Block, DaySchedule};

    const WAKE: u32 = 480;

    fn at(s: &str) -> u32 {
        parse_hhmm(s).unwrap()
    }

    fn task(id: i64, start: Option<&str>, minutes: u32) -> Task {
        Task {
            id,
            title: format!("task {id}"),
            notes: String::new(),
            priority: Priority::Medium,
            completed: false,
            rollover: false,
            due_date: "2026-10-05".into(),
            slice_id: None,
            start_time: start.map(at),
            duration_minutes: minutes,
        }
    }

    fn open(start: &str, end: &str) -> Slice {
        Slice {
            id: "slice_1".into(),
            kind: SliceKind::Open,
            label: String::new(),
            start: at(start),
            end: at(end),
        }
    }

    fn block(start: &str, end: &str) -> Block {
        Block {
            label: "Class".into(),
            start: at(start),
            end: at(end),
        }
    }

    /// Open 8-10, class 10-1, open 1-4, class 4-5, open 5-10.
    fn monday() -> Vec<Slice> {
        let day = DaySchedule {
            blocks: vec![block("10:00", "13:00"), block("16:00", "17:00")],
            divisions: vec![],
        };
        compute_slices(WAKE, at("22:00"), &day)
    }

    fn starts(plan: &SlicePlan) -> Vec<u32> {
        plan.tasks.iter().map(|p| p.start).collect()
    }

    #[test]
    fn slices_know_if_they_are_past_now_or_later() {
        let day = monday();
        let (first, second) = (&day[0], &day[2]);
        assert_eq!(slice_when(first, Some(at("11:00"))), When::Past);
        assert_eq!(slice_when(second, Some(at("11:00"))), When::Later);
        assert_eq!(slice_when(second, Some(at("14:00"))), When::Now);
        assert_eq!(slice_when(second, None), When::Later);
    }

    #[test]
    fn the_day_turns_over_at_4_am() {
        assert_eq!(now_clock(at("02:30")), DAY + at("02:30"));
        assert_eq!(now_clock(at("05:00")), at("05:00"));
    }

    #[test]
    fn move_target_is_now_or_the_next_slice() {
        let day = monday();
        assert_eq!(
            move_target(&day, at("14:00")),
            Some(("slice_2".into(), at("14:00")))
        );
        assert_eq!(
            move_target(&day, at("11:00")),
            Some(("slice_2".into(), at("13:00")))
        );
        assert_eq!(move_target(&day, at("23:00")), None);
    }

    #[test]
    fn timed_task_sits_at_its_start() {
        let plan = plan_slice(
            &open("13:00", "16:00"),
            vec![task(1, Some("14:00"), 30)],
            WAKE,
        );
        let p = &plan.tasks[0];
        assert_eq!(
            (p.start, p.end, p.pushed, p.overflow),
            (at("14:00"), at("14:30"), false, 0)
        );
    }

    #[test]
    fn overlapping_task_is_pushed_down() {
        let tasks = vec![task(1, Some("14:00"), 60), task(2, Some("14:30"), 30)];
        let plan = plan_slice(&open("13:00", "16:00"), tasks, WAKE);
        assert_eq!(starts(&plan), vec![at("14:00"), at("15:00")]);
        assert!(plan.tasks[1].pushed);
    }

    #[test]
    fn untimed_tasks_stack_from_the_start_around_timed_ones() {
        let tasks = vec![
            task(1, None, 15),
            task(2, Some("13:15"), 30),
            task(3, None, 15),
        ];
        let plan = plan_slice(&open("13:00", "16:00"), tasks, WAKE);
        assert_eq!(starts(&plan), vec![at("13:00"), at("13:15"), at("13:45")]);
    }

    #[test]
    fn overflow_is_measured_and_one_task_may_fill_the_slice() {
        let late = plan_slice(
            &open("13:00", "16:00"),
            vec![task(1, Some("15:40"), 30)],
            WAKE,
        );
        assert_eq!(late.tasks[0].overflow, 10);
        assert_eq!(
            overflow_label(10).as_deref(),
            Some("10 min past this slice")
        );
        assert_eq!(overflow_label(0), None);
        let full = plan_slice(&open("13:00", "16:00"), vec![task(1, None, 180)], WAKE);
        assert_eq!((full.left, full.tasks[0].overflow), (0, 0));
    }

    #[test]
    fn header_shows_free_planned_and_left() {
        let tasks = vec![task(1, None, 15), task(2, Some("14:00"), 30)];
        let plan = plan_slice(&open("13:00", "16:10"), tasks, WAKE);
        assert_eq!(
            capacity_label(&plan),
            "3h 10m free · 45m planned · 2h 25m left"
        );
    }

    #[test]
    fn tasks_go_by_start_time_not_by_stored_slice() {
        let mut moved = task(1, Some("14:00"), 30);
        moved.slice_id = Some("slice_1".into());
        let mut kept = task(2, None, 30);
        kept.slice_id = Some("slice_3".into());
        let mut stale = task(3, None, 30);
        stale.slice_id = Some("slice_9".into());
        let in_class = task(4, Some("11:00"), 30);
        let day = layout_day(&monday(), vec![moved, kept, stale, in_class], WAKE, None);
        let ids = |i: usize| -> Vec<i64> { day.plans[i].tasks.iter().map(|p| p.task.id).collect() };
        assert!(ids(0).is_empty());
        assert_eq!(ids(1), vec![1]);
        assert_eq!(ids(2), vec![2]);
        let loose: Vec<i64> = day.unassigned.iter().map(|t| t.id).collect();
        assert_eq!(loose, vec![3, 4]);
    }

    #[test]
    fn day_fit_counts_untimed_tasks_in_order() {
        let tasks = vec![
            task(1, None, 60),
            task(2, None, 90),
            task(3, None, 120),
            task(4, None, 90),
            task(5, None, 30),
        ];
        // At 3 PM: 1h left in 1-4, 5h in 5-10, morning is over.
        let day = layout_day(&monday(), tasks, WAKE, Some(at("15:00")));
        assert_eq!(
            day.fit,
            DayFit {
                fits: 4,
                total: 5,
                left: 360
            }
        );
        assert_eq!(
            fit_label(day.fit).as_deref(),
            Some("4 of 5 to-dos fit in the 6h left")
        );
    }

    #[test]
    fn times_after_midnight_land_in_the_late_slice() {
        let slices = compute_slices(WAKE, at("01:00"), &DaySchedule::default());
        let day = layout_day(&slices, vec![task(1, Some("00:30"), 15)], WAKE, None);
        assert_eq!(day.plans[0].tasks[0].start, DAY + at("00:30"));
        assert!(day.unassigned.is_empty());
    }

    #[test]
    fn durations_read_short() {
        assert_eq!(fmt_duration(45), "45m");
        assert_eq!(fmt_duration(120), "2h");
        assert_eq!(fmt_duration(190), "3h 10m");
    }

    #[test]
    fn capacity_line_only_shows_where_it_has_room() {
        let empty = plan_slice(&open("18:00", "22:00"), vec![], WAKE);
        assert!(capacity_fits(&empty));
        let busy_top = plan_slice(&open("18:00", "22:00"), vec![task(1, None, 15)], WAKE);
        assert!(!capacity_fits(&busy_top));
        let later = plan_slice(
            &open("18:00", "22:00"),
            vec![task(1, Some("19:00"), 15)],
            WAKE,
        );
        assert!(capacity_fits(&later));
        let tiny = plan_slice(&open("16:50", "17:00"), vec![], WAKE);
        assert!(!capacity_fits(&tiny));
    }
}
