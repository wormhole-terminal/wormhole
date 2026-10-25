//! The little templates in a rule: a hover text and an `open-url` link, with `{n}` for capture `n`
//! (`{0}` is the whole match).

/// The numbers of the `{n}` a template holds, in order. A `{` that does not open a `{n}` and a `}`
/// that does not close one are errors: nothing in a template is an escape.
pub(crate) fn placeholders(template: &str) -> Result<Vec<usize>, String> {
    let mut found = Vec::new();
    let mut rest = template;
    while let Some(at) = rest.find(['{', '}']) {
        if rest.as_bytes()[at] == b'}' {
            return Err("has a `}` that closes no `{n}`".into());
        }
        let after = &rest[at + 1..];
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || digits > 2 || after.as_bytes().get(digits) != Some(&b'}') {
            return Err("has a `{` that is not a placeholder like `{1}`".into());
        }
        found.push(after[..digits].parse().map_err(|_| "has a placeholder that is not a number")?);
        rest = &after[digits + 1..];
    }
    Ok(found)
}

/// Why a link template cannot be accepted, or its host.
pub(crate) fn https_host(url: &str) -> Result<&str, &'static str> {
    let Some(after) = url.strip_prefix("https://") else {
        return Err("must start with `https://`");
    };
    let end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let host = &after[..end];
    if host.contains('{') || host.contains('}') {
        return Err(
            "has a capture in the host position, which would let a program's output choose where it goes",
        );
    }
    if host.is_empty() {
        return Err("has no host");
    }
    // Each of these can make the text before it look like the host: `user@evil` or `host:port@evil`.
    if !host.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-') {
        return Err(
            "has a host with characters other than a-z, 0-9, `.` and `-` (no user, port or capital letters)",
        );
    }
    if host.starts_with(['.', '-']) || host.ends_with(['.', '-']) || host.contains("..") {
        return Err("has a malformed host");
    }
    Ok(host)
}

/// Whether the path of the `https` link `url` (between the host and the first `?` or `#`) has a segment
/// that is `.` or `..`, however it is spelt: a browser (WHATWG URL) reads `%2e` in any case as a dot,
/// and `\\` as a `/`. Such a segment walks the path up, so `https://host/browse/{1}` with a capture of
/// `..` would open `https://host/` and `{1}{2}` with two `.` does the same. A `{n}` in a template is not
/// a dot, so a template is judged by what it spells itself.
pub(crate) fn has_dot_segment(url: &str) -> bool {
    let after = url.strip_prefix("https://").unwrap_or(url);
    let Some(path_at) = after.find(['/', '\\']) else { return false };
    let path = &after[path_at..];
    let path = &path[..path.find(['?', '#']).unwrap_or(path.len())];
    path.split(['/', '\\']).any(|segment| {
        let mut plain = String::with_capacity(segment.len());
        let mut rest = segment;
        while let Some(at) = rest.find('%') {
            plain.push_str(&rest[..at]);
            let after = &rest[at + 1..];
            if after.len() >= 2 && after.is_char_boundary(2) && after[..2].eq_ignore_ascii_case("2e") {
                plain.push('.');
                rest = &after[2..];
            } else {
                plain.push('%');
                rest = after;
            }
        }
        plain.push_str(rest);
        plain == "." || plain == ".."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_numbers_in_braces() {
        assert_eq!(placeholders("Open {1} in Jira"), Ok(vec![1]));
        assert_eq!(placeholders("{0} {2}{10}"), Ok(vec![0, 2, 10]));
        assert_eq!(placeholders("plain"), Ok(vec![]));
    }

    #[test]
    fn anything_else_in_braces_is_an_error() {
        for bad in ["{", "}", "{}", "{a}", "{1", "{{1}}", "a } b", "{123}", "{-1}", "{ 1}"] {
            assert!(placeholders(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_host_is_read_up_to_the_first_slash_query_or_fragment() {
        assert_eq!(https_host("https://acme.atlassian.net/browse/{1}"), Ok("acme.atlassian.net"));
        assert_eq!(https_host("https://acme.atlassian.net?q={1}"), Ok("acme.atlassian.net"));
        assert_eq!(https_host("https://acme.atlassian.net#{1}"), Ok("acme.atlassian.net"));
        assert_eq!(https_host("https://a-b.example"), Ok("a-b.example"));
    }

    #[test]
    fn a_capture_user_port_other_scheme_or_odd_host_is_refused() {
        for bad in [
            "http://acme.net/",
            "ftp://acme.net/",
            "//acme.net/",
            "https://{1}.acme.net/",
            "https://acme.net{1}/",
            "https://acme.net@evil.example/",
            "https://acme.net:8443/",
            "https://Acme.net/",
            "https:///path",
            "https://.acme.net/",
            "https://acme..net/",
            "https://acme.net\\@evil.example/",
        ] {
            assert!(https_host(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_dot_segment_is_found_however_it_is_spelt() {
        for url in [
            "https://h.example/a/../b",
            "https://h.example/a/./b",
            "https://h.example/..",
            "https://h.example/.",
            "https://h.example/%2e%2e/x",
            "https://h.example/%2E/x",
            "https://h.example/.%2e",
            "https://h.example/a/%2e%2E",
            "https://h.example/a\\..\\b",
            "https://h.example/browse/..?q=1",
            "https://h.example/a/..#frag",
        ] {
            assert!(has_dot_segment(url), "{url}");
        }
    }

    #[test]
    fn other_paths_are_not_dot_segments() {
        for url in [
            "https://h.example",
            "https://h.example/",
            "https://h.example/a/b.c",
            "https://h.example/...",
            "https://h.example/a/.hidden",
            "https://h.example/%252e",
            "https://h.example/{1}",
            "https://h.example/browse/ACME-1",
            "https://h.example/?q=../..",
            "https://h.example#../",
            "https://h.example/a%2",
            "https://h.example/a%é2e",
        ] {
            assert!(!has_dot_segment(url), "{url}");
        }
    }
}
