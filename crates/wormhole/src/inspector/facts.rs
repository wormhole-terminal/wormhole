//! An element's style described in the design language's terms (`_2`, `radius_chip`,
//! `body`, a token's name instead of a hex value). Pure apart from the token-color
//! cache, so it is tested without a window.

use std::cell::RefCell;

use gpui_kit::*;
use wormhole_core::tokens::UI_TOKENS;

use crate::settings::{SettingsStore, UiColors};
use crate::theme_bridge::token;

/// The text roles as GPUI sizes them, smallest first (see `style::TextRole`).
pub(super) const TEXT_ROLES: [(&str, f32); 4] =
    [("caption", 0.75), ("body", 0.875), ("heading", 1.0), ("title", 1.25)];

/// An element's style, described with the design language's names.
#[derive(Clone, PartialEq)]
pub(super) struct Facts {
    pub(super) height: String,
    pub(super) min_height: String,
    pub(super) padding: String,
    pub(super) padding_x: String,
    pub(super) padding_y: String,
    pub(super) gap: String,
    pub(super) radius: String,
    pub(super) border: String,
    pub(super) text: String,
    pub(super) background: String,
    pub(super) text_color: String,
    pub(super) border_color: String,
}

impl Facts {
    pub(super) fn of(s: &StyleRefinement, ui: UiColors, rem: f32) -> Self {
        let p = &s.padding;
        let sides = |a: Option<DefiniteLength>, b: Option<DefiniteLength>| {
            if a == b { spacing(a) } else { format!("{} {}", spacing(a), spacing(b)) }
        };
        let gap = sides(s.gap.width, s.gap.height);
        let radius = s.corner_radii.top_left.map_or("–".into(), |r| radius_name(ui, abs_px(r, rem)));
        let border = s.border_widths.top.map_or("–".into(), |w| format!("{}px", abs_px(w, rem)));
        let text = s.text.font_size.map_or("inherited".into(), |size| {
            let r = abs_px(size, rem) / rem;
            TEXT_ROLES
                .iter()
                .find(|(_, role)| (role - r).abs() < 0.01)
                .map_or(format!("{:.1}px", r * rem), |(name, _)| (*name).into())
        });
        let fill = s.background.as_ref().and_then(|f| f.color()).and_then(|b| b.as_solid());
        Self {
            height: length(s.size.height),
            min_height: length(s.min_size.height),
            padding: format!(
                "{} {} {} {}",
                spacing(p.top),
                spacing(p.right),
                spacing(p.bottom),
                spacing(p.left)
            ),
            padding_x: sides(p.left, p.right),
            padding_y: sides(p.top, p.bottom),
            gap,
            radius,
            border,
            text,
            background: fill.map_or("–".into(), color_name),
            text_color: s.text.color.map_or("inherited".into(), color_name),
            border_color: s.border_color.map_or("–".into(), color_name),
        }
    }

    /// "gap _2 → _3" for every value that differs from `now`.
    pub(super) fn changes_to(&self, now: &Facts) -> Vec<String> {
        let pairs = [
            ("height", &self.height, &now.height),
            ("padding x", &self.padding_x, &now.padding_x),
            ("padding y", &self.padding_y, &now.padding_y),
            ("gap", &self.gap, &now.gap),
            ("radius", &self.radius, &now.radius),
            ("text", &self.text, &now.text),
        ];
        pairs.iter().filter(|(_, a, b)| a != b).map(|(name, a, b)| format!("{name} {a} → {b}")).collect()
    }
}

/// A spacing value as its grid step (`_2`), or plain pixels / percent.
fn spacing(len: Option<DefiniteLength>) -> String {
    match len {
        None => "–".into(),
        Some(DefiniteLength::Absolute(AbsoluteLength::Rems(r))) => {
            let steps = r.0 * 4.0;
            if steps.fract() == 0.0 {
                format!("_{steps:.0}")
            } else if (steps * 2.0).fract() == 0.0 {
                format!("_{:.0}p5", steps.trunc())
            } else {
                format!("{}rem", r.0)
            }
        }
        Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(p))) => format!("{}px", f32::from(p)),
        Some(DefiniteLength::Fraction(f)) => format!("{:.0}%", f * 100.0),
    }
}

