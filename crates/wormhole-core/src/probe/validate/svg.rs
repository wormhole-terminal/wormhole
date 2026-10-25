//! An agent's icon: a small one-colour SVG, drawn by the app in the theme's colour.
//!
//! The file is checked as text, conservatively: it must be made of drawing elements only, with no
//! reference to anything outside itself and nothing that runs. The app's renderer is what finally
//! draws it; this keeps what reaches the renderer inside what was reviewed.
//!
//! What is accepted: `<svg>`, `<g>`, `<path>`, `<circle>`, `<rect>`, `<ellipse>`, `<line>`,
//! `<polyline>`, `<polygon>`, `<title>` and `<desc>`, with any attributes except `style`, an event
//! handler (`on…`) or a link (`href`, `xlink:href`), plus comments and the XML declaration (`<?xml …?>`,
//! first only). What is
//! refused, by name: `<script>`, `<image>`, `<style>`, `<foreignObject>`, `<use>` and every other
//! element, a DOCTYPE or entity, CDATA, and anything it cannot read as tags and attributes.

/// An icon's size, at most.
pub(crate) const MAX_BYTES: usize = 8 * 1024;

const ALLOWED: &[&str] =
    &["svg", "g", "path", "circle", "rect", "ellipse", "line", "polyline", "polygon", "title", "desc"];

/// Why `text` is not an acceptable icon, if it is not one.
pub(crate) fn problem(text: &str) -> Option<String> {
    // The XML declaration is the one `<?` accepted, and only first; `<?xml-stylesheet …?>` and any other
    // processing instruction are not.
    let mut rest = text.trim_start_matches('\u{feff}').trim_start();
    if let Some(after) = rest.strip_prefix("<?xml").filter(|after| after.starts_with(char::is_whitespace)) {
        let Some(end) = after.find("?>") else { return Some("has an unterminated `<?xml`".into()) };
        rest = &after[end + 2..];
    }
    let mut saw_svg = false;
    while let Some(at) = rest.find('<') {
        rest = &rest[at + 1..];
        if let Some(after) = rest.strip_prefix("!--") {
            let Some(end) = after.find("-->") else { return Some("has an unterminated comment".into()) };
            rest = &after[end + 3..];
        } else if rest.starts_with('?') {
            return Some(
                "has a processing instruction (`<?`) other than the XML declaration at the start".into(),
            );
        } else if rest.starts_with('!') {
            return Some("has a DOCTYPE, entity or CDATA, which are not allowed".into());
        } else if let Some(after) = rest.strip_prefix('/') {
            let Some(end) = after.find('>') else { return Some("has an unterminated closing tag".into()) };
            if let Some(why) = element_problem(after[..end].trim()) {
                return Some(why);
            }
            rest = &after[end + 1..];
        } else {
            let name_end =
                rest.find(|c: char| c.is_whitespace() || c == '/' || c == '>').unwrap_or(rest.len());
            let name = &rest[..name_end];
            if let Some(why) = element_problem(name) {
                return Some(why);
            }
            saw_svg |= name == "svg";
            match attributes(&rest[name_end..]) {
                Ok(after) => rest = after,
                Err(why) => return Some(why),
            }
        }
    }
    (!saw_svg).then(|| "is not an SVG: there is no `<svg>` element".into())
}

fn element_problem(name: &str) -> Option<String> {
    if ALLOWED.contains(&name) {
        return None;
    }
    Some(match name.to_ascii_lowercase().as_str() {
        "script" | "image" | "style" | "foreignobject" | "use" => {
            format!("has a `<{name}>`, which is not allowed in an icon")
        }
        _ => format!("has a `<{name}>`; an icon is made of paths and basic shapes only"),
    })
}

