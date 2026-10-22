//! What a popover is, whoever draws it: a column of rows under an element, which a person reads
//! and acts in: a line of text, a text field, a row of buttons, a row of colour dots, a list of
//! actions like a menu's. No AppKit and no widget is named here, so the same value is an
//! `NSPopover` of native controls on macOS (`macos::popover`), GPUI's own card elsewhere
//! (`style::popover_layer`, handed over as `Drawn`), and plain text in tests and devctl's `popover` command.
//!
//! The model lives in the app, not in `wormhole-core`: pressing a button runs a closure on GPUI's
//! `Window` and `App`.
//!
//! A popover is not modal, unlike a menu: AppKit calls the controls' actions from its own event
//! dispatch, while GPUI may have its `App` borrowed. So no closure here is ever run from a control's
//! action. The platform puts what happened (a press, a text, a colour) in a queue, and a foreground
//! task of GPUI takes it from there and runs the closure in an update of the popover's window with
//! nothing borrowed. The same goes for opening and closing, which make AppKit change the key
//! window and so call GPUI back.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::theme::tokens::Appearance;

use super::menu::{Callback, Item, Swatches};

/// What typing in a field ended with.
pub type Commit = Rc<dyn Fn(&str, &mut Window, &mut App)>;
/// A colour chosen outside the dots (the system's colour panel), as sRGB.
pub type ColourChange = Rc<dyn Fn(Hsla, &mut Window, &mut App)>;
/// The pointer came onto or left the popover itself.
pub type Hover = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// How it goes away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    /// Open until a click outside it, Esc, an action or [`Handle::close`]: a card the person asked
    /// for.
    Transient,
    /// Opened by the pointer resting on something, closed by the app when the pointer has left both:
    /// a click elsewhere doesn't close it, and it doesn't take the keyboard.
    Hover,
}

/// A small picture on a button: the platform draws its own for the word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Folder,
    Copy,
    Check,
    Terminal,
}

/// What a button says for a while after it was pressed ("Copied"), as the old card did.
#[derive(Clone)]
pub struct Done {
    pub title: SharedString,
    pub glyph: Option<Glyph>,
    pub for_: Duration,
}

#[derive(Clone)]
pub struct Button {
    pub title: SharedString,
    pub glyph: Option<Glyph>,
    pub enabled: bool,
    /// Why it is off, for the tooltip.
    pub hint: Option<SharedString>,
    pub on_press: Callback,
    /// The popover closes before `on_press` runs.
    pub closes: bool,
    pub done: Option<Done>,
}

impl Button {
    pub fn new(
        title: impl Into<SharedString>,
        glyph: Option<Glyph>,
        run: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            title: title.into(),
            glyph,
            enabled: true,
            hint: None,
            on_press: Rc::new(run),
            closes: true,
            done: None,
        }
    }

    pub fn off(mut self, hint: impl Into<SharedString>) -> Self {
        self.enabled = false;
        self.hint = Some(hint.into());
        self
    }

    pub fn enabled(self, enabled: bool, hint: &'static str) -> Self {
        if !enabled {
            return self.off(hint);
        }
        self
    }

    /// It stays open and reads `title` for `for_` after it is pressed.
    pub fn then_says(mut self, title: impl Into<SharedString>, glyph: Option<Glyph>, for_: Duration) -> Self {
        self.closes = false;
        self.done = Some(Done { title: title.into(), glyph, for_ });
        self
    }
}

/// A text field. ↩ keeps what was typed; ⎋ drops it; a click outside the popover keeps it, as
/// closing a popover over a field does.
#[derive(Clone)]
pub struct Field {
    pub value: SharedString,
    pub label: SharedString,
    /// The field has the keyboard, its text selected, as soon as the popover is up.
    pub focused: bool,
    pub on_commit: Commit,
    pub on_cancel: Option<Callback>,
}

/// The colour that is none of the dots', chosen in the system's colour panel.
#[derive(Clone)]
pub struct Custom {
    /// The group's own colour, when it has one: the last dot wears it and is ringed.
    pub color: Option<Hsla>,
    /// Where the panel starts when there is none.
    pub start: Hsla,
    pub on_change: ColourChange,
}

/// A row of colour dots, with a last one that opens the colour panel.
#[derive(Clone)]
pub struct Colours {
    pub swatches: Swatches,
    pub custom: Option<Custom>,
}

#[derive(Clone)]
pub enum Row {
    /// A path, wrapped where it has to be.
    Path(SharedString),
    Field(Field),
    Buttons(Vec<Button>),
    Colours(Colours),
    /// Rows like a menu's: each closes the popover and then runs.
    Actions(Vec<Item>),
    Separator,
}

#[derive(Clone)]
pub struct Popover {
    pub rows: Vec<Row>,
    pub behavior: Behavior,
    /// The theme's, so the popover is light or dark with the window and not with the system.
    pub appearance: Appearance,
    /// Runs once the popover has gone, however (after a field's commit).
    pub on_close: Option<Callback>,
    pub on_hover: Option<Hover>,
}

impl Popover {
    pub fn new(behavior: Behavior, appearance: Appearance) -> Self {
        Self { rows: Vec::new(), behavior, appearance, on_close: None, on_hover: None }
    }

    pub fn row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    pub fn on_close(mut self, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(run));
        self
    }

    pub fn on_hover(mut self, run: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(run));
        self
    }
}

/// An open popover, to close it from the app.
#[derive(Clone)]
pub struct Handle {
    close: Rc<dyn Fn(&mut App)>,
}

