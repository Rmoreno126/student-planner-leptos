//! Shared data types. Compiled for both the server and the browser,
//! so nothing in here may depend on the database.

use serde::{Deserialize, Serialize};

/// Task priority. Codes match the original app (1 = low, 3 = high).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    /// Numeric code stored in the database.
    pub fn code(self) -> i16 {
        match self {
            Priority::Low => 1,
            Priority::Medium => 2,
            Priority::High => 3,
        }
    }

    /// Inverse of [`Priority::code`]; `None` for unknown codes.
    pub fn from_code(code: i16) -> Option<Self> {
        match code {
            1 => Some(Priority::Low),
            2 => Some(Priority::Medium),
            3 => Some(Priority::High),
            _ => None,
        }
    }

    /// Short label shown on the priority badge.
    pub fn label(self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Medium => "Med",
            Priority::High => "High",
        }
    }
}

/// A task as stored and shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub notes: String,
    pub priority: Priority,
    pub completed: bool,
    pub rollover: bool,
    /// ISO date, `YYYY-MM-DD`.
    pub due_date: String,
    /// Id of the open slice this task sits in, e.g. `slice_2`. `None` = unsliced.
    pub slice_id: Option<String>,
    /// Start time in minutes since midnight.
    pub start_time: Option<u32>,
    /// Planned length in minutes (15 by default).
    pub duration_minutes: u32,
}

/// Fields needed to create a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewTask {
    pub title: String,
    pub notes: String,
    pub priority: Priority,
    /// `None` means today (in the app time zone).
    pub due_date: Option<String>,
    pub slice_id: Option<String>,
    pub start_time: Option<u32>,
    /// Planned length in minutes (15 by default).
    pub duration_minutes: u32,
}

/// Partial update. `None` fields are left unchanged; a blank title is ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub priority: Option<Priority>,
    pub duration_minutes: Option<u32>,
}

/// Largest start time allowed (47:59, so bedtimes past midnight fit).
pub const MAX_START_TIME: u32 = 2879;
/// A new task's length when none is chosen.
pub const DEFAULT_DURATION: u32 = 15;
/// Longest allowed task: a whole day.
pub const MAX_DURATION: u32 = 1440;

impl NewTask {
    /// Checks the fields before they reach the database.
    pub fn validate(&self) -> Result<(), String> {
        let title = self.title.trim();
        if title.is_empty() {
            return Err("Task title cannot be empty".into());
        }
        if title.chars().count() > 200 {
            return Err("Task title is too long (max 200 characters)".into());
        }
        if let Some(date) = &self.due_date {
            if !is_iso_date(date) {
                return Err("Due date must look like YYYY-MM-DD".into());
            }
        }
        if let Some(time) = self.start_time {
            if time > MAX_START_TIME {
                return Err("Start time is out of range".into());
            }
        }
        if !(1..=MAX_DURATION).contains(&self.duration_minutes) {
            return Err("Duration must be between 1 minute and 24 hours".into());
        }
        Ok(())
    }
}

/// Shape check only (`YYYY-MM-DD`); Postgres rejects impossible dates.
pub fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Everything the History tab shows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    /// Planned for a past day, never finished, and not set to roll over.
    pub missed: Vec<Task>,
    /// Finished tasks from past days.
    pub done: Vec<Task>,
    /// Everything the user archived.
    pub archived: Vec<Task>,
}

/// Weekday names, Monday first. The position is the number stored in the database.
pub const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Monday is 0 ... Sunday is 6.
pub fn weekday_index(name: &str) -> Option<u8> {
    WEEKDAY_NAMES
        .iter()
        .position(|day| *day == name)
        .and_then(|i| u8::try_from(i).ok())
}

/// One blocked window of the day: a class, a shift, an appointment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleBlock {
    pub id: i64,
    pub label: String,
    /// 0 = Monday ... 6 = Sunday. `None` for a one-time block.
    pub weekday: Option<u8>,
    /// ISO date (`YYYY-MM-DD`) for a one-time block.
    pub on_date: Option<String>,
    /// Start, in minutes since midnight.
    pub start: u32,
    /// End, in minutes since midnight.
    pub end: u32,
}

/// A new blocked window: weekly (a weekday) or one-time (a date), never both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewBlock {
    pub label: String,
    pub weekday: Option<u8>,
    pub on_date: Option<String>,
    pub start: u32,
    pub end: u32,
}

/// A time that splits a weekday's open time into separate slices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Split {
    pub id: i64,
    pub weekday: u8,
    pub at: u32,
}

/// The whole schedule: day limits, blocked windows, and split times.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    pub wake: u32,
    pub sleep: u32,
    pub blocks: Vec<ScheduleBlock>,
    pub splits: Vec<Split>,
}

/// Everything the Daily tab needs to lay out one day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayData {
    pub name: String,
    pub date: String,
    pub wake: u32,
    pub sleep: u32,
    pub blocks: Vec<ScheduleBlock>,
    pub divisions: Vec<u32>,
    /// Current minute of the day, only when this is today's plan (not a preview).
    pub now: Option<u32>,
}

/// Checks a block's name and times.
pub fn check_block(label: &str, start: u32, end: u32) -> Result<(), String> {
    let label = label.trim();
    if label.is_empty() {
        return Err("Give the block a name".into());
    }
    if label.chars().count() > 80 {
        return Err("The name is too long (max 80 characters)".into());
    }
    if end > 1440 || start >= end {
        return Err("The end time must be after the start time".into());
    }
    Ok(())
}

