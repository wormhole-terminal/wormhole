//! Which of a folder's subfolders the list shows for what was typed, and where the typed text matched.

use std::ops::Range;

/// A subfolder in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    /// Where the typed text matched in `name` (bytes), to mark; none when nothing was typed.
    pub mark: Option<Range<usize>>,
}

/// The rows for `filter` out of `names`, which are in the order `ls` lists them. A name that begins with a
/// dot is left out unless `filter` begins with one. Without a filter that is all of them; with one, the names
/// that begin with it come first and then those that contain it, each group in the order it came in. The
/// match ignores case.
pub fn rows(names: &[String], filter: &str) -> Vec<Row> {
    let dots = filter.starts_with('.');
    let shown = names.iter().filter(|name| dots || !name.starts_with('.'));
    if filter.is_empty() {
        return shown.map(|name| Row { name: name.clone(), mark: None }).collect();
    }
    let (mut first, mut inside) = (Vec::new(), Vec::new());
    for name in shown {
        match find(name, filter) {
            Some(mark) if mark.start == 0 => first.push(Row { name: name.clone(), mark: Some(mark) }),
            Some(mark) => inside.push(Row { name: name.clone(), mark: Some(mark) }),
            None => {}
        }
    }
    first.extend(inside);
    first
}

/// The first place `needle` occurs in `hay`, ignoring case, as a byte range of `hay`.
fn find(hay: &str, needle: &str) -> Option<Range<usize>> {
    hay.char_indices().find_map(|(start, _)| {
        let len = matched_len(&hay[start..], needle)?;
        Some(start..start + len)
    })
}

/// How many bytes of `hay` the start of it takes up, if it begins with `needle` ignoring case.
fn matched_len(hay: &str, needle: &str) -> Option<usize> {
    let mut chars = hay.char_indices();
    let mut used = 0;
    for wanted in needle.chars() {
        let (at, c) = chars.next()?;
        if c != wanted && !c.to_lowercase().eq(wanted.to_lowercase()) {
            return None;
        }
        used = at + c.len_utf8();
    }
    Some(used)
}

#[cfg(test)]
mod tests {
    use super::rows;

    fn names(rows: &[super::Row]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    fn list(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn nothing_typed_shows_everything_but_dot_folders() {
        let all = list(&[".git", "docs", "src"]);
        let shown = rows(&all, "");
        assert_eq!(names(&shown), ["docs", "src"]);
        assert!(shown.iter().all(|r| r.mark.is_none()));
    }

    #[test]
    fn a_dot_shows_the_dot_folders() {
        let all = list(&[".cache", ".git", "docs", "a.b"]);
        assert_eq!(names(&rows(&all, ".")), [".cache", ".git", "a.b"], "prefix matches first");
        assert_eq!(names(&rows(&all, ".g")), [".git"]);
    }

    #[test]
    fn a_dot_folder_does_not_match_a_word_typed_without_one() {
        let all = list(&[".github", "github-docs"]);
        assert_eq!(names(&rows(&all, "git")), ["github-docs"]);
    }

    #[test]
    fn names_that_begin_with_it_come_first_then_those_that_contain_it() {
        let all = list(&["app-src", "docs", "source", "src", "xsrc"]);
        assert_eq!(names(&rows(&all, "src")), ["src", "app-src", "xsrc"]);
        assert_eq!(names(&rows(&all, "so")), ["source"]);
    }

    #[test]
    fn it_ignores_case_and_marks_the_match() {
        let all = list(&["Documents", "my-docs"]);
        let found = rows(&all, "DOC");
        assert_eq!(names(&found), ["Documents", "my-docs"]);
        assert_eq!(found[0].mark, Some(0..3));
        assert_eq!(found[1].mark, Some(3..6));
    }

    #[test]
    fn marks_are_byte_ranges_of_the_name_even_past_wide_characters() {
        let all = list(&["éa-docs"]);
        let found = rows(&all, "doc");
        assert_eq!(found[0].mark, Some(4..7));
        assert_eq!(&found[0].name[4..7], "doc");
    }

    #[test]
    fn nothing_matching_leaves_nothing() {
        assert!(rows(&list(&["a", "b"]), "zz").is_empty());
    }
}
