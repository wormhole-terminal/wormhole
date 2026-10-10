//! The find bar's view: its text field, with what it shows read from the terminal it
//! belongs to. The workspace draws it over the terminal (the terminal's own view is cached, and
//! a field inside it would not redraw as it is typed in).

use std::rc::Rc;

use gpui_kit::component::input::{Input, InputEvent, InputState};

use super::*;
use crate::style;

pub struct FindBar {
    terminal: WeakEntity<TerminalView>,
    input: Entity<InputState>,
    _input_events: Subscription,
}

impl FindBar {
    pub(super) fn new(
        terminal: WeakEntity<TerminalView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Find"));
        let events = cx.subscribe_in(&input, window, |this, input, event: &InputEvent, window, cx| {
            match event {
                InputEvent::Change => {
                    let text = input.read(cx).value().to_string();
                    let _ = this.terminal.update(cx, |terminal, cx| terminal.set_find_text(text, cx));
                }
                // ↩ the next match, ⇧↩ the previous.
                InputEvent::PressEnter { shift, .. } => {
                    let delta = if *shift { -1 } else { 1 };
                    let _ = this.terminal.update(cx, |terminal, cx| terminal.find_step(delta, window, cx));
                }
                // The field's outline follows the focus.
                InputEvent::Focus | InputEvent::Blur => cx.notify(),
            }
        });
        Self { terminal, input, _input_events: events }
    }

    /// Give the field the keyboard, its text selected to type over.
    pub(super) fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
    }

    pub(super) fn set_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.set_value(text.to_string(), window, cx));
    }

    /// Whether the field has the keyboard.
    pub fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.input.focus_handle(cx).is_focused(window)
    }
}

impl Render for FindBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(terminal) = self.terminal.upgrade() else { return div().into_any_element() };
        let status = terminal.read(cx).find_status();
        let ui = cx.global::<SettingsStore>().ui;
        let on_option = {
            let terminal = self.terminal.clone();
            Rc::new(move |option, _: &mut Window, cx: &mut App| {
                let _ = terminal.update(cx, |terminal, cx| terminal.toggle_find_option(option, cx));
            })
        };
        let on_step = {
            let terminal = self.terminal.clone();
            Rc::new(move |delta, window: &mut Window, cx: &mut App| {
                let _ = terminal.update(cx, |terminal, cx| terminal.find_step(delta, window, cx));
            })
        };
        let on_close = {
            let terminal = self.terminal.clone();
            Rc::new(move |(), window: &mut Window, cx: &mut App| {
                let _ = terminal.update(cx, |terminal, cx| terminal.close_find(window, cx));
            })
        };
        let field = Input::new(&self.input).appearance(false).into_any_element();
        style::find_bar(
            "find-bar",
            ui,
            style::FindBarParts {
                field,
                focused: self.is_focused(window, cx),
                count: status.count,
                invalid: status.invalid,
                options: status.options,
                on_option,
                on_step,
                on_close,
            },
        )
        .into_any_element()
    }
}
