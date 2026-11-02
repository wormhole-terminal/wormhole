//! Terminal's preview: two split terminals, compiled from the design's `SplitMock`
//! (`ui/generated/split_mock.rs`) and filled from the settings: the gaps between the panes and the minimap
//! along their right edges. The picture is the design's miniature (11-point code, a fixed height), in the
//! theme's colours; only the sample lines and the colours are chosen here.

use gpui_kit::*;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::card::{Words, card};
use crate::settings::{SettingsStore, settings};
use crate::settings_window::{SettingsWindow, ui};
use crate::theme_bridge::hsla;
use crate::ui::{LeftLine, Look, RightLine, SplitMockProps};

/// The same colour, `alpha` as opaque.
fn faded(color: Hsla, alpha: f32) -> Hsla {
    Hsla { a: color.a * alpha, ..color }
}

/// Terminal → two split terminals under the page's options.
pub(in crate::settings_window) fn draw_split_preview(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let (gaps, minimap) = (settings(cx).split_gaps, settings(cx).minimap);
    let colors = &cx.global::<SettingsStore>().tokens.terminal;
    let (bg, fg) = (hsla(colors.background), hsla(colors.foreground));
    let (blue, green, yellow) = (hsla(colors.ansi[4]), hsla(colors.ansi[2]), hsla(colors.ansi[3]));
    // text: sample (the preview's made-up terminals)
    let left = [
        ("~/api $ cargo test", blue),
        ("running 412 tests", fg),
        ("test limits::burst ... ok", fg),
        ("test limits::refill ... ok", fg),
        ("test result: ok", green),
        ("~/api $ ", blue),
    ];
    // text: sample (the preview's made-up terminals)
    let right = [
        ("~/api $ git log --oneline", blue),
        ("a41f2c9 limits: refill", yellow),
        ("9e03b11 limits: burst", yellow),
        ("77d0a2e api: routes", yellow),
        ("~/api $ ", blue),
    ];
    let mock = SplitMockProps {
        id: "terminal-split-mock".into(),
        spoken: wormhole_text::text(SETTINGS_WINDOW_PREVIEW_TERMINAL_LABEL).into(),
        gaps,
        minimap,
        split: !gaps,
        bg,
        // The panes' faint ring when there are gaps: the foreground at 8 % over the pane.
        ring_fill: if gaps { bg.blend(faded(fg, 0.08)) } else { bg },
        sep: faded(fg, 0.16),
        rule: faded(fg, 0.16),
        bar: faded(fg, 0.28),
        left: left.iter().map(|(text, color)| LeftLine { text: (*text).into(), color: *color }).collect(),
        right: right.iter().map(|(text, color)| RightLine { text: (*text).into(), color: *color }).collect(),
    };
    card(
        ui,
        Words {
            id: "terminal-preview-card",
            what: SETTINGS_WINDOW_SPLIT_TERMINALS,
            spoken: SETTINGS_WINDOW_PREVIEW_TERMINAL_LABEL,
            caption: SETTINGS_TERMINAL_PREVIEW_CAPTION,
        },
        div().child(mock.render(&Look::of(cx))),
    )
}
