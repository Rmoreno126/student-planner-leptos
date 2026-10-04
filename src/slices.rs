//! Slice engine: wake/sleep window, minus blocked windows, split at division times.
//! Times are minutes since midnight (08:00 = 480). Overnight values run past 1440.

const DAY: u32 = 1440;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub label: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaySchedule {
    pub blocks: Vec<Block>,
    pub divisions: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceKind {
    Open,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slice {
    pub id: String,
    pub kind: SliceKind,
    pub label: String,
    pub start: u32,
    pub end: u32,
}

/// "13:30" -> Some(810)
pub fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let (h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// 780 -> "01:00 PM"
pub fn fmt_12h(mins: u32) -> String {
    let m = mins % DAY;
    let (h24, min) = (m / 60, m % 60);
    let ampm = if h24 < 12 { "AM" } else { "PM" };
    let h12 = if h24 % 12 == 0 { 12 } else { h24 % 12 };
    format!("{:02}:{:02} {}", h12, min, ampm)
}

pub fn compute_slices(wake: u32, sleep: u32, day: &DaySchedule) -> Vec<Slice> {
    let sleep = if sleep <= wake { sleep + DAY } else { sleep };

    // Normalize blocks (same rules as the JS draft), sorted by start.
    let mut blocks: Vec<(String, u32, u32)> = day
        .blocks
        .iter()
        .map(|b| {
            let start = if b.start < wake {
                b.start + DAY
            } else {
                b.start
            };
            let end = if b.end <= start { b.end + DAY } else { b.end };
            (b.label.clone(), start, end)
        })
        .collect();
    blocks.sort_by_key(|b| b.1);

    // Open gaps between blocks.
    let mut open: Vec<(u32, u32)> = Vec::new();
    let mut cur = wake;
    for (_, bs, be) in &blocks {
        if cur < *bs {
            open.push((cur, *bs));
        }
        cur = cur.max(*be);
    }
    if cur < sleep {
        open.push((cur, sleep));
    }

    // Split open gaps at division times.
    let mut divs: Vec<u32> = day
        .divisions
        .iter()
        .map(|&d| if d < wake { d + DAY } else { d })
        .collect();
    divs.sort();

    let mut split: Vec<(u32, u32)> = Vec::new();
    for (start, end) in open {
        let mut sub_start = start;
        for &d in &divs {
            if d > sub_start && d < end {
                split.push((sub_start, d));
                sub_start = d;
            }
        }
        if sub_start < end {
            split.push((sub_start, end));
        }
    }

    // Merge open + blocked, chronological, then label and number.
    let mut combined: Vec<(u32, u32, Option<String>)> = Vec::new();
    combined.extend(split.into_iter().map(|(s, e)| (s, e, None)));
    combined.extend(blocks.into_iter().map(|(l, s, e)| (s, e, Some(l))));
    combined.sort_by_key(|c| c.0);

    let mut n = 0;
    combined
        .into_iter()
        .map(|(start, end, blocked)| match blocked {
            Some(label) => Slice {
                id: format!("blocked_{start}"),
                kind: SliceKind::Blocked,
                label: format!("🚫 {} ({} - {})", label, fmt_12h(start), fmt_12h(end)),
                start,
                end,
            },
            None => {
                n += 1;
                Slice {
                    id: format!("slice_{n}"),
                    kind: SliceKind::Open,
                    label: format!("Slice {}: {} - {}", n, fmt_12h(start), fmt_12h(end)),
                    start,
                    end,
                }
            }
        })
        .collect()
}

/// Which blocked window (if any) contains this time?
pub fn blocked_at(slices: &[Slice], t: u32, wake: u32) -> Option<&Slice> {
    let t = if t < wake { t + DAY } else { t };
    slices
        .iter()
        .find(|s| s.kind == SliceKind::Blocked && t >= s.start && t < s.end)
}

/// Which open slice contains this time? Returns its id.
pub fn open_slice_id_at(slices: &[Slice], t: u32, wake: u32) -> Option<&str> {
    let t = if t < wake { t + DAY } else { t };
    slices
        .iter()
        .find(|s| s.kind == SliceKind::Open && t >= s.start && t < s.end)
        .map(|s| s.id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAKE: u32 = 8 * 60;
    const SLEEP: u32 = 22 * 60;

    fn block(label: &str, s: &str, e: &str) -> Block {
        Block {
            label: label.into(),
            start: parse_hhmm(s).unwrap(),
            end: parse_hhmm(e).unwrap(),
        }
    }

    fn monday() -> DaySchedule {
        DaySchedule {
            blocks: vec![
                block("Class", "10:00", "13:00"),
                block("Class", "16:00", "17:00"),
            ],
            divisions: vec![],
        }
    }

    #[test]
    fn formats_times() {
        assert_eq!(fmt_12h(0), "12:00 AM");
        assert_eq!(fmt_12h(720), "12:00 PM");
        assert_eq!(fmt_12h(780), "01:00 PM");
        assert_eq!(parse_hhmm("13:30"), Some(810));
        assert_eq!(parse_hhmm("25:00"), None);
    }

    #[test]
    fn empty_day_is_one_slice() {
        let s = compute_slices(WAKE, SLEEP, &DaySchedule::default());
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].label, "Slice 1: 08:00 AM - 10:00 PM");
    }

    #[test]
    fn monday_alternates_open_and_blocked() {
        let s = compute_slices(WAKE, SLEEP, &monday());
        let kinds: Vec<_> = s.iter().map(|x| x.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                SliceKind::Open,
                SliceKind::Blocked,
                SliceKind::Open,
                SliceKind::Blocked,
                SliceKind::Open
            ]
        );
        assert_eq!(s[0].label, "Slice 1: 08:00 AM - 10:00 AM");
        assert_eq!(s[1].label, "🚫 Class (10:00 AM - 01:00 PM)");
        assert_eq!(s[2].label, "Slice 2: 01:00 PM - 04:00 PM");
        assert_eq!(s[4].label, "Slice 3: 05:00 PM - 10:00 PM");
    }

    #[test]
    fn saturday_split_at_1pm() {
        let day = DaySchedule {
            blocks: vec![],
            divisions: vec![parse_hhmm("13:00").unwrap()],
        };
        let s = compute_slices(WAKE, SLEEP, &day);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].label, "Slice 1: 08:00 AM - 01:00 PM");
        assert_eq!(s[1].label, "Slice 2: 01:00 PM - 10:00 PM");
    }

    #[test]
    fn division_inside_a_block_is_ignored() {
        let mut day = monday();
        day.divisions = vec![parse_hhmm("11:00").unwrap()];
        assert_eq!(compute_slices(WAKE, SLEEP, &day).len(), 5);
    }

    #[test]
    fn overnight_bedtime_rolls_to_next_day() {
        let s = compute_slices(WAKE, 60, &DaySchedule::default());
        assert_eq!(s[0].label, "Slice 1: 08:00 AM - 01:00 AM");
    }

    #[test]
    fn finds_blocked_and_open_slices_for_a_time() {
        let s = compute_slices(WAKE, SLEEP, &monday());
        assert!(blocked_at(&s, parse_hhmm("11:00").unwrap(), WAKE).is_some());
        assert!(blocked_at(&s, parse_hhmm("14:00").unwrap(), WAKE).is_none());
        assert_eq!(
            open_slice_id_at(&s, parse_hhmm("14:00").unwrap(), WAKE),
            Some("slice_2")
        );
    }
}