/// Read attributes up to the end of the tag; what follows the tag.
fn attributes(mut rest: &str) -> Result<&str, String> {
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("/>").or_else(|| rest.strip_prefix('>')) {
            return Ok(after);
        }
        let name_end = rest
            .find(|c: char| c == '=' || c == '>' || c == '/' || c.is_whitespace())
            .ok_or("has a tag that does not end")?;
        let name = rest[..name_end].to_ascii_lowercase();
        if name.is_empty() {
            return Err("has a tag it cannot read".into());
        }
        if name == "style" || name == "href" || name == "xlink:href" || name.starts_with("on") {
            return Err(format!("has a `{name}` attribute, which is not allowed in an icon"));
        }
        rest = rest[name_end..].trim_start();
        // A bare attribute has no value; every other must be quoted.
        let Some(after_eq) = rest.strip_prefix('=') else { continue };
        let value = after_eq.trim_start();
        let quote =
            value.chars().next().filter(|q| matches!(q, '"' | '\'')).ok_or("has an unquoted attribute")?;
        let end = value[1..].find(quote).ok_or("has an attribute value that does not end")?;
        // `url(...)` can point at another resource in a paint attribute.
        if value[1..1 + end].to_ascii_lowercase().contains("url(") {
            return Err(format!("has a `{name}` that refers to something with `url(`, which is not allowed"));
        }
        rest = &value[end + 2..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"<?xml version="1.0"?><!-- a mark --><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><title>Acme</title><g fill="none"><path d="M0 0h24v24H0z"/><circle cx="1" cy="1" r="1"></circle></g></svg>"#;

    #[test]
    fn a_plain_icon_is_accepted() {
        assert_eq!(problem(OK), None);
        assert_eq!(problem("<svg><path d='M0 0'/></svg>"), None, "single quotes");
        assert_eq!(problem("<svg ><path d=\"M0 0\" /></svg >"), None);
    }

    #[test]
    fn what_runs_or_reaches_out_is_refused_by_name() {
        for (bad, word) in [
            ("<svg><script>alert(1)</script></svg>", "script"),
            ("<svg><image href='x.png'/></svg>", "image"),
            ("<svg><style>*{}</style></svg>", "style"),
            ("<svg><foreignObject/></svg>", "foreignObject"),
            ("<svg><use href='#a'/></svg>", "use"),
            ("<svg><SCRIPT/></svg>", "SCRIPT"),
            ("<svg><path d='M0 0' style='fill:red'/></svg>", "style"),
            ("<svg onload='x()'><path d='M0 0'/></svg>", "onload"),
            ("<svg><a href='https://x'><path d='M0 0'/></a></svg>", "<a>"),
            ("<svg><path d='M0 0' fill='url(https://x/y#z)'/></svg>", "url("),
            ("<svg><path d='M0 0' xlink:href='x'/></svg>", "xlink:href"),
            ("<!DOCTYPE svg [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><svg/>", "DOCTYPE"),
            ("<?xml-stylesheet href='x.css'?><svg><path d='M0 0'/></svg>", "processing instruction"),
            ("<?xml version='1.0'?><?xml-stylesheet href='x.css'?><svg/>", "processing instruction"),
            ("<svg><?php echo 1 ?></svg>", "processing instruction"),
            ("<svg/><?xml version='1.0'?>", "processing instruction"),
            ("<svg><![CDATA[x]]></svg>", "CDATA"),
        ] {
            let why = problem(bad).unwrap_or_else(|| panic!("accepted {bad}"));
            assert!(why.contains(word), "{bad}: {why}");
        }
    }

    #[test]
    fn the_xml_declaration_is_fine_first_with_a_byte_order_mark_or_white_space() {
        let svg = "<svg><path d='M0 0'/></svg>";
        for head in [
            "<?xml version='1.0'?>",
            "\u{feff}<?xml version='1.0'?>",
            "\n  <?xml version='1.0' encoding='UTF-8'?>\n",
        ] {
            assert_eq!(problem(&format!("{head}{svg}")), None, "{head:?}");
        }
        assert!(problem(&format!("<?xml{svg}")).is_some(), "`<?xml` must be followed by white space");
    }

    #[test]
    fn what_it_cannot_read_is_refused() {
        for bad in
            ["", "plain text", "<svg", "<svg a=1/>", "<svg a='1/>", "<svg><!-- x</svg>", "<g/>", "<svg></svg"]
        {
            assert!(problem(bad).is_some(), "{bad:?}");
        }
    }

    #[test]
    fn a_greater_than_inside_a_quoted_value_does_not_end_the_tag() {
        assert_eq!(problem("<svg a='>'><path d='M0 0'/></svg>"), None);
        assert!(problem("<svg a='>'><script/></svg>").is_some());
    }
}
