//! Google Calendar-style picture of a blocked window: hour lines down the side and a
//! gray striped event, so the user sees at a glance where the busy time ends.

use leptos::prelude::*;

use crate::slices::fmt_12h;

/// Pixels of height per minute of the block.
const PX_PER_MIN: f32 = 0.9;
/// The shortest the block is drawn, so short events stay readable.
const MIN_HEIGHT: f32 = 56.0;

/// Hour label: "11 AM", or "10:50 AM" when it isn't on the hour.
pub fn mark_label(minutes: u32) -> String {
    let full = fmt_12h(minutes);
    if minutes % 60 == 0 {
        full.replacen(":00", "", 1)
            .trim_start_matches('0')
            .to_string()
    } else {
        full
    }
}

/// Where a label is drawn: the start, then every full hour inside the block.
pub fn marks(start: u32, end: u32) -> Vec<u32> {
    let mut out = vec![start];
    let mut hour = (start / 60 + 1) * 60;
    while hour < end {
        out.push(hour);
        hour += 60;
    }
    out
}

/// Height in pixels for a block of this length.
pub fn block_height(start: u32, end: u32) -> f32 {
    (end.saturating_sub(start) as f32 * PX_PER_MIN).max(MIN_HEIGHT)
}

/// The calendar-style picture for a blocked window, plus a "free from" line after it.
#[component]
pub fn BlockedTimeline(start: u32, end: u32) -> impl IntoView {
    let height = block_height(start, end);
    let wrapper_style = format!("height:{height}px");
    let lines = marks(start, end)
        .into_iter()
        .map(|minute| {
            let top = ((minute - start) as f32 * PX_PER_MIN).min(height - 1.0);
            let line_style = format!("top:{top}px");
            view! { <div class="cal-hour" style=line_style><span>{mark_label(minute)}</span></div> }
        })
        .collect::<Vec<_>>();
    let range = format!("{} – {}", fmt_12h(start), fmt_12h(end));
    let free = format!("✅ Free from {}", mark_label(end));

    view! {
        <div class="cal-block" style=wrapper_style>
            {lines}
            <div class="cal-event">{range}</div>
        </div>
        <p class="cal-free">{free}</p>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hour_marks_start_with_the_block_and_follow_full_hours() {
        assert_eq!(marks(600, 780), vec![600, 660, 720]);
        assert_eq!(marks(600, 650), vec![600]);
        assert_eq!(marks(650, 780), vec![650, 660, 720]);
    }

    #[test]
    fn labels_drop_the_minutes_on_the_hour() {
        assert_eq!(mark_label(600), "10 AM");
        assert_eq!(mark_label(720), "12 PM");
        assert_eq!(mark_label(780), "1 PM");
        assert_eq!(mark_label(650), "10:50 AM");
    }

    #[test]
    fn short_blocks_keep_a_readable_height() {
        assert!((block_height(600, 780) - 162.0).abs() < 0.01);
        assert!((block_height(600, 610) - MIN_HEIGHT).abs() < 0.01);
    }
}
