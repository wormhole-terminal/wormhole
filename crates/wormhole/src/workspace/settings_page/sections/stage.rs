//! Appearance's live preview: the window in miniature over a wallpaper, in
//! the current theme, primary and opacity, drawn with the design's own words at about a third of
//! their size (its 1440 × 900 window at [`SCALE`]) rather than as bars. The blur isn't drawn:
//! it is the window's own effect view behind everything GPUI paints, which a drawing inside a
//! window can't have; the caption says so.

use wormhole_core::agents::AgentId;
use wormhole_core::fonts::FontSlot;

use super::*;
use crate::theme_bridge::hsla;

/// The design's window at this scale: every size below is the real one times it, so the
/// miniature keeps the window's proportions.
const SCALE: f32 = 0.36;
const STAGE_H: f32 = 356.0;
const WINDOW_W: f32 = 1440.0 * SCALE;
const WINDOW_H: f32 = 900.0 * SCALE;
const MINI_SIDEBAR_W: f32 = 350.0 * SCALE;
const MINI_PANEL_W: f32 = 360.0 * SCALE;
const MINI_TITLE_H: f32 = 40.0 * SCALE;
const MINI_BAND_H: f32 = 32.0 * SCALE;
const MINI_ROW_H: f32 = 28.0 * SCALE;
const MINI_ROW_TWO_H: f32 = 44.0 * SCALE;
const MINI_STATUS_H: f32 = 28.0 * SCALE;
const MINI_INSET: f32 = 16.0 * SCALE;
const MINI_GAP: f32 = 8.0 * SCALE;
const MINI_GAP_SM: f32 = 4.0 * SCALE;
const MINI_LEAD: f32 = 16.0 * SCALE;
const MINI_TEXT: f32 = 14.0 * SCALE;
const MINI_LABEL: f32 = 13.0 * SCALE;
const MINI_CAPTION: f32 = 12.0 * SCALE;
const MINI_HEADING: f32 = 16.0 * SCALE;
const MINI_LINE: f32 = 20.0 * SCALE;
const MINI_CAPTION_LINE: f32 = 16.0 * SCALE;
const MINI_ICON: f32 = 16.0 * SCALE;
const MINI_GLYPH: f32 = 12.0 * SCALE;
/// The title row's folder, separators and tool icon (`style::GLYPH`).
const MINI_CRUMB_ICON: f32 = 14.0 * SCALE;
const MINI_DOT: f32 = 8.0 * SCALE;
const MINI_CHIP_H: f32 = 20.0 * SCALE;
const MINI_CHIP_PAD: f32 = 6.0 * SCALE;
const MINI_TAB_H: f32 = 28.0 * SCALE;
const MINI_TERM_PAD: f32 = 12.0 * SCALE;
const MINI_NOTE_INDENT: f32 = 40.0 * SCALE;
const MINI_CHECK: f32 = 14.0 * SCALE;
const MINI_METER_W: f32 = 48.0 * SCALE;
const MINI_METER_H: f32 = 4.0 * SCALE;
const MINI_CURSOR_W: f32 = 8.0 * SCALE;
const MINI_CURSOR_H: f32 = 16.0 * SCALE;
const MINI_DIVIDER_H: f32 = 12.0 * SCALE;
/// The window's corners and hairlines are not scaled: a 0.36 hairline would vanish.
const MINI_RADIUS: f32 = 6.0;
const MINI_CORNER: f32 = 2.0;
const MINI_RULE: f32 = 1.0;
/// The wallpaper's two washes, from the theme's own colors.
const WASH_ALPHA: f32 = 0.55;
const WASH_ANGLE: f32 = 135.0;
/// The cursor block, a little see-through as the terminal draws it.
const CURSOR_OPACITY: f32 = 0.8;
/// How full the agent's context meter is (62 %).
const CONTEXT_USED: f32 = 0.62;

