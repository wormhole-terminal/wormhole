//! The quick panel's view: the search row, the rows, Reply in place, and the footer's key hints.

use std::time::Instant;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use wormhole_core::notify::{self, State, Terminal};

use super::super::*;
use super::rows::{Do, Row};
use super::{close, do_items, run, terminals_now};

/// The tallest the panel gets before its list scrolls.
const MAX_H: f32 = 560.0;
/// A quiet line ("Nothing needs you") is a row tall, like the rows it stands in for.
const NOTE_H: f32 = style::QP_ROW_H;
/// The answer region under a waiting row: the question, the reply field and its hints.
const ANSWER_H: f32 = 112.0;

/// The window's height for these rows: search, rows, footer (the window is the panel).
pub(super) fn height_for(rows: &[Row]) -> f32 {
    let list: f32 = rows
        .iter()
        .map(|row| match row {
            Row::Label { .. } => style::ROW_H,
            Row::Note(_) => NOTE_H,
            Row::Terminal(_) | Row::Do(_) => style::QP_ROW_H,
        })
        .sum();
    (style::QP_SEARCH_H + list + style::PANEL_LIST_TOP + style::QP_ROW_H).min(MAX_H)
}

/// Reply in place: the waiting terminal and what is being typed to it.
struct Answering {
    tab: i64,
    input: Entity<InputState>,
}

pub(crate) struct QuickPanel {
    search: Entity<InputState>,
    /// Every terminal and what Do offers, read outside the panel's frames (see `refresh`).
    terminals: Vec<Terminal>,
    items: Vec<Do>,
    /// The selected row, by what it is (rows come and go as terminals change).
    selected: Option<String>,
    answering: Option<Answering>,
    /// The window's height as last set.
    height: f32,
    _subscriptions: Vec<Subscription>,
    /// Reads the terminals again once a second, so times and states stay current while it is open.
    _clock: Task<()>,
}

/// What identifies a row across redraws.
fn key(row: &Row) -> Option<String> {
    match row {
        Row::Terminal(terminal) => Some(format!("tab:{}", terminal.tab)),
        Row::Do(item) => Some(format!("do:{}", item.title())),
        Row::Label { .. } | Row::Note(_) => None,
    }
}

