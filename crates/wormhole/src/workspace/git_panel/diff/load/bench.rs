//! Timings for the diff pipeline, to compare with the numbers in the commit that
//! moved it off the UI thread: `cargo test -p wormhole --features devctl
//! bench_diff_pipeline -- --ignored --nocapture`.

use std::time::Instant;

use super::build;
use crate::workspace::git_panel::diff::rows::BuildKey;
use gpui_kit::component::highlighter::HighlightTheme;
use wormhole_core::git::{DiffLine, FileDiff, Hunk, LineKind};

fn source(functions: usize, tweak: bool) -> String {
    let mut text = String::new();
    for f in 0..functions {
        let t = if tweak && f % 40 == 7 { " + 1" } else { "" };
        text.push_str(&format!(
            "/// Doc for item {f}.\npub fn item_{f}(input: &str, count: usize) -> Result<Vec<String>, Error> {{\n    let mut out = Vec::new();\n    for part in input.split(',') {{\n        if part.len() > count {t} {{\n            out.push(part.to_string());\n        }}\n    }}\n    Ok(out)\n}}\n\n"
        ));
    }
    text
}

/// A file of `functions` functions with a one-line change in every 40th.
fn diff(functions: usize) -> FileDiff {
    let (old, new) = (source(functions, false), source(functions, true));
    let (ol, nl): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let line =
        |kind, old, new, text: &str| DiffLine { kind, old, new, text: text.to_string(), no_newline: false };
    let mut hunks = Vec::new();
    for (i, (o, n)) in ol.iter().zip(&nl).enumerate().filter(|(_, (o, n))| o != n) {
        let n_ = i as u32 + 1;
        let context = |from: usize, to: usize| {
            (from..to).map(|c| line(LineKind::Context, Some(c as u32 + 1), Some(c as u32 + 1), ol[c]))
        };
        let mut lines: Vec<DiffLine> = context(i.saturating_sub(3), i).collect();
        lines.push(line(LineKind::Removed, Some(n_), None, o));
        lines.push(line(LineKind::Added, None, Some(n_), n));
        lines.extend(context(i + 1, (i + 4).min(ol.len())));
        hunks.push(Hunk { header: format!("@@ -{i},7 +{i},7 @@"), lines });
    }
    FileDiff { hunks, old_text: Some(old), new_text: Some(new), ..FileDiff::default() }
}

#[test]
#[ignore = "timing, not a check"]
// The timings are the output.
#[allow(clippy::print_stdout)]
fn bench_diff_pipeline() {
    let dark = HighlightTheme::default_dark();
    let light = HighlightTheme::default_light();
    let key = |split| BuildKey { split, tab_width: 4, syntax: 0 };
    let time = |what: &str, f: &dyn Fn()| {
        let start = Instant::now();
        f();
        println!("BENCH {what}: {:.1} ms", start.elapsed().as_secs_f64() * 1000.0);
    };
    // Warm nothing: the first diff of a language pays for its queries.
    let tiny = diff(3);
    time("first diff of the language (2 query compiles, 2 parses)", &|| {
        build(&tiny, "a.rs", key(true), &dark);
    });
    for functions in [100, 1000, 5000] {
        let d = diff(functions);
        let lines = d.new_text.as_deref().map_or(0, |t| t.lines().count());
        time(&format!("{lines} lines, new texts (parse, no compile)"), &|| {
            build(&d, "a.rs", key(true), &dark);
        });
        time(&format!("{lines} lines, split -> unified (parse cached)"), &|| {
            build(&d, "a.rs", key(false), &dark);
        });
        time(&format!("{lines} lines, other theme (parse cached, colors again)"), &|| {
            build(&d, "a.rs", key(false), &light);
        });
    }
}
