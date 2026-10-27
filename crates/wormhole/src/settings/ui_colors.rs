//! The interface colors and shape the shared components draw from.

use gpui_kit::{Hsla, Pixels, px};
use wormhole_core::theme::tokens::{ThemeTokens, Translucency};

use crate::theme_bridge::{hsla, token};

/// Window chrome colors and shape (corner radii, border width), resolved from the
/// design tokens for quick access. `style` builds the shared components from them.
/// Every color field reads one token, listed in `SURFACES` and `COLORS` below.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiColors {
    /// Tiny elements: swatches, the hover box of a small icon button.
    pub radius_sm: Pixels,
    /// Everything chip-shaped, so it all matches: tabs, choice chips, badges, key
    /// caps. The theme's `metrics.radius_chip`, which follows `radius_sm` by default.
    pub radius_chip: Pixels,
    /// Controls and rows: buttons, inputs, selects, list rows.
    pub radius: Pixels,
    /// Cards, menus and other floating surfaces.
    pub radius_lg: Pixels,
    pub border_width: Pixels,
    /// The line between the window's columns (`metrics.divider_width`), 0 for none; the main
    /// window's also follows the Column dividers setting (`without_dividers`).
    pub divider_width: Pixels,
    /// The keyboard focus ring's line (`metrics.focus_ring_width`); its color is `focus`.
    pub focus_ring_width: Pixels,
    /// Icon buttons show what a click did with their own drawing (`style::icon_motion`); the main
    /// window's follows the Animate icons setting (`without_icon_motion`). Reduce Motion stops them
    /// apart from this: GPUI shows an animation's last frame then, which is where each one rests.
    pub icon_motion: bool,
    /// The terminal / main area, at the window opacity.
    pub background: Hsla,
    /// Left sidebar surface, at the window opacity.
    pub panel: Hsla,
    /// Right panel surface, at the window opacity.
    pub panel_right: Hsla,
    /// Floating surfaces: menus, popovers, key caps (`surface.elevated`).
    pub elevated: Hsla,
    /// The active title tab's fill (`tab.active.background`, the elevated surface unless a
    /// theme says otherwise).
    pub tab_active: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    /// Links and inline actions ("Edit in Fonts", "Open Notifications").
    pub link: Hsla,
    pub hover: Hsla,
    /// A pressed control (`state.active`).
    pub active: Hsla,
    /// What a dragged item would drop on, and the text a search found (`state.highlight`).
    pub highlight: Hsla,
    /// A selected right-panel row, the active icon tab, a pressed toggle (`state.selected`,
    /// a tint of the primary). Text on it is `selected_text` / `selected_muted`.
    pub selected: Hsla,
    /// The current session's row in the sidebar, and the settings nav's current page
    /// (`sidebar.row.selected`, the same tint, opaque, in a translucent window too).
    pub selected_row: Hsla,
    /// Names and icons on a selected tint (`sidebar.row.selected.text`).
    pub selected_text: Hsla,
    /// Second lines and metadata on a selected tint (`sidebar.row.selected.muted`).
    pub selected_muted: Hsla,
    /// A badge or the × inside a selected tint (`state.selected.inset`).
    pub selected_inset: Hsla,
    /// The active tab or the chosen segment of a segmented control (`chip.selected`,
    /// the solid primary).
    pub selected_chip: Hsla,
    /// Text and icons on a solid selected fill (`text.on_selected`): only on
    /// `selected_chip` and other solid fills, never on a tint.
    pub on_selected: Hsla,
    /// Secondary text on a solid selected fill (`text.on_selected.muted`).
    pub on_selected_muted: Hsla,
    /// The selected result in a palette, picker or quick panel (`list.selected`, on
    /// `elevated`), with its title and detail text.
    pub list_selected: Hsla,
    pub list_selected_text: Hsla,
    pub list_selected_muted: Hsla,
    /// Folder and section bands (`band`), one under the pointer and an icon button's
    /// hover (`band.hover`), and a band stuck to the top of a list (`band.solid`, opaque).
    /// A sidebar card's fill and edge (`card`, `card.edge`).
    pub card: Hsla,
    pub card_edge: Hsla,
    pub band: Hsla,
    pub band_hover: Hsla,
    pub band_solid: Hsla,
    pub border: Hsla,
    /// The line between the columns (`divider`, the border's unless a theme says).
    pub divider: Hsla,
    /// Control outlines and key caps (`border.strong`).
    pub border_strong: Hsla,
    /// The keyboard focus ring (`border.focus`).
    pub focus: Hsla,
    /// A text field's or stepper's fill (`input.background`).
    pub input: Hsla,
    /// What floats casts this (`shadow`): the palette, the find bar, popovers.
    pub shadow: Hsla,
    /// A tooltip, and the readout beside a resize handle (`tooltip.*`).
    pub tooltip: Hsla,
    pub tooltip_text: Hsla,
    pub primary: Hsla,
    /// Status colors for icons, dots and outlines (3:1).
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
    pub info: Hsla,
    /// The status colors as text (4.5:1), also on a selected row (`status.*.text`).
    pub success_text: Hsla,
    pub warning_text: Hsla,
    pub danger_text: Hsla,
    /// Inline messages (`style::banner`): each kind's fill and outline (`banner.<kind>.*`).
    pub banner_info: Hsla,
    pub banner_info_border: Hsla,
    pub banner_warning: Hsla,
    pub banner_warning_border: Hsla,
    pub banner_danger: Hsla,
    pub banner_danger_border: Hsla,
    pub banner_success: Hsla,
    pub banner_success_border: Hsla,
    /// A file's added and removed line counts (`diff.added` / `diff.removed`).
    pub added: Hsla,
    pub removed: Hsla,
    /// Git branch names (`git.branch`); see `style::branch_label`.
    pub branch: Hsla,
    /// A branch chip's fill (`git.branch.background`).
    pub branch_bg: Hsla,
    /// A tinted chip: the sidebar's filter, a project override (`chip.tint`).
    pub chip_tint: Hsla,
    /// An unchecked checkbox's outline (`checkbox.border`, 3:1), its box and a checked one's
    /// fill (`checkbox.background`, `checkbox.checked`).
    pub checkbox_border: Hsla,
    pub checkbox_bg: Hsla,
    pub checkbox_checked: Hsla,
    /// What sits on the solid primary: a checked box's mark (`text.on_primary`).
    pub on_primary: Hsla,
    /// A resize handle's line under the pointer and while dragging (`resize.handle.*`).
    pub resize_hover: Hsla,
    pub resize_active: Hsla,
    /// Agent status dots and glyphs.
    pub agent_working: Hsla,
    pub agent_done: Hsla,
    pub agent_attention: Hsla,
    pub agent_exited: Hsla,
    /// The bell badge on a status tile (`agent.bell`).
    pub agent_bell: Hsla,
    /// The halo round a needs-you dot and the fill of a waiting tab (`agent.attention.ring`
    /// / `.tint`).
    pub attention_ring: Hsla,
    pub attention_tint: Hsla,
    /// "Needs input" and "Finished" as text, also on the selected row (`agent.*.text`).
    pub attention_text: Hsla,
    pub done_text: Hsla,
    /// A status tile: black unless the agent has a colour of its own (the registry's, not a token),
    /// the mark on each, the sheen across its top and its hairline edge.
    pub tile_background: Hsla,
    pub tile_mark: Hsla,
    pub tile_mark_brand: Hsla,
    pub tile_sheen: Hsla,
    pub tile_edge: Hsla,
}

