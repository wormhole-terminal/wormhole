//! What a colour picker keeps between frames: the working colour of its editor, the hex field and the
//! places of the area and the hue slider, which pointer events are read against.
//!
//! The editor works in hue, saturation and value ([`Hsv`]) and not in the colour it makes: a hex code
//! can't remember a hue once the colour is grey or black, and a drag that passes through one would
//! otherwise snap the slider back to red. The setting keeps only the hex, so the state follows the
//! setting when something else changes it (a reset, `config.json`) and ignores a change it made itself.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use wormhole_core::theme::tokens::{Color, Hsv};

use crate::theme_bridge;

/// Where an element was laid out last frame, written as it is drawn and read by the pointer.
pub(super) type Place = Rc<Cell<Bounds<Pixels>>>;

pub struct ColorPickState {
    /// The editor's colour.
    pub(super) hsv: Hsv,
    /// The editor is shown under the presets.
    pub(super) editing: bool,
    /// The line the keyboard is on in the list, `None` until it moves: the current value's is it then.
    pub(super) highlight: Option<usize>,
    /// What was typed is not a colour: said under the field until the next key.
    pub(super) bad_hex: bool,
    pub(super) hex: Entity<InputState>,
    pub(super) trigger: FocusHandle,
    pub(super) area_focus: FocusHandle,
    pub(super) hue_focus: FocusHandle,
    pub(super) area: Place,
    pub(super) hue: Place,
    /// The colour in force the last time the state looked, as the bytes of its hex code.
    seen: Option<[u8; 4]>,
    /// Hears the field change, to take back the message that what was typed is not a colour.
    _typing: Subscription,
}

impl ColorPickState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hex = cx.new(|cx| InputState::new(window, cx).placeholder("#a35cf2")); // text: an example code
        // The field's own keys (a backspace) never reach the control, so the message goes with the text.
        let typing = cx.subscribe_in(&hex, window, |state, _, event: &InputEvent, window, _| {
            if matches!(event, InputEvent::Change) && state.bad_hex {
                state.bad_hex = false;
                window.refresh();
            }
        });
        Self {
            hsv: Hsv::new(0.0, 0.0, 0.0),
            editing: false,
            highlight: None,
            bad_hex: false,
            hex,
            trigger: cx.focus_handle().tab_stop(true),
            area_focus: cx.focus_handle().tab_stop(true),
            hue_focus: cx.focus_handle().tab_stop(true),
            area: Place::default(),
            hue: Place::default(),
            seen: None,
            _typing: typing,
        }
    }

    /// Follow the colour in force: when it is not the one this state last wrote or saw, the editor starts
    /// from it. Called on every frame the window draws, so it compares bytes and builds nothing.
    pub fn follow(&mut self, color: Color, window: &mut Window, cx: &mut Context<Self>) {
        let color = color.with_alpha(1.0);
        let bytes = color.to_bytes();
        if self.seen == Some(bytes) {
            return;
        }
        self.seen = Some(bytes);
        self.hsv = Hsv::from_color(color);
        self.bad_hex = false;
        let hex = color.to_hex();
        self.hex.update(cx, |field, cx| field.set_value(hex, window, cx));
    }

    /// Make `hsv` the working colour as a pointer or a key moved it; the hex code it makes, for the setting.
    /// The field shows the code, unless it is the field being typed in.
    pub(super) fn set(&mut self, hsv: Hsv, window: &mut Window, cx: &mut Context<Self>) -> String {
        let color = hsv.to_color();
        self.hsv = hsv;
        self.bad_hex = false;
        self.seen = Some(color.to_bytes());
        let hex = color.to_hex();
        if !self.hex.read(cx).focus_handle(cx).is_focused(window) {
            let shown = hex.clone();
            self.hex.update(cx, |field, cx| field.set_value(shown, window, cx));
        }
        hex
    }

    /// The colour typed in the field: `Some` (and the editor is moved to it) when it is a colour, else the
    /// field is marked and nothing changes.
    pub(super) fn typed(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<String> {
        let typed = self.hex.read(cx).value().to_string();
        match Color::from_hex(&typed) {
            Ok(color) => {
                let hsv = Hsv::from_color(color);
                self.hsv = hsv;
                self.bad_hex = false;
                // The field keeps what was typed until it is left; the setting gets the code in full.
                self.seen = Some(hsv.to_color().to_bytes());
                Some(color.with_alpha(1.0).to_hex())
            }
            Err(_) => {
                self.bad_hex = true;
                window.refresh();
                None
            }
        }
    }

    /// The colour the editor shows, as the face of a swatch.
    pub(super) fn shown(&self) -> Hsla {
        theme_bridge::hsla(self.hsv.to_color())
    }
}

#[cfg(test)]
impl ColorPickState {
    /// Whether the field is saying that what was typed is not a colour.
    pub fn says_bad(&self) -> bool {
        self.bad_hex
    }
}
