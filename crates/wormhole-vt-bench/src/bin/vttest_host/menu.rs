//! A vttest menu as it is drawn: its entries, its highest choice, and what identifies it.

/// A menu entry as vttest lists it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub number: u32,
    pub name: String,
    /// vttest marks an entry it has not written with `*` instead of `.`.
    pub unimplemented: bool,
}

/// The entries on a menu screen and the highest number the prompt allows.
pub fn parse(rows: &[String]) -> (Vec<Entry>, u32) {
    let mut entries = Vec::new();
    let mut highest = 0;
    for row in rows {
        if let Some(rest) = row.split("Enter choice number (0 - ").nth(1)
            && let Some(n) = rest.split(')').next().and_then(|n| n.parse().ok())
        {
            highest = n;
        }
        // "          3. Test of character sets", or "3* ..." for an entry vttest has not written.
        let trimmed = row.trim_start();
        let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
        let after = &trimmed[digits..];
        if digits == 0
            || !(after.starts_with(". ") || after.starts_with("* "))
            || row.len() - trimmed.len() < 8
        {
            continue;
        }
        if let Ok(number) = trimmed[..digits].parse() {
            entries.push(Entry {
                number,
                name: after[2..].trim().to_string(),
                unimplemented: after.starts_with('*'),
            });
        }
    }
    (entries, highest)
}

/// What identifies a menu: the lines above its first entry and the highest choice. Not the
/// entries, whose names change (a toggle says "Enable" then "Disable"), nor the page they are on.
pub fn fingerprint(rows: &[String]) -> String {
    let title: Vec<&str> =
        rows.iter().map(String::as_str).take_while(|r| !r.trim_start().starts_with("0. Exit")).collect();
    let (_, highest) = parse(rows);
    format!("{highest}\n{}", title.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::{fingerprint, parse};

    fn screen() -> Vec<String> {
        [
            "",
            "         Choose test type:",
            "          0. Exit",
            "          1. Test of cursor movements",
            "          2* Test of keyboard",
            "          12. Modify test-parameters",
            "",
            "          Enter choice number (0 - 12): ",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    #[test]
    fn entries_are_read_off_the_screen() {
        let (entries, highest) = parse(&screen());
        assert_eq!(highest, 12);
        let seen: Vec<_> = entries.iter().map(|e| (e.number, e.name.as_str(), e.unimplemented)).collect();
        assert_eq!(
            seen,
            [
                (0, "Exit", false),
                (1, "Test of cursor movements", false),
                (2, "Test of keyboard", true),
                (12, "Modify test-parameters", false)
            ]
        );
    }

    #[test]
    fn a_toggle_changing_its_name_leaves_the_menu_the_same() {
        let mut other = screen();
        other[3] = "          1. Another name".into();
        assert_eq!(fingerprint(&screen()), fingerprint(&other));
    }

    #[test]
    fn a_numbered_line_of_a_test_is_not_a_menu_entry() {
        // Indented less than a menu is, so the numbered lines of a test screen do not count.
        let (entries, _) = parse(&["1. a numbered line".to_string()]);
        assert!(entries.is_empty());
    }
}