impl Handle {
    pub fn new(close: impl Fn(&mut App) + 'static) -> Self {
        Self { close: Rc::new(close) }
    }

    /// Close it. Its `on_close` runs afterwards, in an update of its own: not from inside this
    /// call, which may be in an update of the window.
    pub fn close(&self, cx: &mut App) {
        (self.close)(cx);
    }
}

// The text form is for devctl's `popover` and for tests.
#[cfg(any(test, feature = "devctl"))]
impl Popover {
    /// The rows as lines of text: `popover transient`, then `# path`, `field "value" (focused)`,
    /// `buttons: [Open in Finder] [Copy path]` (`(off)` for a disabled one, `->` what it says after
    /// it was pressed), `colours: Grey | Blue [x] | custom #rrggbb [x]`, `- action` and `---`.
    pub fn outline(&self) -> Vec<String> {
        let mut out = vec![format!(
            "popover {}",
            match self.behavior {
                Behavior::Transient => "transient",
                Behavior::Hover => "hover",
            }
        )];
        for row in &self.rows {
            match row {
                Row::Path(text) => out.push(format!("# {text}")),
                Row::Field(field) => out.push(format!(
                    "field {:?}{}",
                    field.value,
                    if field.focused { " (focused)" } else { "" }
                )),
                Row::Buttons(buttons) => {
                    let each: Vec<String> = buttons
                        .iter()
                        .map(|button| {
                            let off = if button.enabled { "" } else { " (off)" };
                            let done = button.done.as_ref().map(|d| format!(" -> {}", d.title));
                            format!("[{}{off}{}]", button.title, done.unwrap_or_default())
                        })
                        .collect();
                    out.push(format!("buttons: {}", each.join(" ")));
                }
                Row::Colours(colours) => {
                    let mut names: Vec<String> = colours
                        .swatches
                        .swatches
                        .iter()
                        .enumerate()
                        .map(|(ix, swatch)| {
                            let ring = if colours.swatches.selected == Some(ix) { " [x]" } else { "" };
                            format!("{}{ring}", swatch.name)
                        })
                        .collect();
                    if let Some(custom) = &colours.custom {
                        names.push(match custom.color {
                            Some(color) => format!("custom {} [x]", hex(color)),
                            None => "custom".to_string(),
                        });
                    }
                    out.push(format!("colours: {}", names.join(" | ")));
                }
                Row::Actions(items) => {
                    for item in items {
                        out.push(format!("- {}{}", item.title, if item.enabled { "" } else { " (off)" }));
                    }
                }
                Row::Separator => out.push("---".to_string()),
            }
        }
        out
    }
}

/// `#rrggbb` of a colour.
#[cfg(any(test, feature = "devctl"))]
pub fn hex(color: Hsla) -> String {
    let rgba = Rgba::from(color);
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", byte(rgba.r), byte(rgba.g), byte(rgba.b))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui_kit::hsla;
    use wormhole_core::theme::tokens::Appearance;

    use super::{Behavior, Button, Colours, Custom, Field, Glyph, Popover, Row, hex};
    use crate::platform::menu::{Item, Swatch, Swatches};

    fn swatches(selected: Option<usize>) -> Swatches {
        let swatch = |name: &str| Swatch { color: hsla(0.6, 0.8, 0.5, 1.0), name: name.to_string().into() };
        Swatches {
            swatches: vec![swatch("Grey"), swatch("Blue")],
            selected,
            on_pick: std::rc::Rc::new(|_, _, _| {}),
        }
    }

    #[test]
    fn the_outline_shows_every_row() {
        let popover = Popover::new(Behavior::Transient, Appearance::Dark)
            .row(Row::Field(Field {
                value: "Review".into(),
                label: "Name".into(),
                focused: true,
                on_commit: std::rc::Rc::new(|_, _, _| {}),
                on_cancel: None,
            }))
            .row(Row::Colours(Colours {
                swatches: swatches(Some(1)),
                custom: Some(Custom {
                    color: Some(hsla(0.0, 1.0, 0.5, 1.0)),
                    start: hsla(0.0, 0.0, 0.5, 1.0),
                    on_change: std::rc::Rc::new(|_, _, _| {}),
                }),
            }))
            .row(Row::Separator)
            .row(Row::Actions(vec![Item::new("Ungroup"), Item::new("Pin group").enabled(false)]));
        assert_eq!(
            popover.outline(),
            [
                "popover transient",
                "field \"Review\" (focused)",
                "colours: Grey | Blue [x] | custom #ff0000 [x]",
                "---",
                "- Ungroup",
                "- Pin group (off)"
            ]
        );
    }

    #[test]
    fn a_button_that_says_something_after_it_is_pressed_shows_it() {
        let popover = Popover::new(Behavior::Hover, Appearance::Light).row(Row::Buttons(vec![
            Button::new("Open in Finder", Some(Glyph::Folder), |_, _| {}),
            Button::new("Copy path", Some(Glyph::Copy), |_, _| {}).then_says(
                "Copied",
                Some(Glyph::Check),
                Duration::from_millis(1200),
            ),
            Button::new("Insert into terminal", Some(Glyph::Terminal), |_, _| {})
                .off("No terminal is running"),
        ]));
        assert_eq!(
            popover.outline(),
            ["popover hover", "buttons: [Open in Finder] [Copy path -> Copied] [Insert into terminal (off)]"]
        );
    }

    #[test]
    fn hex_is_the_srgb_bytes() {
        assert_eq!(hex(hsla(0.0, 1.0, 0.5, 1.0)), "#ff0000");
    }
}
