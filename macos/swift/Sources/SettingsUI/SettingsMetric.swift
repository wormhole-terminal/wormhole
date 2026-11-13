// The settings window's own measures, each from the GPUI app (`settings_window/` and `style/scale.rs`); the
// shared ones (control height, hit area, radii) are DesignSystem's `Metric` and `Radius`.

import CoreGraphics

enum SettingsMetric {
    /// The sidebar's ideal width (`nav.rs` `NAV_W`); the system lets it be dragged.
    static let navWidth: CGFloat = 220
    /// The page's column (`settings_window/mod.rs` `COLUMN_W`) and the intro's measure (`INTRO_W`).
    static let columnWidth: CGFloat = 760
    static let introWidth: CGFloat = 560
    /// The window's opening size and least size, and the widest it goes: nav, column and a margin (`WINDOW_SIZE`, `WINDOW_MIN`,
    /// `WINDOW_MAX_W`).
    static let windowSize = CGSize(width: 920, height: 720)
    static let windowMin = CGSize(width: 640, height: 420)
    static var windowMaxWidth: CGFloat { navWidth + columnWidth + 64 }
    /// The least a setting row is tall (`SETTING_ROW_MIN_H`).
    static let rowMinHeight: CGFloat = 40
    /// The width of a select, a slider and a shortcut recorder (`SELECT_W`, `SLIDER_W`, `RECORDER_W`).
    static let controlWidth: CGFloat = 200
    /// A shell profile's name column in the Shells list (`SHELL_NAME_W`).
    static let shellNameWidth: CGFloat = 120
    /// The Open folders in select's width (`SHELL_W`).
    static let shellWidth: CGFloat = 240
    /// A text field's width, a pair's (name and email) and the one-letter field (`style/text_field.rs`), and
    /// the environment box's; the label before an agent's login field (`AGENT_LABEL_W`).
    static let fieldWidth: CGFloat = 240
    static let fieldPairWidth: CGFloat = 168
    static let fieldLetterWidth: CGFloat = 48
    static let fieldBoxWidth: CGFloat = 2 * fieldPairWidth + 8
    static let agentLabelWidth: CGFloat = 48
    /// An import row's label column and the value after the arrow (`LABEL_W`, `VALUE_W`).
    static let importLabelWidth: CGFloat = 150
    static let importValueWidth: CGFloat = 180
    /// The update download's meter (`METER_W`).
    static let meterWidth: CGFloat = 120
    /// A slider's value column (`STEPPER_VALUE_W`).
    static let stepperValueWidth: CGFloat = 56
    /// The "Nothing matches" card's width (`NO_RESULTS_W`).
    static let noResultsWidth: CGFloat = 420
    /// A colour swatch under the branch and primary colour rows.
    static let swatchSize: CGFloat = 20
    /// The changed-row dot (`MARK_DOT`).
    static let markDot: CGFloat = 6
    /// What `settings_text_size` multiplies the window's text by (`style/text_scale.rs`: eighths, so the caption role lands on
    /// 10.5, 12, 13.5 and 15 pt), for this window alone. The default is exactly 1. Heights fixed in points stay, as in GPUI.
    static func textScale(_ step: String?) -> CGFloat {
        switch step {
        case "small": 0.875
        case "large": 1.125
        case "larger": 1.25
        default: 1
        }
    }
    /// The preview strip's height, head and scene together (`preview_column.rs` `PREVIEW_H`), the stage's inset.
    static let previewHeight: CGFloat = 256
    static let stageInset: CGFloat = 8
}
