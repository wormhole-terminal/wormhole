//! The palette's view: 600 wide, 72 below the window's top, on the scrim; the search row,
//! the scope row, the results (a virtual list up to 560 in all), and the footer's keys.

use std::time::Instant;

use gpui_kit::component::input::Input;
use wormhole_core::keymap::ActionId;
use wormhole_core::search::Scope;
use wormhole_core::settings_index;

use super::super::overlay::scrim;
use super::rows::{self, Item, Row, Snapshot};
use super::*;

/// How far below the window's top the palette opens, and the most it grows to before its list
/// scrolls.
const TOP: f32 = 72.0;
const MAX_H: f32 = 560.0;
/// The least room left under it, and beside it in a narrow window.
const MARGIN: f32 = 24.0;
const SIDE_MARGIN: f32 = 16.0;
/// Room under the last result.
const LIST_BOTTOM: f32 = 8.0;

/// A row's height, for the list's.
fn row_height(snapshot: &Snapshot, scope: Scope, row: &Row) -> f32 {
    match row {
        Row::Label { .. } => style::ROW_H,
        Row::Result { item: Item::Terminal(tab), .. } if scope == Scope::Terminals => {
            let asks = snapshot.terminal(*tab).and_then(rows::question).is_some();
            if asks { style::ROW_TWO_LINE_H } else { style::QP_ROW_H }
        }
        Row::Result { .. } | Row::Note(_) => style::QP_ROW_H,
    }
}

/// What a row's drawing needs, captured for the list (which draws only the rows in view).
#[derive(Clone)]
struct RowEnv {
    ui: UiColors,
    rows: Rc<Vec<Row>>,
    snapshot: Rc<Snapshot>,
    scope: Scope,
    selected: Option<usize>,
    code_font: SharedString,
    palette: WeakEntity<CommandPalette>,
    now: Instant,
}

fn action_icon(id: ActionId) -> IconName {
    use ActionId::*;
    match id {
        NewTerminal | NewWindow | NewNote => IconName::Plus,
        OpenFolder => IconName::FolderOpen,
        ShowChanges => IconName::GitCompare,
        ShowHistory => IconName::GitCommitHorizontal,
        ShowNotes => IconName::StickyNote,
        ShowProject => IconName::Folder,
        ShowUsage => IconName::Gauge,
        ToggleSidebar => IconName::PanelLeft,
        ToggleRightPanel => IconName::PanelRight,
        StashPrompt => IconName::Inbox,
        KeyboardShortcuts => IconName::Keyboard,
        ThemeStudio => IconName::Palette,
        Settings | CheckForUpdates => IconName::Settings,
        CloseTerminal | CloseWindow => IconName::X,
        PinTerminal => IconName::Pin,
        CopyPath => IconName::Copy,
        RevealInFinder => IconName::ExternalLink,
        _ => IconName::Command,
    }
}

/// A setting's value as its row shows it at the right (every value, "80 %" included): a
/// name, on / off, or a number as the page writes it.
fn setting_value(id: &str, cx: &App) -> Option<String> {
    let (_, _, entry) = settings_index::entry(id)?;
    let key = entry.keys.first()?;
    let value = serde_json::to_value(settings(cx)).ok()?.get(*key)?.clone();
    value_text(key, &value)
}

/// `value` of the config key `key` in the words of its settings row.
fn value_text(key: &str, value: &serde_json::Value) -> Option<String> {
    match value {
        // A choice's config word reads as a name ("system" → "System"); a theme's name is one.
        serde_json::Value::String(text) if !text.is_empty() => {
            let mut chars = text.chars();
            let first: String = chars.next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
            Some(first + chars.as_str())
        }
        serde_json::Value::Bool(on) => Some(if *on { "On" } else { "Off" }.to_string()),
        // The opacity is a share, written as its slider's percentage.
        serde_json::Value::Number(n) if key == "opacity" => {
            Some(format!("{} %", (n.as_f64()? * 100.0).round() as i64))
        }
        serde_json::Value::Number(n) => match n.as_u64() {
            Some(whole) => Some(wormhole_core::limits::count_grouped(whole as usize)),
            // A fraction as the page writes it: the line height's 1.2, a size's 13.5.
            None => Some(format!("{}", (n.as_f64()? * 100.0).round() / 100.0)),
        },
        _ => None,
    }
}

