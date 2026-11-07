//! The words in a hook's `title`, `body`, `text`, `tooltip` and `path`: fixed text with `{field}` for a
//! field of the event and `{n}` for capture `n` of its pattern (`{0}` is the whole match). Nothing in a
//! template is an escape and nothing is evaluated: a template names what goes in, never what to do.

/// One piece of a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    /// A field of the event, by name.
    Field(String),
    /// A capture of the hook's pattern.
    Capture(usize),
}

/// The longest field name in a template.
const MAX_NAME: usize = 24;

/// `template` as its pieces. A `{` that does not open a placeholder and a `}` that does not close one
/// are errors.
pub fn parse_template(template: &str) -> Result<Vec<Piece>, String> {
    let mut pieces = Vec::new();
    let mut rest = template;
    while let Some(at) = rest.find(['{', '}']) {
        if rest.as_bytes()[at] == b'}' {
            return Err("has a `}` that closes no `{field}`".into());
        }
        if at > 0 {
            pieces.push(Piece::Text(rest[..at].to_string()));
        }
        let after = &rest[at + 1..];
        let close = after.find('}').ok_or("has a `{` that is never closed")?;
        let name = &after[..close];
        pieces.push(placeholder(name)?);
        rest = &after[close + 1..];
    }
    if !rest.is_empty() {
        pieces.push(Piece::Text(rest.to_string()));
    }
    Ok(pieces)
}

fn placeholder(name: &str) -> Result<Piece, String> {
    if !name.is_empty() && name.len() <= 2 && name.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(Piece::Capture(name.parse().map_err(|_| "has a capture that is not a number")?));
    }
    let mut chars = name.chars();
    let ok = name.len() <= MAX_NAME
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if ok {
        Ok(Piece::Field(name.to_string()))
    } else {
        Err(format!(
            "has `{{{name}}}`, which is neither a field name like `{{took_s}}` nor a capture like `{{1}}`"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Piece {
        Piece::Text(s.into())
    }
    fn field(s: &str) -> Piece {
        Piece::Field(s.into())
    }

    #[test]
    fn a_template_is_words_fields_and_captures() {
        assert_eq!(
            parse_template("{took_s} s in {folder_name} ({1}/{0})"),
            Ok(vec![
                field("took_s"),
                text(" s in "),
                field("folder_name"),
                text(" ("),
                Piece::Capture(1),
                text("/"),
                Piece::Capture(0),
                text(")")
            ])
        );
        assert_eq!(parse_template(""), Ok(vec![]));
        assert_eq!(parse_template("plain"), Ok(vec![text("plain")]));
    }

    #[test]
    fn a_brace_that_is_not_a_placeholder_is_an_error() {
        for bad in [
            "{",
            "}",
            "a } b",
            "{ x}",
            "{X}",
            "{1x}",
            "{}",
            "{{x}}",
            "{123}",
            "{a-b}",
            "{x",
            &format!("{{{}}}", "a".repeat(25)),
        ] {
            assert!(parse_template(bad).is_err(), "{bad:?}");
        }
    }
}
