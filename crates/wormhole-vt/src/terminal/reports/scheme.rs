//! Light and dark: the host tells the engine its color scheme, and a program that asks (or that
//! asked to be told) hears it.
//!
//! Spec: Contour's "Color palette update notifications"
//! (<https://github.com/contour-terminal/contour/blob/master/docs/vt-extensions/color-palette-update-notifications.md>),
//! which other terminals have adopted. `CSI ? 996 n` asks;
//! the answer is `CSI ? 997 ; 1 n` for a dark scheme and `CSI ? 997 ; 2 n` for a light one.
//! `CSI ? 2031 h` asks to be told: the terminal then sends the same report, unasked, whenever the
//! scheme changes (and `CSI ? 2031 l` stops it). The spec does not say that enabling the mode
//! reports at once, so it does not: a program asks with 996 first.
//!
//! A terminal that has never been told its scheme does not answer 996: it has no truthful
//! answer, and a program has to cope with terminals that stay silent.

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;
use crate::types::ColorScheme;

impl Terminal {
    /// Tell the engine whether the host draws a light or a dark scheme, for programs that ask
    /// (`CSI ? 996 n`, answered `CSI ? 997 ; 1 n` dark, `2` light). While a program has mode 2031
    /// on, a change is reported to it at once, in [`replies`](Terminal::replies): write them out
    /// as you do after a feed. Telling the same scheme again changes nothing.
    pub fn set_color_scheme(&mut self, scheme: ColorScheme) {
        let changed = self.scheme != Some(scheme);
        self.scheme = Some(scheme);
        if changed && self.modes.contains(Modes::THEME_REPORT) {
            self.report_color_scheme();
        }
    }

    /// The scheme the host last told, `None` before it did.
    #[must_use]
    pub fn color_scheme(&self) -> Option<ColorScheme> {
        self.scheme
    }

    /// `CSI ? 997 ; Ps n`, when the scheme is known.
    pub(in crate::terminal) fn report_color_scheme(&mut self) {
        let ps = match self.scheme {
            Some(ColorScheme::Dark) => 1,
            Some(ColorScheme::Light) => 2,
            None => return,
        };
        self.reply(format_args!("\x1b[?997;{ps}n"));
    }
}