/// Appearance → the window over the desktop: theme, primary and opacity as they are now.
pub(super) fn draw_stage(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    // The main window in miniature: its colours, whatever this window's look.
    let ui = crate::settings::ui(cx);
    let store = cx.global::<SettingsStore>();
    let tokens = &store.tokens;
    let status_bar = store.settings.status_bar;
    let base = hsla(tokens.terminal.background.with_alpha(1.0));
    let (mono, _) = crate::fonts::font(FontSlot::Terminal, cx);
    let mini = Mini {
        ui,
        filter: crate::workspace::sidebar::sidebar_filter_on(cx),
        mono,
        green: hsla(tokens.terminal.ansi[2]),
        yellow: hsla(tokens.terminal.ansi[3]),
    };
    let wallpaper = linear_gradient(
        WASH_ANGLE,
        linear_color_stop(Hsla { a: WASH_ALPHA, ..ui.branch }, 0.0),
        linear_color_stop(Hsla { a: WASH_ALPHA, ..ui.info }, 1.0),
    );
    let window = div()
        .w(px(WINDOW_W))
        .h(px(WINDOW_H))
        .flex()
        .flex_col()
        .rounded(px(MINI_RADIUS))
        .overflow_hidden()
        .bg(ui.background)
        .border(px(MINI_RULE))
        .border_color(ui.border_strong)
        .child(
            div().flex_1().min_h_0().flex().child(mini.sidebar()).child(mini.terminal()).child(mini.panel()),
        )
        .when(status_bar, |d| d.child(mini.status_bar()));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .h(px(STAGE_H))
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded(ui.radius_lg)
                .overflow_hidden()
                .outlined(ui, ui.border)
                .bg(base)
                .child(div().size_full().flex().items_center().justify_center().bg(wallpaper).child(window)),
        )
        .child(div().text_caption().text_color(ui.muted).child(
            "Live preview: your windows over the desktop, with everything below applied as you change it \
             (the blur behind a window isn't drawn here)",
        ))
        .into_any_element()
}

/// What the miniature draws with: the theme's colors, the terminal's font, and the ANSI colors
/// an agent's transcript uses.
struct Mini {
    ui: UiColors,
    /// The sidebar's funnel exists (its feature flag is on).
    filter: bool,
    mono: SharedString,
    green: Hsla,
    yellow: Hsla,
}

impl Mini {
    /// A line of interface text at the miniature's size, on one line.
    fn text(&self, text: impl Into<SharedString>, size: f32, line: f32, color: Hsla) -> Div {
        div()
            .flex_none()
            .min_w_0()
            .text_size(px(size))
            .line_height(px(line))
            .text_color(color)
            .whitespace_nowrap()
            .overflow_hidden()
            .child(text.into())
    }

    fn body(&self, text: impl Into<SharedString>, color: Hsla) -> Div {
        self.text(text, MINI_TEXT, MINI_LINE, color)
    }

    fn caption(&self, text: impl Into<SharedString>, color: Hsla) -> Div {
        self.text(text, MINI_CAPTION, MINI_CAPTION_LINE, color)
    }

    /// An icon at the miniature's size: an `IconName` draws itself at the text size around it.
    fn icon(&self, icon: IconName, size: f32, color: Hsla) -> Div {
        div().flex_none().flex().items_center().text_size(px(size)).text_color(color).child(icon)
    }

    /// The lead column of a row or band.
    fn lead(&self, child: impl IntoElement) -> Div {
        div().flex_none().size(px(MINI_LEAD)).flex().items_center().justify_center().child(child)
    }

    /// A status dot: filled, or a ring for an idle terminal.
    fn dot(&self, color: Hsla, hollow: bool) -> Div {
        let dot = div().flex_none().size(px(MINI_DOT)).rounded_full();
        if hollow { dot.border(px(MINI_RULE)).border_color(color) } else { dot.bg(color) }
    }

    /// The working spinner, held still: a ring in the agent's working color.
    fn ring(&self, color: Hsla) -> Div {
        div().flex_none().size(px(MINI_GLYPH)).rounded_full().border(px(MINI_RULE)).border_color(color)
    }

