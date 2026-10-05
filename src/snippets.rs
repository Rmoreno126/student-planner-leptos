//! The snippet buttons above the notes editor. The user picks and orders up to
//! [`MAX_BUTTONS`] of them, and the choice is saved in the database.

/// One insertable snippet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snippet {
    /// Stable id used when saving the toolbar.
    pub id: &'static str,
    /// Text shown on the button.
    pub label: &'static str,
    /// What the button inserts, always on a new line.
    pub text: &'static str,
}

/// Most buttons the toolbar can hold.
pub const MAX_BUTTONS: usize = 7;

/// Every snippet the user can choose from.
pub static CATALOG: [Snippet; 11] = [
    Snippet {
        id: "task",
        label: "☑ Task",
        text: "[ ] ",
    },
    Snippet {
        id: "list",
        label: "• List",
        text: "- ",
    },
    Snippet {
        id: "number",
        label: "1. Number",
        text: "1. ",
    },
    Snippet {
        id: "h1",
        label: "H1",
        text: "# ",
    },
    Snippet {
        id: "h2",
        label: "H2",
        text: "## ",
    },
    Snippet {
        id: "h3",
        label: "H3",
        text: "### ",
    },
    Snippet {
        id: "bold",
        label: "Bold",
        text: "**bold**",
    },
    Snippet {
        id: "italic",
        label: "Italic",
        text: "*italic*",
    },
    Snippet {
        id: "quote",
        label: "Quote",
        text: "> ",
    },
    Snippet {
        id: "code",
        label: "Code",
        text: "`code`",
    },
    Snippet {
        id: "divider",
        label: "Divider",
        text: "---",
    },
];

/// The toolbar a new user starts with.
pub const DEFAULT_TOOLBAR: [&str; 5] = ["task", "list", "number", "h3", "bold"];

/// Finds a snippet by id.
pub fn find(id: &str) -> Option<&'static Snippet> {
    CATALOG.iter().find(|s| s.id == id)
}

/// Keeps known ids, drops repeats, caps at [`MAX_BUTTONS`], and falls back to the
/// default toolbar when nothing is left.
pub fn sanitize(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        if find(id).is_some() && !out.contains(id) && out.len() < MAX_BUTTONS {
            out.push(id.clone());
        }
    }
    if out.is_empty() {
        DEFAULT_TOOLBAR.iter().map(|s| s.to_string()).collect()
    } else {
        out
    }
}

/// Splits a saved comma-separated toolbar into ids.
pub fn parse(csv: &str) -> Vec<String> {
    csv.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Joins ids into the comma-separated form that gets saved.
pub fn join(ids: &[String]) -> String {
    ids.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn default_toolbar_ids_all_exist() {
        for id in DEFAULT_TOOLBAR {
            assert!(find(id).is_some(), "{id} is not in the catalog");
        }
    }

    #[test]
    fn catalog_ids_are_unique() {
        let mut all: Vec<&str> = CATALOG.iter().map(|s| s.id).collect();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), CATALOG.len());
    }

    #[test]
    fn sanitize_drops_unknown_and_repeats_and_caps() {
        let many = ids(&[
            "task", "task", "nope", "list", "number", "h1", "h2", "h3", "bold", "italic",
        ]);
        assert_eq!(
            sanitize(&many),
            ids(&["task", "list", "number", "h1", "h2", "h3", "bold"])
        );
    }

    #[test]
    fn empty_choice_falls_back_to_default() {
        assert_eq!(sanitize(&[]), ids(&DEFAULT_TOOLBAR));
        assert_eq!(sanitize(&ids(&["nope"])), ids(&DEFAULT_TOOLBAR));
    }

    #[test]
    fn parse_and_join_round_trip() {
        let list = ids(&["task", "bold"]);
        assert_eq!(parse(&join(&list)), list);
        assert!(parse("").is_empty());
    }
}
