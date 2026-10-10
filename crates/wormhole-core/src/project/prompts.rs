//! `prompts/*.md`: reusable prompts for the agents.

use std::fs;
use std::io;
use std::path::Path;

use super::{MAX_PROMPT_BYTES, MAX_PROMPT_DIR_ENTRIES, PROMPTS_DIR, ProjectError, ProjectFolder};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// File stem, e.g. `review` for `prompts/review.md`.
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    /// Text to paste, without front matter or the title heading.
    pub body: String,
}

/// Load all `*.md` files (non-recursive) sorted by id. `Ok(None)` if the directory is absent.
/// `before_read` sees each path (the directory, then every file) just before it is read.
pub(super) fn load_dir(
    folder: &ProjectFolder,
    before_read: &mut dyn FnMut(&Path),
) -> Result<Option<Vec<Prompt>>, ProjectError> {
    let dir = folder.path(PROMPTS_DIR);
    before_read(&dir);
    #[cfg(test)]
    super::tests::count_read();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ProjectError::io(&dir, e)),
    };
    let mut prompts = Vec::new();
    for (count, entry) in entries.enumerate() {
        if count == MAX_PROMPT_DIR_ENTRIES {
            let message = format!("has more than {MAX_PROMPT_DIR_ENTRIES} entries");
            return Err(ProjectError::invalid(&dir, message));
        }
        let path = entry.map_err(|e| ProjectError::io(&dir, e))?.path();
        let is_md = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        // `is_file` only stats; whether a link may be followed is the read's decision.
        if !is_md || !path.is_file() {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        before_read(&path);
        // Deleted between listing and reading: just skip it.
        let Some(text) = folder.read_text(&path, MAX_PROMPT_BYTES)? else { continue };
        prompts.push(parse_prompt(id, &text));
    }
    prompts.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Some(prompts))
}

/// Title precedence: front matter `title:`, then a leading `# Heading`, then the id.
pub(crate) fn parse_prompt(id: &str, text: &str) -> Prompt {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let (meta, rest) = split_front_matter(text);
    let field = |key: &str| {
        meta.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
            .filter(|v| !v.is_empty())
    };
    let mut title = field("title");
    let mut body = rest.trim_start_matches(['\n', '\r']);
    if let Some(first) = body.lines().next()
        && let Some(heading) = first.strip_prefix("# ")
    {
        // Remove the heading from the body even when front matter already gave a title: it's a
        // title, not part of the instructions for the agent.
        title.get_or_insert_with(|| heading.trim().to_string());
        body = body[first.len()..].trim_start_matches(['\n', '\r']);
    }
    Prompt {
        id: id.to_string(),
        title: title.unwrap_or_else(|| id.to_string()),
        description: field("description"),
        body: body.trim_end().to_string(),
    }
}

/// Split a `---\nkey: value\n---` block off the top. Only flat `key: value` pairs are understood
/// (no YAML dependency); unknown keys are ignored. Without a closing `---` there is no front matter.
fn split_front_matter(text: &str) -> (Vec<(String, String)>, &str) {
    let Some(after) = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) else {
        return (Vec::new(), text);
    };
    let mut offset = 0;
    let mut meta = Vec::new();
    for line in after.split_inclusive('\n') {
        offset += line.len();
        let trimmed = line.trim();
        if trimmed == "---" {
            return (meta, &after[offset..]);
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let value = value.trim();
            let value = unquote(value, '"').or_else(|| unquote(value, '\'')).unwrap_or(value);
            meta.push((key.trim().to_string(), value.to_string()));
        }
    }
    (Vec::new(), text)
}

fn unquote(value: &str, quote: char) -> Option<&str> {
    value.strip_prefix(quote)?.strip_suffix(quote)
}

#[cfg(test)]
mod tests {
    use super::super::tests::project;
    use super::*;

    #[test]
    fn titles() {
        let p =
            parse_prompt("x", "---\ntitle: \"Review PR\"\ndescription: Checks diff\n---\n\nLook at it.\n");
        assert_eq!(p.title, "Review PR");
        assert_eq!(p.description.as_deref(), Some("Checks diff"));
        assert_eq!(p.body, "Look at it.");

        let p = parse_prompt("x", "# Fix tests\n\nRun them.\n");
        assert_eq!((p.title.as_str(), p.body.as_str()), ("Fix tests", "Run them."));

        let p = parse_prompt("plain", "Just text\n# not a title\n");
        assert_eq!(p.title, "plain");
        assert_eq!(p.body, "Just text\n# not a title");

        // Front matter wins over heading; heading is still removed from the body.
        let p = parse_prompt("x", "---\ntitle: A\n---\n# B\nbody");
        assert_eq!((p.title.as_str(), p.body.as_str()), ("A", "body"));

        // Unclosed front matter is ordinary text.
        let p = parse_prompt("x", "---\ntitle: A\nbody");
        assert_eq!(p.title, "x");
        assert!(p.body.starts_with("---"));

        let p = parse_prompt("x", "\u{feff}---\r\ntitle: Win\r\n---\r\nbody\r\n");
        assert_eq!((p.title.as_str(), p.body.as_str()), ("Win", "body"));
    }

    #[test]
    fn loads_sorted_md_only() {
        let (_tmp, p) = project(&[
            ("prompts/zeta.md", "# Zeta\nz"),
            ("prompts/alpha.md", "a"),
            ("prompts/notes.txt", "ignored"),
            ("prompts/sub/nested.md", "ignored"),
        ]);
        let prompts = p.prompts().expect("ok").expect("present");
        let ids: Vec<_> = prompts.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["alpha", "zeta"]);
        assert_eq!(prompts[1].title, "Zeta");
    }

    #[test]
    fn non_utf8_is_an_error_with_path() {
        let (_tmp, p) = project(&[]);
        let dir = p.dir.join("prompts");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("bad.md"), [0xff, 0xfe, 0x00]).expect("write");
        let err = p.prompts().expect_err("invalid utf-8");
        assert!(err.path().ends_with("bad.md"));
    }
}
