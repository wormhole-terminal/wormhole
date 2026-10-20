//! The examples that are tools or documentation run here, so that they keep working with the API:
//! each is included as a module and its logic is called, with the checks it carries (`quickstart`
//! and `repaint` assert as they go) and a few of ours (`ansi_to_html`'s page).
//!
//! `cargo test --examples` builds every example as well; this runs the ones that do not need a
//! file or a terminal.

#[allow(dead_code)]
#[path = "../examples/ansi_to_html.rs"]
mod ansi_to_html;
#[allow(dead_code)]
#[path = "../examples/quickstart.rs"]
mod quickstart;
#[allow(dead_code)]
#[path = "../examples/repaint.rs"]
mod repaint;

#[test]
fn quickstart_runs() {
    quickstart::main();
}

#[test]
fn repaint_rebuilds_a_busy_terminal() {
    repaint::main().expect("a terminal between sequences can be repainted");
}

#[test]
fn the_repaint_check_notices_a_difference() {
    let original = repaint::busy_terminal();
    let mut replica = repaint::restore(&original).expect("between sequences");
    repaint::assert_same(&original, &replica);

    replica.feed(b"\x1b[1;1HX");
    let different = std::panic::catch_unwind(|| repaint::assert_same(&original, &replica));
    assert!(different.is_err(), "a changed cell must fail the comparison");
}

#[test]
fn ansi_to_html_makes_styled_spans_and_escapes_the_rest() {
    let page = ansi_to_html::convert(b"a<b>&\x1b[1;31mred\x1b[0m plain\r\n", 40);
    assert!(page.contains("a&lt;b&gt;&amp;"), "{page}");
    assert!(page.contains("<span style=\"color:#cc0000;font-weight:bold;\">red</span> plain\n"), "{page}");
}

#[test]
fn ansi_to_html_reads_the_screen_not_the_bytes() {
    // A carriage return and an erase replace what was printed; the page shows the result.
    let page = ansi_to_html::convert(b"hello\rj\x1b[K\r\nsecond", 40);
    assert!(page.contains("<pre>j\nsecond\n</pre>"), "{page}");
}

#[test]
fn ansi_to_html_draws_a_wide_character_once() {
    let page = ansi_to_html::convert("\u{4e16}\u{754c}\r\n".as_bytes(), 40);
    assert!(page.contains("<pre>\u{4e16}\u{754c}\n</pre>"), "{page}");
}

#[test]
fn ansi_to_html_links_only_safe_schemes() {
    let input = b"\x1b]8;;https://example.org/?a=1&b=\"2\"\x07ok\x1b]8;;\x07 \
                  \x1b]8;;javascript:alert(1)\x07bad\x1b]8;;\x07 \
                  \x1b]8;;data:text/html,x\x07worse\x1b]8;;\x07";
    let page = ansi_to_html::convert(input, 80);
    assert!(page.contains("<a href=\"https://example.org/?a=1&amp;b=&quot;2&quot;\">ok</a>"), "{page}");
    assert!(!page.contains("javascript:") && !page.contains("data:text"), "{page}");
    assert!(page.contains("bad") && page.contains("worse"), "the text stays: {page}");
}

#[test]
fn ansi_to_html_keeps_history_and_joins_wrapped_lines() {
    let mut input = Vec::new();
    for n in 0..120 {
        input.extend_from_slice(format!("line {n}\r\n").as_bytes());
    }
    input.extend_from_slice(b"0123456789abcdefghij\r\n"); // wraps at 8 columns
    let page = ansi_to_html::convert(&input, 8);
    assert!(page.contains("<pre>line 0\n"), "the first line scrolled into the history: {page}");
    assert!(page.contains("\n0123456789abcdefghij\n"), "a soft wrap is one line: {page}");
}

#[test]
fn ansi_to_html_uses_the_palette_a_program_set() {
    let page = ansi_to_html::convert(b"\x1b]4;1;rgb:12/34/56\x07\x1b[31mred\x1b[0m\r\n", 40);
    assert!(page.contains("color:#123456;"), "{page}");
}