fn length(len: Option<Length>) -> String {
    match len {
        None => "–".into(),
        Some(Length::Auto) => "auto".into(),
        Some(Length::Definite(d)) => spacing(Some(d)),
    }
}

pub(super) fn abs_px(len: AbsoluteLength, rem: f32) -> f32 {
    match len {
        AbsoluteLength::Pixels(p) => f32::from(p),
        AbsoluteLength::Rems(r) => r.0 * rem,
    }
}

fn radius_name(ui: UiColors, value: f32) -> String {
    let named = [
        ("radius_chip", ui.radius_chip),
        ("radius_sm", ui.radius_sm),
        ("radius", ui.radius),
        ("radius_lg", ui.radius_lg),
    ];
    // radius_chip equals radius_sm unless the theme sets it, so name every match.
    let names: Vec<&str> =
        named.iter().filter(|(_, r)| (f32::from(*r) - value).abs() < 0.01).map(|(n, _)| *n).collect();
    match names.as_slice() {
        [] if value >= 999.0 => "full".into(),
        [] => format!("{value}px"),
        names => format!("{} ({value}px)", names.join(" / ")),
    }
}

/// The token a color comes from, or its hex value.
fn color_name(color: Hsla) -> String {
    TOKEN_COLORS.with(|cache| color_name_in(color, &cache.borrow()))
}

