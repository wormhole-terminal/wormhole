// The settings window's own measures, each from the GPUI app (`settings_window/` and `style/scale.rs`); the
// shared ones (control height, hit area, radii) are DesignSystem's `Metric` and `Radius`.

import CoreGraphics

enum SettingsMetric {
    /// The nav's width (`nav.rs` `NAV_W`).
    static let navWidth: CGFloat = 220
    /// The page's column (`settings_window/mod.rs` `COLUMN_W`) and the intro's measure (`INTRO_W`).
    static let columnWidth: CGFloat = 760
    static let introWidth: CGFloat = 560
    /// The least a setting row is tall (`SETTING_ROW_MIN_H`).
    static let rowMinHeight: CGFloat = 40
    /// The width of a select, a slider and a shortcut recorder (`SELECT_W`, `SLIDER_W`, `RECORDER_W`).
    static let controlWidth: CGFloat = 200
    /// A stepper's value column and each of its halves (`STEPPER_VALUE_W`, `STEPPER_HALF_W`).
    static let stepperValueWidth: CGFloat = 56
    static let stepperHalfWidth: CGFloat = 28
    /// A switch's track and knob (gpui-kit's `Switch`).
    static let switchWidth: CGFloat = 36
    static let switchHeight: CGFloat = 20
    static let switchKnob: CGFloat = 16
    /// The changed-row dot (`MARK_DOT`).
    static let markDot: CGFloat = 6
    /// The preview strip's height, head and scene together (`preview_column.rs` `PREVIEW_H`), the stage's inset.
    static let previewHeight: CGFloat = 256
    static let stageInset: CGFloat = 8
    /// A nav row's height (28) and inset from the nav's edges (8).
    static let navRowHeight: CGFloat = 28
    static let navInset: CGFloat = 8
}
