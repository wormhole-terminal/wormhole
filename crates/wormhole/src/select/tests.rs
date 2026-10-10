//! Matching, filtering and when a list is virtualized.

use super::{Choice, Needle, filter, virtualized};

fn options() -> Vec<Choice<u8>> {
    vec![
        Choice::new(0, "All projects"),
        Choice::new(1, "wormhole").detail("~/Projects/terminal"),
        Choice::new(2, "Website").detail("~/Sites/web"),
    ]
}

fn values(options: &[Choice<u8>]) -> Vec<u8> {
    options.iter().map(|c| c.value).collect()
}

#[test]
fn an_empty_query_keeps_every_option_in_order() {
    assert_eq!(values(&filter(&options(), "")), [0, 1, 2]);
    assert_eq!(values(&filter(&options(), "  ")), [0, 1, 2]);
}

#[test]
fn matches_the_label_ignoring_case_and_surrounding_spaces() {
    assert_eq!(values(&filter(&options(), " WEB ")), [2]);
    assert_eq!(values(&filter(&options(), "rmhol")), [1]);
}

#[test]
fn matches_the_second_line_too() {
    assert_eq!(values(&filter(&options(), "sites")), [2]);
    assert_eq!(values(&filter(&options(), "projects")), [0, 1]);
}

#[test]
fn no_match_leaves_nothing() {
    assert!(filter(&options(), "zzz").is_empty());
}

#[test]
fn a_needle_is_found_anywhere_in_the_text_in_any_case() {
    let needle = Needle::new("  ÄRG ");
    assert!(needle.found_in("Ärger"));
    assert!(needle.found_in("zum ärgerlich"));
    assert!(!needle.found_in("Arger"));
    assert!(Needle::new("").found_in("anything"));
    assert!(Needle::new("x").found_in("a X b"));
    assert!(!Needle::new("longer than the text").found_in("short"));
}

#[test]
fn only_long_lists_of_single_lines_are_virtualized() {
    assert!(!virtualized(99, false));
    assert!(virtualized(100, false));
    assert!(virtualized(5000, false));
    // A second line makes rows differ in height, which a uniform list cannot draw.
    assert!(!virtualized(5000, true));
}

#[test]
fn a_needle_that_runs_past_the_end_is_not_a_match() {
    assert!(!Needle::new("abc").found_in("ab"));
    assert!(Needle::new("abc").found_in("abc"));
    assert!(!Needle::new("abc").found_in(""));
}

/// The old filter, which lowercased the query and both lines of every option for every
/// keystroke, against the new one on five thousand options. Prints; run with
/// `--ignored --nocapture`.
#[test]
#[ignore]
#[allow(clippy::print_stdout)] // a benchmark: the numbers are the output
fn filtering_five_thousand_options() {
    let options: Vec<Choice<usize>> = (0..5000)
        .map(|i| Choice::new(i, format!("feature/Branch-{i}")).detail(format!("origin/feature/Branch-{i}")))
        .collect();
    let old = |query: &str| {
        options
            .iter()
            .filter(|c| {
                let query = query.trim().to_lowercase();
                query.is_empty()
                    || c.label.to_lowercase().contains(&query)
                    || c.detail.as_ref().is_some_and(|d| d.to_lowercase().contains(&query))
            })
            .count()
    };
    for query in ["BRANCH-49", "zzz"] {
        let t = std::time::Instant::now();
        let n = (0..20).map(|_| old(query)).sum::<usize>() / 20;
        let old_us = t.elapsed().as_secs_f64() * 1e6 / 20.0;
        let t = std::time::Instant::now();
        let m = (0..20).map(|_| filter(&options, query).len()).sum::<usize>() / 20;
        let new_us = t.elapsed().as_secs_f64() * 1e6 / 20.0;
        assert_eq!(n, m);
        println!("{query:?}: old {old_us:.0} us, new {new_us:.0} us ({m} matches)");
    }
}
