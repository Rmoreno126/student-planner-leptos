//! Pure text logic for the notes editor. The browser handles typing; these functions
//! decide what the text should become, so they can be tested without a browser.
//! All offsets here are byte offsets; `utf16_to_byte` and `byte_to_utf16` convert to
//! and from the positions the browser reports.

use crate::checklist::checkbox_matches;

/// What Tab inserts.
pub const TAB: &str = "    ";

/// A piece of text for the highlight layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Ordinary text.
    Plain(String),
    /// The rest of a line after a ticked checkbox at its start (shown dimmed).
    Done(String),
    /// A checklist marker. `index` is its position among all markers in the text.
    Marker {
        index: usize,
        checked: bool,
        text: String,
    },
}

/// The result of an edit: the new text and where the caret goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub text: String,
    pub caret: usize,
}

fn at_line_start(text: &str, pos: usize) -> bool {
    text[..pos]
        .rsplit('\n')
        .next()
        .unwrap_or("")
        .chars()
        .all(|c| c == ' ' || c == '\t')
}

/// Splits `text` into the pieces the highlight layer paints.
pub fn segments(text: &str) -> Vec<Segment> {
    let found = checkbox_matches(text);
    let mut out = Vec::new();
    let mut cursor = 0;
    for (index, &(start, end, checked)) in found.iter().enumerate() {
        if start > cursor {
            out.push(Segment::Plain(text[cursor..start].to_string()));
        }
        out.push(Segment::Marker {
            index,
            checked,
            text: text[start..end].to_string(),
        });
        cursor = end;
        if checked && at_line_start(text, start) {
            let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
            let stop = found
                .get(index + 1)
                .map_or(line_end, |&(next, _, _)| next.min(line_end));
            if stop > end {
                out.push(Segment::Done(text[end..stop].to_string()));
                cursor = stop;
            }
        }
    }
    if cursor < text.len() {
        out.push(Segment::Plain(text[cursor..].to_string()));
    }
    out
}

/// Which checkbox (by index) is the caret strictly inside? Used to decide whether a
/// click should tick it. Clicking just before or after it only moves the caret.
pub fn marker_at_caret(text: &str, caret: usize) -> Option<usize> {
    checkbox_matches(text)
        .iter()
        .position(|&(start, end, _)| caret > start && caret < end)
}

/// Converts a browser caret position (UTF-16 units) into a byte offset in `text`.
pub fn utf16_to_byte(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (byte, ch) in text.char_indices() {
        if seen >= units {
            return byte;
        }
        seen += ch.len_utf16();
    }
    text.len()
}

/// Converts a byte offset in `text` into a browser caret position (UTF-16 units).
pub fn byte_to_utf16(text: &str, byte: usize) -> usize {
    text.get(..byte)
        .unwrap_or(text)
        .chars()
        .map(char::len_utf16)
        .sum()
}

/// Replaces `start..end` with `insert` and puts the caret after it.
pub fn insert_at(text: &str, start: usize, end: usize, insert: &str) -> Edit {
    let start = start.min(text.len());
    let end = end.clamp(start, text.len());
    let mut out = String::with_capacity(text.len() + insert.len());
    out.push_str(&text[..start]);
    out.push_str(insert);
    out.push_str(&text[end..]);
    Edit {
        text: out,
        caret: start + insert.len(),
    }
}

/// Tab: insert spaces over the selection and keep going.
pub fn tab(text: &str, start: usize, end: usize) -> Edit {
    insert_at(text, start, end, TAB)
}

/// Inserts `snippet` on its own line: if the caret is mid-line, a line break goes first.
pub fn insert_snippet(text: &str, caret: usize, snippet: &str) -> Edit {
    let caret = caret.min(text.len());
    let at_start = caret == 0 || text[..caret].ends_with('\n');
    let prefix = if at_start { "" } else { "\n" };
    insert_at(text, caret, caret, &format!("{prefix}{snippet}"))
}

/// Right after the user types a space following `[]`, turn it into an open checkbox `[ ]`.
pub fn normalize_marker(text: &str, caret: usize) -> Option<Edit> {
    let head = text.get(..caret)?;
    if !head.ends_with("[] ") {
        return None;
    }
    Some(insert_at(text, caret - 3, caret, "[ ] "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checklist::toggle_checkbox;

    #[test]
    fn segments_split_markers_and_dim_done_rows() {
        let parts = segments("[x] done thing\nnext [ ] later");
        assert_eq!(
            parts,
            vec![
                Segment::Marker {
                    index: 0,
                    checked: true,
                    text: "[x]".into()
                },
                Segment::Done(" done thing".into()),
                Segment::Plain("\nnext ".into()),
                Segment::Marker {
                    index: 1,
                    checked: false,
                    text: "[ ]".into()
                },
                Segment::Plain(" later".into()),
            ]
        );
    }

    #[test]
    fn marker_hit_test_needs_the_caret_inside_the_brackets() {
        let text = "a [ ] b";
        assert_eq!(marker_at_caret(text, 2), None);
        assert_eq!(marker_at_caret(text, 3), Some(0));
        assert_eq!(marker_at_caret(text, 4), Some(0));
        assert_eq!(marker_at_caret(text, 5), None);
    }

    #[test]
    fn clicking_a_marker_ticks_the_same_one_the_task_card_would() {
        let text = "x [] y [x] z";
        let index = marker_at_caret(text, 3).expect("caret is inside the first marker");
        assert_eq!(index, 0);
        assert_eq!(
            toggle_checkbox(text, index).as_deref(),
            Some("x [x] y [x] z")
        );
    }

    #[test]
    fn utf16_and_byte_offsets_agree_even_with_emoji() {
        let text = "a😀[ ]";
        assert_eq!(utf16_to_byte(text, 3), 5);
        assert_eq!(byte_to_utf16(text, 5), 3);
        assert_eq!(utf16_to_byte(text, 2), 5);
        assert_eq!(utf16_to_byte(text, 99), text.len());
    }

    #[test]
    fn tab_inserts_spaces_over_a_selection() {
        let edit = tab("hello world", 5, 11);
        assert_eq!(
            edit,
            Edit {
                text: "hello    ".into(),
                caret: 9
            }
        );
    }

    #[test]
    fn snippets_start_a_new_line_only_when_mid_line() {
        assert_eq!(
            insert_snippet("some words", 10, "[ ] "),
            Edit {
                text: "some words\n[ ] ".into(),
                caret: 15
            }
        );
        assert_eq!(
            insert_snippet("a\n", 2, "- "),
            Edit {
                text: "a\n- ".into(),
                caret: 4
            }
        );
        assert_eq!(
            insert_snippet("", 0, "# "),
            Edit {
                text: "# ".into(),
                caret: 2
            }
        );
    }

    #[test]
    fn space_after_empty_brackets_becomes_an_open_checkbox() {
        assert_eq!(
            normalize_marker("todo [] ", 8),
            Some(Edit {
                text: "todo [ ] ".into(),
                caret: 9
            })
        );
        assert_eq!(normalize_marker("[]", 2), None);
        assert_eq!(normalize_marker("a [] b", 6), None);
    }
}