    /// A folder band's branch chip.
    fn chip(&self, text: &'static str) -> Div {
        let ui = self.ui;
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(MINI_GAP_SM))
            .h(px(MINI_CHIP_H))
            .px(px(MINI_CHIP_PAD))
            .rounded(px(MINI_CORNER))
            .bg(ui.branch_bg)
            .child(self.icon(IconName::GitBranch, MINI_GLYPH, ui.branch))
            .child(self.caption(text, ui.branch).font_weight(FontWeight::MEDIUM))
    }

    /// A count badge.
    fn badge(&self, text: &'static str) -> Div {
        let ui = self.ui;
        div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .min_w(px(MINI_LEAD))
            .h(px(MINI_LEAD))
            .px(px(MINI_GAP_SM))
            .rounded(px(MINI_CORNER))
            .bg(ui.border)
            .child(self.caption(text, ui.text).font_weight(FontWeight::SEMIBOLD))
    }

    /// A title row (the sidebar's, a pane's, the panel's): 40 high in the window.
    fn title_row(&self) -> Div {
        div()
            .flex_none()
            .h(px(MINI_TITLE_H))
            .flex()
            .items_center()
            .gap(px(MINI_GAP_SM))
            .pl(px(MINI_INSET))
            .pr(px(MINI_GAP))
    }

    /// A folder or section band: chevron, name, what follows it, and what sits at its end.
    fn band(&self, name: &'static str, detail: Option<Div>, trail: Vec<Div>, folded: bool) -> Div {
        let ui = self.ui;
        let chevron = if folded { IconName::ChevronRight } else { IconName::ChevronDown };
        div()
            .flex_none()
            .h(px(MINI_BAND_H))
            .flex()
            .items_center()
            .gap(px(MINI_GAP))
            .px(px(MINI_INSET))
            .bg(ui.band)
            .border_t(px(MINI_RULE))
            .border_b(px(MINI_RULE))
            .border_color(ui.border)
            .child(self.lead(self.icon(chevron, MINI_GLYPH, ui.muted)))
            .child(self.text(name, MINI_LABEL, MINI_LINE, ui.text).font_weight(FontWeight::SEMIBOLD))
            .children(detail)
            .child(div().flex_1())
            .children(trail)
    }

    /// A terminal's row: its glyph, name and, on a second line, what it is doing.
    fn row(
        &self,
        lead: Div,
        name: &'static str,
        detail: Option<(&'static str, Hsla)>,
        selected: bool,
    ) -> Div {
        let ui = self.ui;
        let name_color = if selected { ui.selected_text } else { ui.text };
        let name_weight = if selected { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM };
        div()
            .flex_none()
            .h(px(if detail.is_some() { MINI_ROW_TWO_H } else { MINI_ROW_H }))
            .flex()
            .flex_col()
            .justify_center()
            .px(px(MINI_INSET))
            .when(selected, |d| d.bg(ui.selected_row))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(MINI_GAP))
                    .child(self.lead(lead))
                    .child(self.body(name, name_color).font_weight(name_weight)),
            )
            .children(detail.map(|(detail, color)| {
                div().pl(px(MINI_LEAD + MINI_GAP)).child(
                    self.caption(detail, color).font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    }),
                )
            }))
    }

    /// The sidebar: its title rows, then its folders and terminals.
    fn sidebar(&self) -> Div {
        let ui = self.ui;
        div()
            .w(px(MINI_SIDEBAR_W))
            .flex_none()
            .flex()
            .flex_col()
            .bg(ui.panel)
            .border_r(px(MINI_RULE))
            .border_color(ui.border)
            // The traffic lights' row: the collapse button and the gear.
            .child(
                self.title_row()
                    .child(div().flex_1())
                    .child(self.icon(IconName::PanelLeft, MINI_ICON, ui.muted))
                    .child(self.icon(IconName::Settings, MINI_ICON, ui.muted)),
            )
            .child(
                self.title_row()
                    .child(
                        self.text("Terminals", MINI_HEADING, MINI_LINE, ui.text)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(div().flex_1())
                    .when(self.filter, |d| d.child(self.icon(IconName::Funnel, MINI_ICON, ui.muted)))
                    .child(self.icon(IconName::Plus, MINI_ICON, ui.muted)),
            )
            .child(self.band(
                "terminal",
                Some(self.chip("ui-unify")),
                vec![self.caption("±3", ui.muted)],
                false,
            ))
            .child(self.row(
                self.ring(ui.agent_working),
                "sidebar polish",
                Some(("Editing sidebar.rs…", ui.selected_muted)),
                true,
            ))
            .child(self.row(
                self.dot(ui.agent_attention, false),
                "diff perf",
                Some(("Needs you · Approve cargo test?", ui.agent_attention)),
                false,
            ))
            .child(self.row(self.dot(ui.faint, true), "zsh", None, false))
            .child(self.band(
                "api-server",
                Some(self.chip("feature/auth")),
                vec![self.caption("±1", ui.muted)],
                false,
            ))
            .child(self.row(
                self.icon(IconName::Check, MINI_GLYPH, ui.agent_done),
                "auth tokens",
                Some(("Finished · 12m ago", ui.agent_done)),
                false,
            ))
            .child(self.row(self.dot(ui.faint, true), "rate limits", None, false))
            .child(self.band("dotfiles", None, vec![self.badge("3")], true))
            .child(self.band("website", Some(self.chip("main")), vec![self.badge("2")], true))
    }

    /// The title row's breadcrumb as the window draws it: the folder, the branch chip, then
    /// the terminal's tool, its status, its name and how it is doing. There are no tabs.
    fn breadcrumb(&self) -> Div {
        let ui = self.ui;
        let separator = || self.icon(IconName::ChevronRight, MINI_CRUMB_ICON, ui.faint);
        let folder = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(MINI_GAP))
            .child(self.icon(IconName::Folder, MINI_CRUMB_ICON, ui.muted))
            .child(div().flex().child(self.text("~/Projects/", MINI_LABEL, MINI_LINE, ui.muted)).child(
                self.text("terminal", MINI_LABEL, MINI_LINE, ui.text).font_weight(FontWeight::MEDIUM),
            ));
        let claude = wormhole_core::agents::agent(AgentId::Claude);
        let process = div()
            .flex()
            .items_center()
            .gap(px(MINI_GAP))
            .min_w_0()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(MINI_GAP_SM))
                    .child(style::agent_logo(claude.icon(), claude.monogram, MINI_CRUMB_ICON, ui.text))
                    .child(self.lead(self.ring(ui.agent_working))),
            )
            .child(
                self.text("sidebar polish", MINI_LABEL, MINI_LINE, ui.text).font_weight(FontWeight::MEDIUM),
            )
            .child(self.caption("working · 4m", ui.muted));
        div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(MINI_GAP))
            .px(px(MINI_GAP))
            .overflow_hidden()
            .child(folder)
            .child(separator())
            .child(self.chip("ui-unify"))
            .child(separator())
            .child(process)
    }

    /// A line of the terminal: runs of text in their colors, a bold word among them.
    fn mono_line(&self, parts: &[(&'static str, Hsla, bool)]) -> Div {
        div()
            .flex_none()
            .h(px(MINI_LINE))
            .flex()
            .items_center()
            .whitespace_nowrap()
            .overflow_hidden()
            .children(parts.iter().map(|&(text, color, bold)| {
                div().text_color(color).when(bold, |d| d.font_weight(FontWeight::SEMIBOLD)).child(text)
            }))
    }

    /// The terminal pane: its title row, then an agent's transcript.
    fn terminal(&self) -> Div {
        let ui = self.ui;
        let (text, muted, green, yellow) = (ui.text, ui.muted, self.green, self.yellow);
        let step = |verb: &'static str, what: &'static str| {
            [("● ", green, false), (verb, text, true), (what, text, false)]
        };
        let note = |what: &'static str| [(what, muted, false)];
        let cursor =
            div().flex_none().w(px(MINI_CURSOR_W)).h(px(MINI_CURSOR_H)).bg(ui.text).opacity(CURSOR_OPACITY);
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(ui.background)
            .child(
                self.title_row()
                    .pl(px(MINI_GAP_SM))
                    .border_b(px(MINI_RULE))
                    .border_color(ui.border)
                    .child(self.breadcrumb()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .p(px(MINI_TERM_PAD))
                    .font_family(self.mono.clone())
                    .text_size(px(MINI_LABEL))
                    .line_height(px(MINI_LINE))
                    .child(self.mono_line(&[("~/Projects/terminal on ui-unify", muted, false)]))
                    .child(self.mono_line(&[("$ claude", text, false)]))
                    .child(self.mono_line(&[]))
                    .child(self.mono_line(&[(
                        "> Folder bands, one lead column, and ⌘ hints only while ⌘ is held.",
                        ui.primary,
                        false,
                    )]))
                    .child(self.mono_line(&[]))
                    .child(
                        self.mono_line(&step(
                            "Read",
                            " crates/wormhole/src/workspace/sidebar.rs (532 lines)",
                        )),
                    )
                    .child(self.mono_line(&step("Update", " crates/wormhole/src/workspace/sidebar.rs")))
                    .child(
                        self.mono_line(&note(
                            "└ +18 −11 group_header: branch as a chip, badge right-aligned",
                        )),
                    )
                    .child(self.mono_line(&step("Bash", " cargo test -p wormhole-core")))
                    .child(
                        self.mono_line(&note("└ test result: ok. 214 passed; 0 failed; finished in 3.42s")),
                    )
                    .child(self.mono_line(&step("Update", " crates/wormhole/src/style.rs")))
                    .child(self.mono_line(&note("└ +6 −2 section_header: one shared 28px header row")))
                    .child(self.mono_line(&[]))
                    .child(self.mono_line(&[
                        ("* Editing sidebar.rs… ", yellow, false),
                        ("(esc to interrupt)", muted, false),
                    ]))
                    .child(self.mono_line(&[("> ", text, false)]).child(cursor)),
            )
    }

    /// A next step's row: its checkbox and text.
    fn task(&self, done: bool, text: &'static str) -> Div {
        let ui = self.ui;
        let check = div()
            .flex_none()
            .size(px(MINI_CHECK))
            .rounded(px(MINI_CORNER))
            .flex()
            .items_center()
            .justify_center();
        let check = if done {
            check.bg(ui.checkbox_checked).child(self.icon(IconName::Check, MINI_GLYPH, ui.on_primary))
        } else {
            check.bg(ui.input).border(px(MINI_RULE)).border_color(ui.checkbox_border)
        };
        div()
            .flex_none()
            .h(px(MINI_ROW_H))
            .flex()
            .items_center()
            .gap(px(MINI_GAP))
            .px(px(MINI_INSET))
            .child(self.lead(check))
            .child(self.body(text, if done { ui.muted } else { ui.text }))
    }

    /// A note's row: open or folded, with its title.
    fn note(&self, open: bool, title: &'static str) -> Div {
        let ui = self.ui;
        let chevron = if open { IconName::ChevronDown } else { IconName::ChevronRight };
        div()
            .flex_none()
            .h(px(MINI_ROW_H))
            .flex()
            .items_center()
            .gap(px(MINI_GAP))
            .px(px(MINI_INSET))
            .child(self.lead(self.icon(chevron, MINI_GLYPH, ui.muted)))
            .child(self.body(title, if open { ui.muted } else { ui.text }))
    }

    /// The right panel: its tabs, then Next steps and Notes.
    fn panel(&self) -> Div {
        let ui = self.ui;
        let icon_tab = |icon: IconName, selected: bool| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(MINI_GAP_SM))
                .h(px(MINI_TAB_H))
                .min_w(px(MINI_TAB_H))
                .rounded(px(MINI_CORNER))
                .when(selected, |d| d.bg(ui.selected))
                .child(self.icon(icon, MINI_ICON, if selected { ui.selected_text } else { ui.muted }))
        };
        let add = || self.icon(IconName::Plus, MINI_ICON, ui.muted);
        let note_line = |line: &'static str| {
            self.text(line, MINI_LABEL, MINI_LINE, ui.text).pl(px(MINI_NOTE_INDENT)).pr(px(MINI_INSET))
        };
        div()
            .w(px(MINI_PANEL_W))
            .flex_none()
            .flex()
            .flex_col()
            .bg(ui.panel_right)
            .border_l(px(MINI_RULE))
            .border_color(ui.border)
            .child(
                self.title_row()
                    .pl(px(MINI_GAP))
                    .child(icon_tab(IconName::FileText, true))
                    .child(icon_tab(IconName::GitCompare, false).px(px(MINI_GAP_SM)).child(self.badge("3")))
                    .child(icon_tab(IconName::GitCommitHorizontal, false))
                    .child(icon_tab(IconName::Folder, false))
                    .child(div().flex_1())
                    .child(self.icon(IconName::Inbox, MINI_ICON, ui.muted))
                    .child(self.icon(IconName::PanelRight, MINI_ICON, ui.muted)),
            )
            .child(self.band("Next steps", None, vec![self.badge("2"), add()], false))
            .child(self.task(true, "Fix the flaky login test"))
            .child(self.task(false, "Update README with setup steps"))
            .child(self.task(false, "Tighten folder header spacing"))
            .child(self.band("Notes", None, vec![self.badge("2"), add()], false))
            .child(self.note(true, "Sidebar ideas"))
            .child(note_line("Drop the idle line on quiet rows."))
            .child(note_line("Branch as a chip next to the folder name."))
            .child(note_line(""))
            .child(note_line("One lead column: chevron, status, checkbox."))
            .child(self.note(false, "Release checklist").mt(px(MINI_GAP)))
    }

    /// The status bar: branch, agents, the context meter, the terminal's size.
    fn status_bar(&self) -> Div {
        let ui = self.ui;
        let item = |children: Vec<Div>| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(MINI_GAP_SM))
                .px(px(MINI_GAP_SM))
                .children(children)
        };
        let divider = || div().flex_none().w(px(MINI_RULE)).h(px(MINI_DIVIDER_H)).bg(ui.border);
        let meter = div()
            .flex_none()
            .w(px(MINI_METER_W))
            .h(px(MINI_METER_H))
            .rounded_full()
            .bg(ui.border)
            .overflow_hidden()
            .child(div().w(relative(CONTEXT_USED)).h_full().rounded_full().bg(ui.primary));
        div()
            .flex_none()
            .h(px(MINI_STATUS_H))
            .flex()
            .items_center()
            .gap(px(MINI_GAP_SM))
            .px(px(MINI_TERM_PAD))
            .bg(ui.panel)
            .border_t(px(MINI_RULE))
            .border_color(ui.border)
            .child(item(vec![
                self.icon(IconName::GitBranch, MINI_GLYPH, ui.branch),
                self.caption("ui-unify", ui.branch).font_weight(FontWeight::MEDIUM),
                self.caption("±3", ui.muted),
            ]))
            .child(divider())
            .child(item(vec![self.ring(ui.agent_working), self.caption("Claude working · 4m", ui.muted)]))
            .child(item(vec![
                self.icon(IconName::CircleAlert, MINI_GLYPH, ui.agent_attention),
                self.caption("1 needs you", ui.agent_attention),
            ]))
            .child(item(vec![
                self.icon(IconName::CircleCheck, MINI_GLYPH, ui.agent_done),
                self.caption("1 done", ui.muted),
            ]))
            .child(div().flex_1())
            .child(item(vec![
                self.caption("Context", ui.muted),
                meter,
                self.caption("62%", ui.text).font_weight(FontWeight::SEMIBOLD),
            ]))
            .child(divider())
            .child(item(vec![self.caption("zsh · 90×40", ui.muted)]))
    }
}