/// One color field of [`UiColors`], and the token it reads.
struct Field {
    name: &'static str,
    slot: fn(&mut UiColors) -> &mut Hsla,
    token: &'static str,
}

impl Field {
    fn read(&self, tokens: &ThemeTokens) -> Hsla {
        if tokens.get(self.token).is_none() {
            tracing::warn!(
                field = self.name,
                token = self.token,
                "UiColors reads a token that does not exist"
            );
        }
        token(tokens, self.token)
    }
}

macro_rules! fields {
    ($($field:ident = $token:expr;)*) => {
        &[$(Field { name: stringify!($field), slot: |colors| &mut colors.$field, token: $token }),*]
    };
}

/// The window's own surfaces, drawn at the window opacity so the blur shows through.
const SURFACES: &[Field] = fields! {
    background = "surface.window";
    panel = "sidebar.background";
    panel_right = "surface.panel";
};

/// Every other color field, as the token gives it.
const COLORS: &[Field] = fields! {
    elevated = "surface.elevated";
    tab_active = "tab.active.background";
    text = "text.primary";
    muted = "text.muted";
    faint = "text.faint";
    link = "text.link";
    hover = "state.hover";
    active = "state.active";
    highlight = "state.highlight";
    selected = "state.selected";
    selected_row = "sidebar.row.selected";
    selected_text = "sidebar.row.selected.text";
    selected_muted = "sidebar.row.selected.muted";
    selected_inset = "state.selected.inset";
    selected_chip = "chip.selected";
    on_selected = "text.on_selected";
    on_selected_muted = "text.on_selected.muted";
    list_selected = "list.selected";
    list_selected_text = "list.selected.text";
    list_selected_muted = "list.selected.muted";
    card = "card";
    card_edge = "card.edge";
    band = "band";
    band_hover = "band.hover";
    band_solid = "band.solid";
    border = "border";
    divider = "divider";
    border_strong = "border.strong";
    focus = "border.focus";
    input = "input.background";
    shadow = "shadow";
    tooltip = "tooltip.background";
    tooltip_text = "tooltip.text";
    primary = "primary";
    success = "status.success";
    warning = "status.warning";
    danger = "status.danger";
    info = "status.info";
    success_text = "status.success.text";
    warning_text = "status.warning.text";
    danger_text = "status.danger.text";
    banner_info = "banner.info.background";
    banner_info_border = "banner.info.border";
    banner_warning = "banner.warning.background";
    banner_warning_border = "banner.warning.border";
    banner_danger = "banner.danger.background";
    banner_danger_border = "banner.danger.border";
    banner_success = "banner.success.background";
    banner_success_border = "banner.success.border";
    added = "diff.added";
    removed = "diff.removed";
    branch = "git.branch";
    branch_bg = "git.branch.background";
    chip_tint = "chip.tint";
    checkbox_border = "checkbox.border";
    checkbox_bg = "checkbox.background";
    checkbox_checked = "checkbox.checked";
    on_primary = "text.on_primary";
    resize_hover = "resize.handle.hover";
    resize_active = "resize.handle.active";
    agent_working = "agent.working";
    agent_done = "agent.done";
    agent_attention = "agent.attention";
    agent_exited = "agent.exited";
    agent_bell = "agent.bell";
    attention_ring = "agent.attention.ring";
    attention_tint = "agent.attention.tint";
    attention_text = "agent.attention.text";
    done_text = "agent.done.text";
    tile_background = "tile.background";
    tile_mark = "tile.mark";
    tile_mark_brand = "tile.mark.brand";
    tile_sheen = "tile.sheen";
    tile_edge = "tile.edge";
};