/// [`color_name`] against a list of token colors: an exact match names the token, a
/// match ignoring alpha means the window opacity faded it.
fn color_name_in(color: Hsla, tokens: &[(&'static str, Rgba)]) -> String {
    let rgba = Rgba::from(color);
    let hex = format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        (rgba.r * 255.0).round() as u8,
        (rgba.g * 255.0).round() as u8,
        (rgba.b * 255.0).round() as u8,
        (rgba.a * 255.0).round() as u8
    );
    let exact = tokens.iter().find(|(_, c)| same(*c, rgba, true));
    let faded = tokens.iter().find(|(_, c)| same(*c, rgba, false));
    match (exact, faded) {
        (Some((name, _)), _) => format!("{name}  {hex}"),
        (None, Some((name, _))) => format!("{name} (window opacity)  {hex}"),
        (None, None) => hex,
    }
}

thread_local! {
    /// Every UI token's color, refreshed when the panel renders with new tokens.
    static TOKEN_COLORS: RefCell<Vec<(&'static str, Rgba)>> = const { RefCell::new(Vec::new()) };
    static TOKEN_REVISION: RefCell<u64> = const { RefCell::new(u64::MAX) };
}

fn same(a: Rgba, b: Rgba, with_alpha: bool) -> bool {
    let close = |x: f32, y: f32| (x - y).abs() < 1.5 / 255.0;
    close(a.r, b.r) && close(a.g, b.g) && close(a.b, b.b) && (!with_alpha || close(a.a, b.a))
}

/// Keep [`TOKEN_COLORS`] in step with the theme.
pub(super) fn sync_token_colors(cx: &App) {
    let store = cx.global::<SettingsStore>();
    if TOKEN_REVISION.with(|r| *r.borrow() == store.revisions.theme) {
        return;
    }
    let colors = UI_TOKENS.iter().map(|t| (t.name, Rgba::from(token(&store.tokens, t.name)))).collect();
    TOKEN_COLORS.with(|c| *c.borrow_mut() = colors);
    TOKEN_REVISION.with(|r| *r.borrow_mut() = store.revisions.theme);
}

#[cfg(test)]
mod tests {
    use gpui_kit::{AbsoluteLength, DefiniteLength, Rgba, StyleRefinement, px, rems};

    use super::{Facts, color_name_in, length, radius_name, spacing};
    use crate::settings::UiColors;

    /// A theme's interface colors, for the radius names.
    fn ui() -> UiColors {
        let tokens = wormhole_core::tokens::builtin_tokens();
        UiColors::new(&tokens[0], wormhole_core::tokens::Translucency { opacity: 1.0, blur: true })
    }

    fn rem_len(steps: f32) -> Option<DefiniteLength> {
        Some(DefiniteLength::Absolute(AbsoluteLength::Rems(rems(steps))))
    }

    #[test]
    fn spacing_is_named_by_its_grid_step() {
        assert_eq!(spacing(rem_len(0.5)), "_2");
        assert_eq!(spacing(rem_len(1.5)), "_6");
        assert_eq!(spacing(rem_len(0.125)), "_0p5");
        assert_eq!(spacing(rem_len(0.375)), "_1p5");
        // Not a multiple of half a step: plain rems.
        assert_eq!(spacing(rem_len(0.3)), "0.3rem");
        assert_eq!(spacing(Some(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(10.0))))), "10px");
        assert_eq!(spacing(Some(DefiniteLength::Fraction(0.5))), "50%");
        assert_eq!(spacing(None), "–");
    }

    #[test]
    fn lengths_say_auto_or_their_spacing() {
        assert_eq!(length(None), "–");
        assert_eq!(length(Some(gpui_kit::Length::Auto)), "auto");
        assert_eq!(length(Some(gpui_kit::Length::Definite(rem_len(1.0).unwrap()))), "_4");
    }

    #[test]
    fn radii_are_named_by_role_and_every_match_is_listed() {
        let ui = ui();
        let chip_and_sm = ui.radius_chip == ui.radius_sm;
        let name = radius_name(ui, f32::from(ui.radius_lg));
        assert!(name.starts_with("radius_lg"), "{name}");
        let name = radius_name(ui, f32::from(ui.radius_sm));
        assert_eq!(name.contains("radius_chip"), chip_and_sm, "{name}");
        assert!(name.contains("radius_sm"), "{name}");
        assert_eq!(radius_name(ui, 1234.0), "full");
        assert_eq!(radius_name(ui, 0.123), "0.123px");
    }

    #[test]
    fn a_nudge_is_reported_as_a_change() {
        let ui = ui();
        let mut before = StyleRefinement::default();
        before.padding.left = rem_len(0.5);
        before.padding.right = rem_len(0.5);
        let mut now = before.clone();
        now.padding.left = rem_len(0.75);
        now.padding.right = rem_len(0.75);
        let (before, now) = (Facts::of(&before, ui, 16.0), Facts::of(&now, ui, 16.0));
        assert_eq!(before.padding_x, "_2");
        assert_eq!(now.padding_x, "_3");
        assert_eq!(before.changes_to(&now), ["padding x _2 → _3"]);
        assert!(before.changes_to(&before).is_empty());
    }

    #[test]
    fn text_sizes_are_named_by_role() {
        let ui = ui();
        let mut style = StyleRefinement::default();
        assert_eq!(Facts::of(&style, ui, 16.0).text, "inherited");
        style.text.font_size = Some(rems(0.75).into());
        assert_eq!(Facts::of(&style, ui, 16.0).text, "caption");
        style.text.font_size = Some(rems(1.25).into());
        assert_eq!(Facts::of(&style, ui, 16.0).text, "title");
        style.text.font_size = Some(px(13.0).into());
        assert_eq!(Facts::of(&style, ui, 16.0).text, "13.0px");
    }

    #[test]
    fn colors_are_named_by_the_token_they_come_from() {
        let primary = Rgba { r: 0.2, g: 0.4, b: 0.8, a: 1.0 };
        let tokens = [("primary", primary), ("border", Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.5 })];
        assert_eq!(color_name_in(primary.into(), &tokens), "primary  #3366ccff");
        // The same color at another alpha is the token under the window's opacity.
        let faded = Rgba { a: 0.6, ..primary };
        assert_eq!(color_name_in(faded.into(), &tokens), "primary (window opacity)  #3366cc99");
        let other = Rgba { r: 0.9, g: 0.1, b: 0.1, a: 1.0 };
        assert_eq!(color_name_in(other.into(), &tokens), "#e61a1aff");
    }
}