fn render_row(ix: usize, row: &Row, env: &RowEnv, cx: &mut App) -> AnyElement {
    let ui = env.ui;
    let (item, marks) = match row {
        Row::Label { title, count, trusted } => {
            return style::palette_label(ui, title.clone(), *count)
                .when(*trusted, |d| d.child(style::status_badge(ui, ui.success_text, "Trusted")))
                .into_any_element();
        }
        Row::Note(text) => {
            return div()
                .flex()
                .items_center()
                .h(px(style::QP_ROW_H))
                .px_4()
                .text_body()
                .text_color(ui.muted)
                .child(*text)
                .into_any_element();
        }
        Row::Result { item, marks } => (item, marks),
    };
    let selected = env.selected == Some(ix);
    let icon = |name: IconName| style::icon(name).into_any_element();
    let keycap = |text: String| style::keycap(ui, text).into_any_element();
    let snapshot = &env.snapshot;
    let mut parts =
        match item {
            Item::Terminal(tab) => {
                let Some(terminal) = snapshot.terminal(*tab) else { return div().into_any_element() };
                let glyph = super::super::quick_panel::glyph_of(&terminal.terminal);
                let mut parts = style::PaletteResult::new(
                    style::status_glyph(("palette-glyph", *tab as u64), ui, glyph, cx),
                    terminal.terminal.name.clone(),
                );
                parts.strong = true;
                let scoped = env.scope == Scope::Terminals;
                parts.detail = Some(rows::terminal_detail(terminal, scoped, env.now).into())
                    .filter(|d: &SharedString| !d.is_empty());
                if scoped {
                    parts.second = rows::question(terminal).map(SharedString::from);
                }
                parts.trailing.extend(terminal.number.map(|n| keycap(format!("⌘{n}"))));
                parts
            }
            Item::Folder(path) => {
                let Some(folder) = snapshot.folder(path) else { return div().into_any_element() };
                let mut parts = style::PaletteResult::new(icon(IconName::Folder), folder.name.clone());
                parts.detail = Some(match folder.terminals {
                    0 => folder.parent.clone().into(),
                    1 => format!("{} · 1 terminal", folder.parent).into(),
                    n => format!("{} · {n} terminals", folder.parent).into(),
                });
                parts
            }
            Item::Commit(sha) => {
                let Some(commit) = snapshot.commit(sha) else { return div().into_any_element() };
                let mut parts =
                    style::PaletteResult::new(icon(IconName::GitCommitHorizontal), commit.subject.clone());
                // The short hash at the right, no age.
                parts.trailing.push(
                    div()
                        .text_caption()
                        .font_family(env.code_font.clone())
                        .text_color(if selected { ui.list_selected_muted } else { ui.muted })
                        .child(commit.short.clone())
                        .into_any_element(),
                );
                parts
            }
            Item::Setting(id) => {
                let Some((section, _, entry)) = settings_index::entry(id) else {
                    return div().into_any_element();
                };
                let mut parts =
                    style::PaletteResult::new(icon(settings_page::section_icon(section)), entry.label);
                parts.detail = Some(format!("Settings › {}", section.title()).into());
                let value_color = if selected { ui.list_selected_muted } else { ui.muted };
                parts.trailing.extend(setting_value(id, cx).map(|value| {
                    div().text_caption().text_color(value_color).child(value).into_any_element()
                }));
                if selected {
                    parts.trailing.push(keycap("↩".into()));
                }
                parts
            }
            Item::Action(id) => {
                let Some(action) = snapshot.action(*id) else { return div().into_any_element() };
                let mut parts = style::PaletteResult::new(icon(action_icon(*id)), action.label.clone());
                parts.detail = match id {
                    ActionId::NewTerminal => snapshot.cwd.as_ref().map(|cwd| format!("in {cwd}").into()),
                    ActionId::OpenFolder => Some("start a terminal in another folder".into()),
                    ActionId::ShowChanges => snapshot.changes.map(|n| match n {
                        1 => "1 file changed".into(),
                        n => format!("{n} files changed").into(),
                    }),
                    _ => None,
                };
                parts.trailing.extend(action.key.as_ref().map(|k| keycap(k.glyphs())));
                parts
            }
            Item::ProjectAction(id) => {
                let Some(action) = snapshot.project_action(id) else { return div().into_any_element() };
                let mut parts = style::PaletteResult::new(icon(IconName::Zap), action.title.clone());
                parts.detail = action.command.clone().map(SharedString::from);
                parts.detail_font = Some(env.code_font.clone());
                parts.trailing.extend(action.key.clone().map(keycap));
                parts
            }
            // A terminal with another shell: named like New terminal, with what it runs.
            Item::NewTerminalWith(key) => {
                let Some(label) = snapshot.shell(key).and_then(|choice| item.shell_label(choice)) else {
                    return div().into_any_element();
                };
                let mut parts = style::PaletteResult::new(icon(IconName::Plus), label);
                parts.detail = snapshot.shell(key).map(|choice| choice.command_line.clone().into());
                parts.detail_font = Some(env.code_font.clone());
                parts
            }
        };
    parts.marks = marks.clone();
    let (item, palette) = (item.clone(), env.palette.clone());
    style::palette_result(("palette-row", ix), ui, selected, parts)
        .on_click(move |_, window, cx| {
            let item = item.clone();
            let _ = palette.update(cx, |palette, cx| {
                palette.selected = Some(item.clone());
                palette.run(item, window, cx);
            });
        })
        .into_any_element()
}

