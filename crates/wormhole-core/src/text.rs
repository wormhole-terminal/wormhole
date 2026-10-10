//! Small text helpers shared by the file formats (themes, settings).

/// `" (did you mean `x`?)"` for the known name closest to `name`, or an empty
/// string when nothing is close enough to be a likely typo.
pub(crate) fn did_you_mean<'a>(name: &str, known: impl Iterator<Item = &'a str>) -> String {
    closest(name, known).map_or(String::new(), |k| format!(" (did you mean `{k}`?)"))
}

/// The known name closest to `name`, if one is close enough to be a likely typo.
pub(crate) fn closest<'a>(name: &str, known: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    fn dist(a: &str, b: &str) -> usize {
        let b: Vec<char> = b.chars().collect();
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        for (i, ca) in a.chars().enumerate() {
            let mut cur = vec![i + 1];
            for (j, cb) in b.iter().enumerate() {
                cur.push((prev[j] + usize::from(ca != *cb)).min(prev[j + 1] + 1).min(cur[j] + 1));
            }
            prev = cur;
        }
        prev[b.len()]
    }
    let limit = (name.len() / 3).max(2);
    known.map(|k| (dist(name, k), k)).filter(|(d, _)| *d <= limit).min().map(|(_, k)| k)
}

#[cfg(test)]
mod tests {
    use super::did_you_mean;

    #[test]
    fn suggests_the_closest_name_within_reach() {
        let known = ["primary", "primary.hover", "border"];
        assert_eq!(did_you_mean("primry", known.into_iter()), " (did you mean `primary`?)");
        assert_eq!(did_you_mean("boder", known.into_iter()), " (did you mean `border`?)");
        assert_eq!(did_you_mean("zzzzzzzz", known.into_iter()), "");
        // Ties go to the alphabetically first name, so the hint is stable.
        assert_eq!(did_you_mean("ab", ["ac", "aa"].into_iter()), " (did you mean `aa`?)");
    }
}
