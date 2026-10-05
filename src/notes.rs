//! Notes helpers: split notes into text and checklist rows, and render markdown safely.
//!
//! Checklist markers are counted the same way as `checklist.rs`: every `[ ]` or `[x]`
//! in the notes counts, in order, so a row's `index` is what `toggle_checkbox` expects.

use pulldown_cmark::{html, Event, Parser, Tag, TagEnd};

/// One piece of a task's notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteBlock {
    /// Plain markdown text.
    Text(String),
    /// A checklist row. `index` counts every marker in the notes, starting at 0.
    Check {
        index: usize,
        checked: bool,
        label: String,
    },
}

/// Finds every checklist marker in `line` as `(start, end, checked)` byte offsets.
fn markers(line: &str) -> Vec<(usize, usize, bool)> {
    let bytes = line.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let mut checked = false;
            if j < bytes.len() && (bytes[j] == b'x' || bytes[j] == b'X') {
                checked = true;
                j += 1;
            }
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b']' {
                found.push((i, j + 1, checked));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    found
}

/// Splits notes into markdown text and checklist rows.
pub fn split_notes(notes: &str) -> Vec<NoteBlock> {
    let mut blocks = Vec::new();
    let mut text: Vec<&str> = Vec::new();
    let mut next_index = 0usize;

    for line in notes.lines() {
        let found = markers(line);
        let body = line.trim_start();
        let body = body
            .strip_prefix("- ")
            .or_else(|| body.strip_prefix("* "))
            .unwrap_or(body)
            .trim_start();
        let offset = line.len() - body.len();

        match found.first() {
            Some(&(start, end, checked)) if start == offset => {
                if !text.is_empty() {
                    blocks.push(NoteBlock::Text(text.join("\n")));
                    text.clear();
                }
                blocks.push(NoteBlock::Check {
                    index: next_index,
                    checked,
                    label: line[end..].trim().to_string(),
                });
            }
            _ => text.push(line),
        }
        next_index += found.len();
    }
    if !text.is_empty() {
        blocks.push(NoteBlock::Text(text.join("\n")));
    }
    blocks
}

/// Renders markdown to HTML. Raw HTML is shown as text and links and images are
/// reduced to their text, so notes can never inject markup or scripts.
pub fn render_markdown(text: &str) -> String {
    let events = Parser::new(text)
        .map(|event| match event {
            Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
            other => other,
        })
        .filter(|event| {
            !matches!(
                event,
                Event::Start(Tag::Link { .. })
                    | Event::End(TagEnd::Link)
                    | Event::Start(Tag::Image { .. })
                    | Event::End(TagEnd::Image)
            )
        });
    let mut out = String::new();
    html::push_html(&mut out, events);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checklist::{checklist_progress, toggle_checkbox};

    #[test]
    fn splits_text_and_checklist_rows() {
        let blocks = split_notes("Plan:\n[ ] read\n- [x] write\nnote");
        assert_eq!(
            blocks,
            vec![
                NoteBlock::Text("Plan:".into()),
                NoteBlock::Check {
                    index: 0,
                    checked: false,
                    label: "read".into()
                },
                NoteBlock::Check {
                    index: 1,
                    checked: true,
                    label: "write".into()
                },
                NoteBlock::Text("note".into()),
            ]
        );
    }

    #[test]
    fn markers_inside_text_still_advance_the_index() {
        let notes = "[ ] a\n[x] b\nsee [ ] here\n[ ] d";
        let last = split_notes(notes).last().cloned();
        assert_eq!(
            last,
            Some(NoteBlock::Check {
                index: 3,
                checked: false,
                label: "d".into()
            })
        );
        // Index 3 must be the same marker that toggle_checkbox flips.
        assert_eq!(
            toggle_checkbox(notes, 3).as_deref(),
            Some("[ ] a\n[x] b\nsee [ ] here\n[x] d")
        );
        assert_eq!(checklist_progress(notes), Some((1, 4)));
    }

    #[test]
    fn markdown_is_rendered_but_html_and_links_are_neutralised() {
        let out = render_markdown("**bold** <script>alert(1)</script> [x](javascript:alert(1))");
        assert!(out.contains("<strong>bold</strong>"));
        assert!(!out.contains("<script>"));
        assert!(!out.contains("href"));
    }
}
