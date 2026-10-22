//! The three font settings by slot (terminal, interface, code), and the fonts they come to with a
//! theme's metrics. What a slot is, its size range and the bundled font are `crate::theme::fonts`; the
//! values chosen are settings, so reading and writing them is here.

use crate::theme::fonts::{DEFAULT_UI_FONT_SIZE, FontSlot, ResolvedFonts, SYSTEM_FONT};
use crate::theme::tokens::Metrics;

use super::Settings;

impl Settings {
    /// A slot's family as stored (empty = inherit).
    pub fn slot_family(&self, slot: FontSlot) -> &str {
        match slot {
            FontSlot::Terminal => &self.font_family,
            FontSlot::Interface => &self.ui_font_family,
            FontSlot::Code => &self.code_font_family,
        }
    }

    pub fn set_slot_family(&mut self, slot: FontSlot, family: String) {
        match slot {
            FontSlot::Terminal if family.trim().is_empty() => {}
            FontSlot::Terminal => self.font_family = family,
            FontSlot::Interface => self.ui_font_family = family,
            FontSlot::Code => self.code_font_family = family,
        }
    }

    pub fn slot_size(&self, slot: FontSlot) -> f32 {
        match slot {
            FontSlot::Terminal => self.font_size,
            FontSlot::Interface => self.ui_font_size,
            FontSlot::Code => self.code_font_size,
        }
    }

    /// Change a slot's size by `delta` points, within the slot's range.
    pub fn step_slot_size(&mut self, slot: FontSlot, delta: f32) {
        let (min, max) = slot.size_range();
        let size = match slot {
            FontSlot::Terminal => &mut self.font_size,
            FontSlot::Interface => &mut self.ui_font_size,
            FontSlot::Code => &mut self.code_font_size,
        };
        *size = (*size + delta).clamp(min, max);
    }

    /// The fonts in effect. The user's choice wins over the theme: a non-empty family setting, or
    /// a size changed from the default. Otherwise the theme's metrics apply, then the system font
    /// (interface) or the terminal font (code).
    pub fn resolved_fonts(&self, metrics: &Metrics) -> ResolvedFonts {
        let non_empty = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_string());
        let ui_family = non_empty(&self.ui_font_family)
            .or_else(|| metrics.ui_font_family.as_deref().and_then(non_empty))
            .unwrap_or_else(|| SYSTEM_FONT.to_string());
        let ui_size =
            if self.ui_font_size != DEFAULT_UI_FONT_SIZE { self.ui_font_size } else { metrics.ui_font_size };
        let (min, max) = FontSlot::Interface.size_range();
        ResolvedFonts {
            terminal_family: self.font_family.clone(),
            terminal_size: self.font_size,
            ui_family,
            ui_size: ui_size.clamp(min, max),
            code_family: non_empty(&self.code_font_family).unwrap_or_else(|| self.font_family.clone()),
            code_size: self.code_font_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::theme::fonts::{FontSlot, SYSTEM_FONT, bundled};
    use crate::theme::tokens::Metrics;

    use super::Settings;

    #[test]
    fn empty_families_inherit() {
        let settings = Settings::default();
        let fonts = settings.resolved_fonts(&Metrics::default());
        assert_eq!(fonts.ui_family, SYSTEM_FONT);
        assert_eq!(fonts.code_family, settings.font_family);
        assert_eq!(fonts.ui_size, 13.0);
    }

    #[test]
    fn a_fresh_config_uses_the_bundled_font_for_terminal_and_code() {
        let fonts = Settings::default().resolved_fonts(&Metrics::default());
        assert_eq!(fonts.terminal_family, bundled::FAMILY);
        assert_eq!(fonts.code_family, bundled::FAMILY);
        assert_eq!(fonts.ui_family, SYSTEM_FONT);
    }

    #[test]
    fn theme_metrics_apply_unless_the_user_chose() {
        let metrics =
            Metrics { ui_font_family: Some("Inter".into()), ui_font_size: 14.0, ..Metrics::default() };
        let fonts = Settings::default().resolved_fonts(&metrics);
        assert_eq!((fonts.ui_family.as_str(), fonts.ui_size), ("Inter", 14.0));

        let settings = Settings {
            ui_font_family: "Helvetica Neue".into(),
            ui_font_size: 15.0,
            code_font_family: "SF Mono".into(),
            ..Settings::default()
        };
        let fonts = settings.resolved_fonts(&metrics);
        assert_eq!((fonts.ui_family.as_str(), fonts.ui_size), ("Helvetica Neue", 15.0));
        assert_eq!(fonts.code_family, "SF Mono");
    }

    #[test]
    fn slots_read_write_and_clamp() {
        let mut settings = Settings::default();
        settings.set_slot_family(FontSlot::Code, "Fira Code".into());
        settings.set_slot_family(FontSlot::Terminal, "  ".into());
        assert_eq!(settings.slot_family(FontSlot::Code), "Fira Code");
        assert_eq!(settings.slot_family(FontSlot::Terminal), Settings::default().font_family);
        settings.step_slot_size(FontSlot::Interface, 100.0);
        assert_eq!(settings.slot_size(FontSlot::Interface), 20.0);
        settings.step_slot_size(FontSlot::Code, -100.0);
        assert_eq!(settings.slot_size(FontSlot::Code), 8.0);
    }
}
