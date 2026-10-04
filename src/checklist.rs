/// Returns the number of checked and total checkbox markers in notes.
///
/// Markers follow the draft's `[ ]` / `[x]` syntax, allowing whitespace inside
/// the brackets. Any matching marker counts, including one with no label.
pub fn checklist_progress(notes: &str) -> Option<(usize, usize)> {
    let matches = checkbox_matches(notes);
    if matches.is_empty() {
        return None;
    }

    let checked = matches
        .iter()
        .filter(|(_, _, is_checked)| *is_checked)
        .count();
    Some((checked, matches.len()))
}

/// Toggles the zero-based checkbox marker at `index`, if one exists.
///
/// The selected marker is normalized to `[ ]` or `[x]`, matching the draft;
/// all other text and markers are preserved.
pub fn toggle_checkbox(notes: &str, index: usize) -> Option<String> {
    let matches = checkbox_matches(notes);
    let (start, end, is_checked) = *matches.get(index)?;

    let mut toggled = String::with_capacity(notes.len());
    toggled.push_str(&notes[..start]);
    toggled.push_str(if is_checked { "[ ]" } else { "[x]" });
    toggled.push_str(&notes[end..]);
    Some(toggled)
}

fn checkbox_matches(notes: &str) -> Vec<(usize, usize, bool)> {
    let mut matches = Vec::new();
    let mut search_from = 0;

    while let Some(relative_start) = notes[search_from..].find('[') {
        let start = search_from + relative_start;
        if let Some((end, is_checked)) = checkbox_end(notes, start) {
            matches.push((start, end, is_checked));
            search_from = end;
        } else {
            search_from = start + 1;
        }
    }

    matches
}

fn checkbox_end(notes: &str, start: usize) -> Option<(usize, bool)> {
    let mut chars = notes[start + 1..].char_indices().peekable();

    while let Some((_, ch)) = chars.peek() {
        if is_js_whitespace(*ch) {
            chars.next();
        } else {
            break;
        }
    }

    let mut is_checked = false;
    if let Some((_, ch)) = chars.peek() {
        if *ch == 'x' || *ch == 'X' {
            is_checked = true;
            chars.next();
        }
    }

    while let Some((_, ch)) = chars.peek() {
        if is_js_whitespace(*ch) {
            chars.next();
        } else {
            break;
        }
    }

    let (closing_offset, closing) = chars.next()?;
    (closing == ']').then_some((start + 1 + closing_offset + closing.len_utf8(), is_checked))
}

fn is_js_whitespace(ch: char) -> bool {
    ch.is_whitespace() || ch == '\u{FEFF}'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_all_done_checkboxes() {
        assert_eq!(checklist_progress("[x] One\n[X] Two"), Some((2, 2)));
    }

    #[test]
    fn counts_partial_checkboxes() {
        assert_eq!(
            checklist_progress("[x] One\n[ ] Two\n[ ] Three"),
            Some((1, 3))
        );
    }

    #[test]
    fn returns_none_without_checkboxes() {
        assert_eq!(checklist_progress("Just plain notes\nNo tasks here"), None);
    }

    #[test]
    fn counts_checked_checkbox_with_blank_label() {
        assert_eq!(checklist_progress("[x]"), Some((1, 1)));
    }

    #[test]
    fn counts_indented_checkboxes() {
        assert_eq!(
            checklist_progress("  [x] Nested\n\t[ ] Indented"),
            Some((1, 2))
        );
    }

    #[test]
    fn counts_checkboxes_among_plain_text_lines() {
        let notes = "Opening note\n[x] First task\nMore context\n[ ] Second task\nClosing note";
        assert_eq!(checklist_progress(notes), Some((1, 2)));
    }

    #[test]
    fn toggles_the_indexed_checkbox_and_preserves_other_text() {
        let notes = "Text [X] here\n  [  ] next";
        assert_eq!(
            toggle_checkbox(notes, 1),
            Some("Text [X] here\n  [x] next".to_owned())
        );
        assert_eq!(
            toggle_checkbox(notes, 0),
            Some("Text [ ] here\n  [  ] next".to_owned())
        );
    }

    #[test]
    fn returns_none_when_toggle_index_does_not_exist() {
        assert_eq!(toggle_checkbox("notes [ ]", 1), None);
        assert_eq!(toggle_checkbox("notes only", 0), None);
    }
}
