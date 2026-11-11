// The Studio window's measures, from the GPUI app (`studio/mod.rs`, `token_list.rs`, `token_editor.rs`, `title.rs`).

import DesignSystem
import SwiftUI

enum StudioMetrics {
    /// The window's size, and the smallest it may get: the list (300) and the editor (320) at their widths with the
    /// gallery still two columns (`WINDOW_SIZE`, `WINDOW_MIN`).
    static let windowSize = CGSize(width: 1344, height: 900)
    static let windowMin = CGSize(width: 1300, height: 640)
    static let listWidth: CGFloat = 300  // LIST_W
    static let editorWidth: CGFloat = 320  // EDITOR_W
    /// The longest a token row's value runs before it is cut short (`VALUE_MAX_W`).
    static let valueMaxWidth: CGFloat = 112
    /// The name field while renaming, the most room the name takes before it is cut with "…", the Save menu.
    static let nameFieldWidth: CGFloat = 200  // NAME_FIELD_W
    static let nameMaxWidth: CGFloat = 260  // NAME_MAX_W
    static let menuWidth: CGFloat = 240  // MENU_W
    /// A metric's slider and stepper stop here (`METRIC_MAX`).
    static let metricMax: Double = 24
    /// Where the title row's content starts: the traffic lights are 12 in and span 52, and a gap of 12 follows
    /// (`WindowUI/Chrome.swift` `lightsX`, `assumedLightsSpan`).
    static let lightsEnd: CGFloat = 76
    /// The swatch of a token row: a 16 pt lead column holds it.
    static let swatch: CGFloat = 16
    /// The two columns of the gallery stand this far apart (`part_gap` of a specimen is its own).
    static let galleryGap = Space._4
}
