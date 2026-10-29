//! The Appearance and Fonts sections: theme, accent, window, title row, file icons and the type.

use super::{entry, group, untitled};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const APPEARANCE: &[Group] = &[
    untitled(&[entry("appearance-preview", SETTINGS_APPEARANCE_PREVIEW, &[])
        .with_keywords(SETTINGS_APPEARANCE_PREVIEW_KEYWORDS)]),
    group(
        SETTINGS_APPEARANCE_GROUP_THEME,
        &[
            entry("theme-mode", SETTINGS_APPEARANCE_THEME_MODE, &["theme_mode"])
                .with_description(SETTINGS_APPEARANCE_THEME_MODE_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_THEME_MODE_KEYWORDS),
            entry("theme", SETTINGS_APPEARANCE_THEME, &["theme", "theme_light", "hidden_themes"])
                .with_keywords(SETTINGS_APPEARANCE_THEME_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_APPEARANCE_GROUP_ACCENT,
        &[
            entry("primary-color", SETTINGS_APPEARANCE_PRIMARY_COLOR, &["theme_primary", "primary_color"])
                .with_description(SETTINGS_APPEARANCE_PRIMARY_COLOR_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_PRIMARY_COLOR_KEYWORDS),
            entry("branch-color", SETTINGS_APPEARANCE_BRANCH_COLOR, &["branch_color", "branch_color_custom"])
                .with_description(SETTINGS_APPEARANCE_BRANCH_COLOR_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_BRANCH_COLOR_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_APPEARANCE_GROUP_WINDOW,
        &[
            entry("opacity", SETTINGS_APPEARANCE_OPACITY, &["opacity"])
                .with_description(SETTINGS_APPEARANCE_OPACITY_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_OPACITY_KEYWORDS),
            entry("blur", SETTINGS_APPEARANCE_BLUR, &["blur"])
                .with_description(SETTINGS_APPEARANCE_BLUR_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_BLUR_KEYWORDS),
            // Before macOS 26 there is no glass to turn on, so the row isn't there.
            entry("liquid-glass", SETTINGS_APPEARANCE_LIQUID_GLASS, &["liquid_glass"])
                .with_needs(|_| crate::platform::system::has_liquid_glass())
                .with_description(SETTINGS_APPEARANCE_LIQUID_GLASS_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_LIQUID_GLASS_KEYWORDS),
            entry(
                "translucent",
                SETTINGS_APPEARANCE_TRANSLUCENT,
                &["settings_translucent", "studio_translucent"],
            )
            .with_description(SETTINGS_APPEARANCE_TRANSLUCENT_DESCRIPTION)
            .with_keywords(SETTINGS_APPEARANCE_TRANSLUCENT_KEYWORDS),
            entry("title-border", SETTINGS_APPEARANCE_TITLE_BORDER, &["title_border"])
                .with_description(SETTINGS_APPEARANCE_TITLE_BORDER_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_TITLE_BORDER_KEYWORDS),
            entry("column-dividers", SETTINGS_APPEARANCE_COLUMN_DIVIDERS, &["column_dividers"])
                .with_description(SETTINGS_APPEARANCE_COLUMN_DIVIDERS_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_COLUMN_DIVIDERS_KEYWORDS),
            entry("status-bar", SETTINGS_APPEARANCE_STATUS_BAR, &["status_bar"])
                .with_description(SETTINGS_APPEARANCE_STATUS_BAR_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_STATUS_BAR_KEYWORDS),
            entry("icon-animations", SETTINGS_APPEARANCE_ICON_ANIMATIONS, &["icon_animations"])
                .with_description(SETTINGS_APPEARANCE_ICON_ANIMATIONS_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_ICON_ANIMATIONS_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_APPEARANCE_GROUP_TITLE_ROW,
        &[entry("title-row", SETTINGS_APPEARANCE_TITLE_ROW, &["title_row"])
            .with_description(SETTINGS_APPEARANCE_TITLE_ROW_DESCRIPTION)
            .with_keywords(SETTINGS_APPEARANCE_TITLE_ROW_KEYWORDS)],
    ),
    group(
        SETTINGS_APPEARANCE_GROUP_FILE_ICONS,
        &[
            entry("icon-theme", SETTINGS_APPEARANCE_ICON_THEME, &["icon_theme"])
                .with_description(SETTINGS_APPEARANCE_ICON_THEME_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_ICON_THEME_KEYWORDS),
            entry("icon-themes-folder", SETTINGS_APPEARANCE_ICON_THEMES_FOLDER, &[])
                .with_description(SETTINGS_APPEARANCE_ICON_THEMES_FOLDER_DESCRIPTION)
                .with_keywords(SETTINGS_APPEARANCE_ICON_THEMES_FOLDER_KEYWORDS),
        ],
    ),
];

pub(super) const FONTS: &[Group] = &[
    untitled(&[entry("font-specimen", SETTINGS_FONTS_FONT_SPECIMEN, &[])
        .with_keywords(SETTINGS_FONTS_FONT_SPECIMEN_KEYWORDS)]),
    group(
        SETTINGS_APPEARANCE_GROUP_TERMINAL,
        &[
            entry("font-terminal", SETTINGS_FONTS_FONT_TERMINAL, &["font_family", "font_size"])
                .with_description(SETTINGS_FONTS_FONT_TERMINAL_DESCRIPTION)
                .with_keywords(SETTINGS_FONTS_FONT_TERMINAL_KEYWORDS),
            entry("line-height", SETTINGS_FONTS_LINE_HEIGHT, &["line_height"])
                .with_description(SETTINGS_FONTS_LINE_HEIGHT_DESCRIPTION)
                .with_keywords(SETTINGS_FONTS_LINE_HEIGHT_KEYWORDS),
            entry("font-ligatures", SETTINGS_FONTS_FONT_LIGATURES, &["font_ligatures"])
                .with_description(SETTINGS_FONTS_FONT_LIGATURES_DESCRIPTION)
                .with_keywords(SETTINGS_FONTS_FONT_LIGATURES_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_APPEARANCE_GROUP_CODE,
        &[entry("font-code", SETTINGS_FONTS_FONT_CODE, &["code_font_family", "code_font_size"])
            .with_description(SETTINGS_FONTS_FONT_CODE_DESCRIPTION)
            .with_keywords(SETTINGS_FONTS_FONT_CODE_KEYWORDS)],
    )
    .with_aside(SETTINGS_APPEARANCE_GROUP_CODE_ASIDE),
    group(
        SETTINGS_APPEARANCE_GROUP_INTERFACE,
        &[entry("font-interface", SETTINGS_FONTS_FONT_INTERFACE, &["ui_font_family", "ui_font_size"])
            .with_description(SETTINGS_FONTS_FONT_INTERFACE_DESCRIPTION)
            .with_keywords(SETTINGS_FONTS_FONT_INTERFACE_KEYWORDS)],
    ),
    group(
        SETTINGS_APPEARANCE_GROUP_YOUR_FONTS,
        &[
            entry("fonts-folder", SETTINGS_FONTS_FOLDER, &[])
                .with_description(SETTINGS_FONTS_FOLDER_DESCRIPTION)
                .with_keywords(SETTINGS_FONTS_FOLDER_KEYWORDS),
            entry("custom-fonts", SETTINGS_FONTS_CUSTOM_FONTS, &[])
                .with_keywords(SETTINGS_FONTS_CUSTOM_FONTS_KEYWORDS),
        ],
    ),
];
