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
}

/// Partial update. `None` fields are left unchanged; a blank title is ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub priority: Option<Priority>,
}

/// Largest start time allowed (47:59, so bedtimes past midnight fit).
pub const MAX_START_TIME: u32 = 2879;

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
        }
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
}
