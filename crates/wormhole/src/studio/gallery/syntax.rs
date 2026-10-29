//! Highlighting from the theme's syntax tokens.

use std::ops::Range;
use std::sync::Arc;

use gpui_kit::component::highlighter::{HighlightTheme, Language, SyntaxHighlighter};
use gpui_kit::component::input::Rope;
use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::settings::SettingsStore;
use crate::style::TextRole;
use crate::theme_bridge::hsla;

const CODE_SAMPLE: &str = "/// Count words, ignoring case.\nfn count(text: &str) -> usize {\n    let words = text.split_whitespace();\n    words.map(str::to_lowercase).count() // 42\n}"; // text: sample (the gallery's specimen)

/// A line of the sample and the styles its syntax theme gives it.
type Line = (&'static str, Vec<(Range<usize>, HighlightStyle)>);

/// The syntax sample, parsed once. Building a highlighter compiles the language's whole
/// highlight query (about 25 ms for Rust), so doing it on every render made each step of
/// a color drag in the Studio cost several frames; the sample never changes, only the
/// styles its theme gives it, and those are cheap to look up again.
pub(in crate::studio) struct CodeSample {
    highlighter: SyntaxHighlighter,
    lines: Vec<Line>,
    /// The syntax theme `lines` is styled for.
    theme: Arc<HighlightTheme>,
}

impl CodeSample {
    pub(in crate::studio) fn new(theme: &Arc<HighlightTheme>) -> Self {
        tally("builds");
        let mut highlighter = SyntaxHighlighter::new(Language::from_str("rs").name());
        highlighter.update(None, &Rope::from(CODE_SAMPLE), None);
        let lines = styled(&highlighter, theme);
        Self { highlighter, lines, theme: theme.clone() }
    }

    /// The lines, styled for `theme` (looked up again if it is not the one they were).
    fn lines(&mut self, theme: &Arc<HighlightTheme>) -> &[Line] {
        if !Arc::ptr_eq(&self.theme, theme) {
            self.lines = styled(&self.highlighter, theme);
            self.theme = theme.clone();
        }
        &self.lines
    }
}

fn styled(highlighter: &SyntaxHighlighter, theme: &Arc<HighlightTheme>) -> Vec<Line> {
    tally("styles");
    let mut offset = 0;
    CODE_SAMPLE
        .split('\n')
        .map(|line| {
            let range = offset..offset + line.len();
            let styles = highlighter
                .styles(&range, &**theme)
                .into_iter()
                .filter(|(r, _)| r.start < r.end)
                .map(|(r, s)| (r.start - offset..r.end - offset, s))
                .collect();
            offset += line.len() + 1;
            (line, styles)
        })
        .collect()
}

pub(super) fn render(
    studio: &GalleryView,
    env: &Env,
    _part: usize,
    cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let term = &env.tokens.terminal;
    let highlight = cx.global::<SettingsStore>().highlight.clone();
    let mut sample = studio.sample.borrow_mut();
    let sample = sample.get_or_insert_with(|| CodeSample::new(&highlight));
    let mut code = div()
        .flex()
        .flex_col()
        .p_3()
        .rounded(ui.radius_lg)
        .bg(hsla(term.background))
        .text_color(hsla(term.foreground))
        .font_family(crate::fonts::code_font(cx).0)
        .text_body();
    for (line, styles) in sample.lines(&highlight) {
        code = code.child(
            div()
                .h(px(20.0))
                .whitespace_nowrap()
                .overflow_hidden()
                .child(StyledText::new(line.to_string()).with_highlights(styles.iter().cloned())),
        );
    }
    env.mark("code", code)
}

// Counts what a test is about, per thread (tests run in parallel).
#[cfg(test)]
thread_local! {
    static TALLY: std::cell::RefCell<std::collections::BTreeMap<&'static str, usize>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

#[cfg(test)]
fn tally(what: &'static str) {
    TALLY.with(|t| *t.borrow_mut().entry(what).or_default() += 1);
}

#[cfg(not(test))]
#[inline(always)]
fn tally(_: &'static str) {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui_kit::component::highlighter::HighlightTheme;

    use super::{CODE_SAMPLE, CodeSample, TALLY};

    fn count(what: &str) -> usize {
        TALLY.with(|t| t.borrow().get(what).copied().unwrap_or(0))
    }

    #[test]
    fn the_sample_is_parsed_once_however_often_it_is_drawn() {
        let dark = HighlightTheme::default_dark();
        let mut sample = CodeSample::new(&dark);
        for _ in 0..50 {
            sample.lines(&dark);
        }
        assert_eq!(count("builds"), 1, "the highlighter (and its query) is built once");
        assert_eq!(count("styles"), 1, "and the styles are looked up once for one theme");
    }

    #[test]
    fn a_new_syntax_theme_restyles_without_building_again() {
        let dark = HighlightTheme::default_dark();
        let light = HighlightTheme::default_light();
        let mut sample = CodeSample::new(&dark);
        let before: Vec<_> = sample.lines(&dark).iter().map(|(_, s)| s.clone()).collect();
        let after: Vec<_> = sample.lines(&light).iter().map(|(_, s)| s.clone()).collect();
        assert_eq!(count("builds"), 1);
        assert_eq!(count("styles"), 2);
        assert_ne!(before, after, "the styles follow the theme");
        // Going back is another lookup, and the theme in use is the one asked for.
        assert!(Arc::ptr_eq(&sample.theme, &light));
        sample.lines(&dark);
        assert_eq!(count("styles"), 3);
    }

    #[test]
    fn every_line_of_the_sample_is_there_and_the_code_is_highlighted() {
        let dark = HighlightTheme::default_dark();
        let mut sample = CodeSample::new(&dark);
        let lines = sample.lines(&dark);
        assert_eq!(lines.len(), CODE_SAMPLE.split('\n').count());
        let joined: Vec<_> = lines.iter().map(|(line, _)| *line).collect();
        assert_eq!(joined.join("\n"), CODE_SAMPLE);
        assert!(lines.iter().any(|(_, styles)| !styles.is_empty()), "nothing was highlighted");
        for (line, styles) in lines {
            assert!(styles.iter().all(|(r, _)| r.end <= line.len()), "a style reaches past its line");
        }
    }
}
