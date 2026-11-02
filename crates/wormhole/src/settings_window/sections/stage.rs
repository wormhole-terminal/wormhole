//! Appearance's live preview: one window on the preview card, compiled from the design's `WindowMock`
//! (`ui/generated/window_mock.rs`). The picture is the window's look in miniature, in the theme's colours:
//! title row with the parts the person chose, a sketched sidebar, a few terminal lines and the status bar.
//! This file only fills the component from the settings; where everything sits is the design's.

use gpui_kit::*;
use wormhole_core::config::Settings;
use wormhole_core::view::layout::title_parts::TitlePart;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::card::{Words, card};
use crate::settings::{SettingsStore, settings};
use crate::settings_window::SettingsWindow;
use crate::theme_bridge::{hsla, token};
use crate::ui::{Bar, Line, Look, Part, RightPart, WindowMockProps};

/// The same colour, `alpha` as opaque.
fn faded(color: Hsla, alpha: f32) -> Hsla {
    Hsla { a: color.a * alpha, ..color }
}

/// What a title row part shows in the preview.
fn sample(part: TitlePart) -> &'static str {
    // text: sample (what each part of the title row may show, made up for the preview)
    match part {
        TitlePart::Folder => "wormhole",
        TitlePart::Branch => "main",
        TitlePart::BranchStatus => "↑2",
        TitlePart::Terminal => "cargo test",
        TitlePart::TerminalStatus => "Working",
        TitlePart::AgentGauge => "Opus · 42%",
        TitlePart::Ports => ":3000",
        TitlePart::PullRequest => "#128",
        TitlePart::OpenIn => "Zed",
        TitlePart::Splits => "◫",
        TitlePart::Space => "",
    }
}

/// Appearance → the window with the theme, opacity, dividers, status bar and title row as they are now.
pub(in crate::settings_window) fn draw_stage(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = crate::settings::ui(cx);
    let store = cx.global::<SettingsStore>();
    let tokens = &store.tokens;
    let own: &Settings = &store.settings_without_zen();
    let (bg, fg) = (hsla(tokens.terminal.background), hsla(tokens.terminal.foreground));
    let ansi = |i: usize| hsla(tokens.terminal.ansi[i]);
    let (blue, green) = (ansi(4), ansi(2));
    let row = own.title_row().into_owned();
    let (left, right) = row.ends();
    let part_of = |part: &TitlePart| {
        let branch = *part == TitlePart::Branch;
        let plain = matches!(part, TitlePart::Folder | TitlePart::Terminal);
        (sample(*part), if branch { token(tokens, "git.branch") } else if plain { fg } else { faded(fg, 0.6) }, branch)
    };
    let side = |ix: usize| {
        let fill = match (own.liquid_glass, ix) {
            (true, 0) => faded(gpui_kit::white(), 0.16),
            (true, _) => faded(gpui_kit::white(), 0.07),
            (false, 0) => token(tokens, "primary"),
            (false, _) => faded(fg, 0.08),
        };
        Bar { fill }
    };
    // text: sample (the preview's made-up terminal lines)
    let lines = [("~/wormhole $ cargo test", blue), ("   Compiling wormhole v1.5.0", fg), ("test result: ok. 412 passed", green), ("~/wormhole $ ", blue)];
    let mock = WindowMockProps {
        id: "appearance-window-mock".into(),
        spoken: text_of(SETTINGS_WINDOW_PREVIEW_WINDOW_LABEL),
        fill: faded(bg, own.opacity),
        fg,
        rule: faded(fg, 0.12),
        has_rule: own.column_dividers,
        status_color: faded(fg, 0.7),
        has_status: own.status_bar,
        branch: "main".into(), // text: sample
        changed: "3 changed".into(), // text: sample
        shell: "zsh".into(), // text: sample
        left: left
            .iter()
            .map(part_of)
            .map(|(text, color, bold)| Part { text: text.into(), color, bold })
            .collect(),
        right: right
            .iter()
            .map(part_of)
            .map(|(text, color, bold)| RightPart { text: text.into(), color, bold })
            .collect(),
        side: (0..3).map(side).collect(),
        lines: lines
            .iter()
            .map(|(text, color)| Line { text: (*text).into(), color: *color })
            .collect(),
    };
    let _ = settings(cx);
    card(
        ui,
        Words {
            id: "appearance-preview-card",
            what: SETTINGS_WINDOW_WINDOW,
            spoken: SETTINGS_WINDOW_PREVIEW_WINDOW_LABEL,
            caption: SETTINGS_APPEARANCE_STAGE_CAPTION,
        },
        div().child(mock.render(&Look::of(cx))),
    )
}

fn text_of(key: wormhole_text::Key) -> SharedString {
    wormhole_text::text(key).into()
}