impl CommandPalette {
    /// The scope row: All and the kinds, each with its prefix, in the app's one segmented
    /// control (as Settings and the filter popover have it); the chosen one is solid.
    fn render_scopes(&self, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(style::QP_ROW_H))
            .px_3()
            .rule_bottom(ui)
            .child(style::segmented(ui).children(Scope::ROW.into_iter().enumerate().map(|(ix, scope)| {
                let active = scope == self.scope;
                // The prefix leads the name.
                let label = match scope.prefix() {
                    Some(prefix) => format!("{prefix} {}", scope.title()),
                    None => scope.title().to_string(),
                };
                style::segment(("palette-scope", ix), ui, label, active)
                    .on_click(cx.listener(move |this, _, window, cx| this.set_scope(scope, window, cx)))
            })))
            .into_any_element()
    }

    /// The footer: the keys that work on what is selected, and the count (or a hint) at the right.
    fn render_footer(&self, ui: UiColors, code_font: SharedString, cx: &App) -> AnyElement {
        let hints: &[(&str, &str)] = match (self.scope, self.selected_item()) {
            (Scope::Terminals, _) => &[("↩", "switch"), ("⌘⌫", "close terminal")],
            (Scope::Actions, _) => &[("↩", "run"), ("⌫", "back to all")],
            (_, Some(Item::Setting(_))) => {
                &[("↑↓", "move"), ("↩", "open in Settings"), ("⌘C", "copy setting key")]
            }
            (_, Some(Item::Terminal(_) | Item::Folder(_) | Item::Commit(_))) => {
                &[("↑↓", "move"), ("↩", "open"), ("⌘↩", "open in new window"), ("tab", "scope")]
            }
            _ => &[("↑↓", "move"), ("↩", "open"), ("tab", "scope")],
        };
        let query = self.query(cx);
        let right = if !query.trim().is_empty() {
            let n = rows::result_count(&self.rows);
            div()
                .child(if n == 1 { "1 result".to_string() } else { format!("{n} results") })
                .into_any_element()
        } else if self.scope == Scope::All {
            div()
                .flex()
                .gap_1()
                .child("Type")
                .child(div().font_family(code_font).text_color(ui.text).child(">"))
                .child("for actions")
                .into_any_element()
        } else {
            div().into_any_element()
        };
        style::palette_footer(ui)
            .children(hints.iter().map(|(keys, label)| style::key_hint(ui, *keys, *label)))
            .child(div().flex_1())
            .child(right)
            .into_any_element()
    }
}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let (code_font, _) = crate::fonts::code_font(cx);
        let viewport = window.viewport_size();
        let width = style::QUICK_PANEL_W.min(f32::from(viewport.width) - 2.0 * SIDE_MARGIN);
        let max_h = MAX_H.min(f32::from(viewport.height) - TOP - MARGIN);
        let chrome = style::QP_SEARCH_H + 2.0 * style::QP_ROW_H;
        let content: f32 =
            self.rows.iter().map(|row| row_height(&self.snapshot, self.scope, row)).sum::<f32>()
                + LIST_BOTTOM;
        let list_h = content.min((max_h - chrome).max(style::QP_ROW_H));
        let env = RowEnv {
            ui,
            rows: self.rows.clone(),
            snapshot: self.snapshot.clone(),
            scope: self.scope,
            selected: self.selected_ix(),
            code_font: code_font.clone(),
            palette: cx.weak_entity(),
            now: Instant::now(),
        };
        let list = list(self.list.clone(), move |ix, _, cx| match env.rows.get(ix) {
            Some(row) => div().w_full().child(render_row(ix, row, &env, cx)).into_any_element(),
            None => div().into_any_element(),
        })
        .size_full();
        let card = style::palette_surface(ui, true)
            .id("command-palette")
            .w(px(width))
            .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close(window, cx)))
            .child(
                style::palette_search(ui)
                    .when(self.scope != Scope::All, |d| {
                        d.child(style::tint_chip(ui, None, self.scope.title()))
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_heading()
                            .child(Input::new(&self.search).appearance(false)),
                    )
                    .child(style::keycap(ui, "esc")),
            )
            .child(self.render_scopes(ui, cx))
            .child(div().flex().flex_col().h(px(list_h)).pb(px(LIST_BOTTOM)).child(list))
            .child(self.render_footer(ui, code_font, cx));
        scrim("palette-scrim", cx)
            .child(div().absolute().top(px(TOP)).left_0().right_0().flex().justify_center().child(card))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::value_text;

    /// Every setting shows its value, the opacity as its slider's percentage,
    /// counts grouped, fractions as the page writes them, choices and switches as words.
    #[test]
    fn a_settings_value_reads_as_its_row_writes_it() {
        assert_eq!(value_text("opacity", &json!(0.8)).as_deref(), Some("80 %"));
        assert_eq!(value_text("opacity", &json!(1.0)).as_deref(), Some("100 %"));
        assert_eq!(value_text("font_size", &json!(13.0)).as_deref(), Some("13"));
        assert_eq!(value_text("font_size", &json!(13.5)).as_deref(), Some("13.5"));
        assert_eq!(value_text("line_height", &json!(1.2)).as_deref(), Some("1.2"));
        assert_eq!(value_text("scrollback_lines", &json!(10000)).as_deref(), Some("10,000"));
        assert_eq!(value_text("tab_width", &json!(4)).as_deref(), Some("4"));
        assert_eq!(value_text("theme_mode", &json!("system")).as_deref(), Some("System"));
        assert_eq!(value_text("blur", &json!(true)).as_deref(), Some("On"));
        assert_eq!(value_text("hotkey", &json!("")), None, "nothing set says nothing");
        assert_eq!(value_text("x", &json!(["a"])), None);
    }
}