impl NewBlock {
    /// Checks the fields before they reach the database.
    pub fn validate(&self) -> Result<(), String> {
        check_block(&self.label, self.start, self.end)?;
        match (&self.weekday, &self.on_date) {
            (Some(day), None) if *day <= 6 => Ok(()),
            (None, Some(date)) if is_iso_date(date) => Ok(()),
            (None, Some(_)) => Err("Pick a valid date".into()),
            _ => Err("A block is either weekly (a weekday) or one-time (a date)".into()),
        }
    }
}

impl Schedule {
    /// The inputs for one weekday. One-time blocks count only when `date` is given
    /// (that is, when the day is really today).
    pub fn day(&self, name: &str, weekday: u8, date: Option<&str>) -> DayData {
        let mut blocks: Vec<ScheduleBlock> = self
            .blocks
            .iter()
            .filter(|b| {
                b.weekday == Some(weekday) || (date.is_some() && b.on_date.as_deref() == date)
            })
            .cloned()
            .collect();
        blocks.sort_by_key(|b| b.start);
        let divisions = self
            .splits
            .iter()
            .filter(|s| s.weekday == weekday)
            .map(|s| s.at)
            .collect();
        DayData {
            name: name.to_string(),
            date: date.unwrap_or_default().to_string(),
            wake: self.wake,
            sleep: self.sleep,
            blocks,
            divisions,
            now: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(title: &str) -> NewTask {
        NewTask {
            title: title.into(),
            notes: String::new(),
            priority: Priority::Medium,
            due_date: None,
            slice_id: None,
            start_time: None,
            duration_minutes: DEFAULT_DURATION,
        }
    }

    #[test]
    fn duration_must_be_between_one_minute_and_a_day() {
        let mut t = task("read");
        t.duration_minutes = 0;
        assert!(t.validate().is_err());
        t.duration_minutes = MAX_DURATION;
        assert!(t.validate().is_ok());
        t.duration_minutes = MAX_DURATION + 1;
        assert!(t.validate().is_err());
    }

    #[test]
    fn priority_codes_round_trip() {
        for p in [Priority::Low, Priority::Medium, Priority::High] {
            assert_eq!(Priority::from_code(p.code()), Some(p));
        }
        assert_eq!(Priority::from_code(0), None);
        assert_eq!(Priority::High.label(), "High");
    }

    #[test]
    fn blank_title_is_rejected() {
        assert!(task("   ").validate().is_err());
        assert!(task("write essay").validate().is_ok());
    }

    #[test]
    fn long_title_is_rejected() {
        assert!(task(&"x".repeat(201)).validate().is_err());
        assert!(task(&"x".repeat(200)).validate().is_ok());
    }

    #[test]
    fn date_and_time_are_checked() {
        let mut t = task("a");
        t.due_date = Some("2026-10-04".into());
        assert!(t.validate().is_ok());
        t.due_date = Some("10/04/2026".into());
        assert!(t.validate().is_err());
        t.due_date = None;
        t.start_time = Some(MAX_START_TIME + 1);
        assert!(t.validate().is_err());
    }

    fn block(
        id: i64,
        weekday: Option<u8>,
        date: Option<&str>,
        start: u32,
        end: u32,
    ) -> ScheduleBlock {
        ScheduleBlock {
            id,
            label: format!("block {id}"),
            weekday,
            on_date: date.map(String::from),
            start,
            end,
        }
    }

    #[test]
    fn block_validation_rules() {
        let ok = NewBlock {
            label: "Shift".into(),
            weekday: None,
            on_date: Some("2026-10-07".into()),
            start: 840,
            end: 1080,
        };
        assert!(ok.validate().is_ok());
        let weekly = NewBlock {
            weekday: Some(2),
            on_date: None,
            ..ok.clone()
        };
        assert!(weekly.validate().is_ok());
        assert!(NewBlock {
            label: "  ".into(),
            ..ok.clone()
        }
        .validate()
        .is_err());
        assert!(NewBlock {
            start: 900,
            end: 900,
            ..ok.clone()
        }
        .validate()
        .is_err());
        assert!(NewBlock {
            end: 1441,
            ..ok.clone()
        }
        .validate()
        .is_err());
        assert!(NewBlock {
            weekday: Some(1),
            ..ok.clone()
        }
        .validate()
        .is_err());
        assert!(NewBlock {
            on_date: Some("10/07/2026".into()),
            ..ok.clone()
        }
        .validate()
        .is_err());
        assert!(NewBlock {
            weekday: Some(7),
            on_date: None,
            ..ok
        }
        .validate()
        .is_err());
    }

    #[test]
    fn day_picks_weekly_blocks_and_only_todays_one_time_blocks() {
        let schedule = Schedule {
            wake: 480,
            sleep: 1320,
            blocks: vec![
                block(1, Some(0), None, 600, 650),
                block(2, Some(1), None, 660, 740),
                block(3, None, Some("2026-10-05"), 840, 1080),
                block(4, None, Some("2026-10-06"), 840, 1080),
            ],
            splits: vec![
                Split {
                    id: 1,
                    weekday: 0,
                    at: 780,
                },
                Split {
                    id: 2,
                    weekday: 1,
                    at: 900,
                },
            ],
        };
        let monday = schedule.day("Monday", 0, Some("2026-10-05"));
        let ids: Vec<i64> = monday.blocks.iter().map(|b| b.id).collect();
        assert_eq!(ids, vec![1, 3]);
        assert_eq!(monday.divisions, vec![780]);
        assert_eq!(monday.date, "2026-10-05");
        assert_eq!(schedule.day("Monday", 0, None).blocks.len(), 1);
    }

    #[test]
    fn weekday_names_map_to_numbers() {
        assert_eq!(weekday_index("Monday"), Some(0));
        assert_eq!(weekday_index("Sunday"), Some(6));
        assert_eq!(weekday_index("Funday"), None);
    }
}