impl QuickPanel {
    pub(super) fn new(height: f32, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search terminals and actions"));
        window.focus(&search.focus_handle(cx), cx);
        #[cfg(target_os = "macos")]
        super::macos::prepare(window);
        let window_id = window.window_handle().window_id();
        let panel = cx.weak_entity();
        // Ahead of the search field, which would take ↑ ↓ and ⏎ for itself.
        let keys = cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() != window_id {
                return;
            }
            let handled =
                panel.update(cx, |panel, cx| panel.on_key(&event.keystroke, window, cx)).unwrap_or(false);
            if handled {
                cx.stop_propagation();
            }
        });
        let subscriptions = vec![
            keys,
            cx.subscribe_in(&search, window, |this, _, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    this.selected = None;
                    cx.notify();
                }
            }),
            // A click anywhere else takes the keyboard away: the panel goes.
            cx.observe_window_activation(window, |_, window, cx| {
                if !window.is_window_active() {
                    close(cx);
                }
            }),
        ];
        let clock = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |this, cx| this.refresh(cx)).is_err() {
                    break;
                }
            }
        });
        let mut panel = Self {
            search,
            terminals: Vec::new(),
            items: Vec::new(),
            selected: None,
            answering: None,
            height,
            _subscriptions: subscriptions,
            _clock: clock,
        };
        panel.refresh(cx);
        panel
    }

    /// Read every window's terminals. Never from `render`: GPUI binds an entity read while a
    /// window draws to that window, so a workspace read in the panel's frame would lose its own
    /// window when the panel closes (and its tick with it).
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.terminals = terminals_now(cx);
        self.items = do_items(cx);
        cx.notify();
    }

    fn rows(&self, cx: &App) -> Vec<Row> {
        super::rows(&self.terminals, &self.items, &self.search.read(cx).value())
    }

    /// The selected row's index: the one chosen, else the first that can be.
    fn selected_ix(&self, rows: &[Row]) -> Option<usize> {
        let chosen = self
            .selected
            .as_ref()
            .and_then(|wanted| rows.iter().position(|row| key(row).as_ref() == Some(wanted)));
        chosen.or_else(|| rows.iter().position(Row::selectable))
    }

    fn step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let choices: Vec<usize> = (0..rows.len()).filter(|&ix| rows[ix].selectable()).collect();
        let Some(current) = self.selected_ix(&rows).and_then(|ix| choices.iter().position(|&c| c == ix))
        else {
            return;
        };
        let next = (current as isize + delta).clamp(0, choices.len() as isize - 1) as usize;
        self.selected = key(&rows[choices[next]]);
        cx.notify();
    }

    /// The panel's keys; `true` when one was used.
    fn on_key(&mut self, keystroke: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let m = &keystroke.modifiers;
        let plain = !m.modified();
        match keystroke.key.as_str() {
            "escape" if self.answering.is_some() => self.stop_answering(window, cx),
            "escape" => close(cx),
            "up" if plain && self.answering.is_none() => self.step(-1, cx),
            "down" if plain && self.answering.is_none() => self.step(1, cx),
            "enter" if m.platform => match &self.answering {
                // ⌘⏎ while answering: open the terminal instead.
                Some(answering) => self.open_tab(answering.tab, cx),
                None => self.start_answering(window, cx),
            },
            "enter" if plain => match &self.answering {
                Some(_) => self.send_reply(window, cx),
                None => {
                    let rows = self.rows(cx);
                    if let Some(row) = self.selected_ix(&rows).map(|ix| &rows[ix]) {
                        run(row, cx);
                    }
                }
            },
            _ => return false,
        }
        true
    }

    fn open_tab(&self, tab: i64, cx: &mut App) {
        close(cx);
        cx.defer(move |cx| {
            super::super::menu_bar::run_command(crate::menubar::MenuCommand::OpenTab(tab), cx)
        });
    }

    /// ⌘⏎ on a waiting row: Reply in place.
    fn start_answering(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let Some(terminal) = self.selected_ix(&rows).and_then(|ix| rows[ix].waiting()) else { return };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Reply"));
        window.focus(&input.focus_handle(cx), cx);
        self.answering = Some(Answering { tab: terminal.tab, input });
        cx.notify();
    }

    fn stop_answering(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.answering = None;
        window.focus(&self.search.focus_handle(cx), cx);
        cx.notify();
    }

    /// ⏎ in Reply: type the answer into the terminal and press Return, then go on to the next
    /// terminal that waits.
    fn send_reply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(answering) = &self.answering else { return };
        let (tab, text) = (answering.tab, answering.input.read(cx).value().to_string());
        if text.trim().is_empty() {
            return;
        }
        // After this update: the terminal's window is updated, never inside the panel's.
        cx.defer(move |cx| super::super::notifications::reply(tab, &text, cx));
        let rows = self.rows(cx);
        let waiting: Vec<i64> = rows.iter().filter_map(Row::waiting).map(|t| t.tab).collect();
        let next = waiting.iter().skip_while(|&&t| t != tab).nth(1).or_else(|| waiting.first()).copied();
        self.selected = next.filter(|&t| t != tab).map(|t| format!("tab:{t}"));
        self.stop_answering(window, cx);
    }

    /// Keep the window as tall as what it shows, its top where it is.
    fn fit(&mut self, rows: &[Row], window: &mut Window, cx: &mut Context<Self>) {
        let extra = if self.answering.is_some() { ANSWER_H } else { 0.0 };
        let height = (height_for(rows) + extra).min(MAX_H);
        if (height - self.height).abs() < 0.5 {
            return;
        }
        self.height = height;
        #[cfg(target_os = "macos")]
        if let Some(native) = crate::platform::macos::appkit::ns_window(window) {
            cx.spawn(async move |_, _| super::macos::set_height_keeping_top(&native, height)).detach();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = cx;
            window.resize(size(px(style::QUICK_PANEL_W), px(height)));
        }
    }

    fn render_row(&self, ix: usize, row: &Row, selected: bool, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        match row {
            Row::Label { title, count } => style::palette_label(ui, *title, *count).into_any_element(),
            Row::Note(text) => div()
                .flex()
                .items_center()
                .h(px(NOTE_H))
                .px_4()
                .text_body()
                .text_color(ui.muted)
                .child(*text)
                .into_any_element(),
            Row::Terminal(terminal) => self.terminal_row(ix, terminal, selected, cx),
            Row::Do(item) => self.do_row(ix, item, selected, cx),
        }
    }

    fn terminal_row(
        &self,
        ix: usize,
        terminal: &Terminal,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let glyph = super::glyph_of(terminal);
        let detail_color = match terminal.state {
            State::NeedsYou => ui.attention_text,
            State::Done => ui.done_text,
            State::Failed => ui.danger_text,
            _ if selected => ui.list_selected_muted,
            _ => ui.muted,
        };
        // A command line, running or failed, reads in the code font; a program's words don't.
        let plain_command =
            !terminal.source.told() && matches!(terminal.state, State::Working | State::Failed);
        let (code_font, _) = crate::fonts::code_font(cx);
        let elapsed =
            notify::elapsed(terminal.state, Instant::now().saturating_duration_since(terminal.since));
        let row = Row::Terminal(terminal.clone());
        let tab = terminal.tab;
        let answering = self.answering.as_ref().filter(|a| a.tab == tab);
        let mut parts = style::PaletteResult::new(
            style::status_glyph(("qp-glyph", tab as u64), ui, glyph, cx).into_any_element(),
            terminal.name.clone(),
        );
        parts.strong = true;
        parts.detail = terminal.detail(false).map(SharedString::from);
        parts.detail_color = Some(detail_color);
        parts.detail_font = plain_command.then_some(code_font);
        parts.trailing.extend(elapsed.map(|time| {
            div().text_caption().tabular_nums().text_color(ui.faint).child(time).into_any_element()
        }));
        let line = style::palette_result(("qp-row", ix), ui, selected, parts).on_click(cx.listener(
            move |this, _, _, cx| {
                this.selected = key(&row);
                run(&row, cx);
            },
        ));
        match answering {
            Some(answering) => div()
                .flex()
                .flex_col()
                .child(line)
                .child(self.answer_region(terminal, answering, cx))
                .into_any_element(),
            None => line.into_any_element(),
        }
    }

    /// Under a waiting row while answering: what the agent asked, the reply field, its keys.
    fn answer_region(
        &self,
        terminal: &Terminal,
        answering: &Answering,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let question = terminal.message.clone().unwrap_or_else(|| "Waiting for your answer".into());
        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_3()
            .pl(px(style::ITEM_LEAD_W + style::PANEL_INSET + 8.0))
            .bg(ui.list_selected)
            .child(div().text_caption().text_color(ui.list_selected_muted).child(question))
            .child(
                div()
                    .px_2()
                    .rounded(ui.radius)
                    .outlined(ui, ui.border_strong)
                    .bg(ui.elevated)
                    .child(Input::new(&answering.input).appearance(false)),
            )
            .into_any_element()
    }

    fn do_row(&self, ix: usize, item: &Do, selected: bool, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let icon = match item {
            Do::NewTerminal { .. } => IconName::Plus,
            Do::OpenFolder => IconName::FolderOpen,
            Do::Action(_) => IconName::Play,
        };
        let row = Row::Do(item.clone());
        let mut parts = style::PaletteResult::new(style::icon(icon).into_any_element(), item.title());
        parts.strong = true;
        parts.detail = item.detail().map(SharedString::from);
        parts.trailing.extend(item.keys().map(|keys| style::keycap(ui, keys).into_any_element()));
        style::palette_result(("qp-row", ix), ui, selected, parts)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected = key(&row);
                run(&row, cx);
            }))
            .into_any_element()
    }

    fn footer(&self, cx: &App) -> AnyElement {
        let ui = ui(cx);
        let hints: &[(&str, &str)] = if self.answering.is_some() {
            &[("⏎", "send reply"), ("⌘⏎", "open terminal"), ("esc", "back")]
        } else {
            &[("↑↓", "select"), ("⏎", "open"), ("⌘⏎", "answer here"), ("esc", "close")]
        };
        style::palette_footer(ui)
            .children(hints.iter().map(|(keys, label)| style::key_hint(ui, *keys, *label)))
            .into_any_element()
    }
}

impl Render for QuickPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let rows = self.rows(cx);
        self.fit(&rows, window, cx);
        let selected = self.selected_ix(&rows);
        let hotkey = settings(cx).hotkey.parse::<wormhole_core::keyspec::KeySpec>().ok().map(|k| k.glyphs());
        let list: Vec<AnyElement> = rows
            .iter()
            .enumerate()
            .map(|(ix, row)| self.render_row(ix, row, Some(ix) == selected, cx))
            .collect();
        style::palette_surface(ui, false)
            .id("quick-panel")
            .size_full()
            .child(
                style::palette_search(ui)
                    .child(div().flex_1().min_w_0().child(Input::new(&self.search).appearance(false)))
                    .children(hotkey.map(|keys| style::keycap(ui, keys))),
            )
            .child(
                div()
                    .id("quick-panel-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pt(px(style::PANEL_LIST_TOP))
                    .children(list),
            )
            .child(self.footer(cx))
    }
}