impl UiColors {
    /// These colors with no line between the columns: the Column dividers setting, off.
    pub(crate) fn without_dividers(self) -> Self {
        Self { divider_width: px(0.0), ..self }
    }

    pub(crate) fn without_icon_motion(self) -> Self {
        Self { icon_motion: false, ..self }
    }

    /// The colors for a window that looks like `look`. A translucent one paints its surfaces at
    /// its opacity, and draws its text as core judges it on what shows through
    /// (`ThemeTokens::translucent`).
    pub(crate) fn new(tokens: &ThemeTokens, look: Translucency) -> Self {
        let m = &tokens.metrics;
        let mut colors = Self {
            radius_sm: px(m.radius_sm),
            radius_chip: px(m.radius_chip),
            radius: px(m.radius),
            radius_lg: px(m.radius_lg),
            border_width: px(m.border_width),
            divider_width: px(m.divider_width),
            focus_ring_width: px(m.focus_ring_width),
            icon_motion: true,
            ..Self::default()
        };
        for field in SURFACES {
            let color = field.read(tokens);
            *(field.slot)(&mut colors) = Hsla { a: color.a * look.opacity, ..color };
        }
        for field in COLORS {
            *(field.slot)(&mut colors) = field.read(tokens);
        }
        if let Some(seen) = tokens.translucent(look) {
            colors.text = hsla(seen.text);
            colors.muted = hsla(seen.muted);
            colors.faint = hsla(seen.faint);
            colors.branch = hsla(seen.branch);
            colors.selected_text = hsla(seen.selected_text);
            colors.selected_muted = hsla(seen.selected_muted);
        }
        colors
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use wormhole_core::theme::tokens::UI_TOKENS;

    use super::{COLORS, SURFACES};

    #[test]
    fn every_color_field_comes_from_a_real_token() {
        for field in SURFACES.iter().chain(COLORS) {
            assert!(
                UI_TOKENS.iter().any(|t| t.name == field.token),
                "UiColors::{} reads {:?}, which is not a UI token",
                field.name,
                field.token
            );
        }
    }

    /// Each `Hsla` field is in the table exactly once, so none is left at its default
    /// (transparent) and none is set twice.
    #[test]
    fn every_color_field_is_mapped_once() {
        let mut mapped = BTreeSet::new();
        for field in SURFACES.iter().chain(COLORS) {
            assert!(mapped.insert(field.name), "{} is mapped twice", field.name);
        }
        let source = include_str!("ui_colors.rs");
        let start = source.find("pub struct UiColors {").expect("the struct");
        let body = &source[start..start + source[start..].find("\n}").expect("its end")];
        let fields: BTreeSet<&str> = body
            .lines()
            .filter_map(|line| line.trim().strip_prefix("pub ")?.strip_suffix(": Hsla,"))
            .collect();
        assert!(!fields.is_empty());
        assert_eq!(fields, mapped, "every Hsla field of UiColors needs a row in SURFACES or COLORS");
    }
}
