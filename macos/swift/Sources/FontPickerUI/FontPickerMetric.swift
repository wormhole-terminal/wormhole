// The picker's measures, each from the GPUI app's (`settings_window/font_picker.rs`) unless it says it is ours.

import CoreGraphics

public enum FontPickerMetric {
    /// The list's tallest height and one row (`LIST_HEIGHT`, `ENTRY_HEIGHT`).
    static let listHeight: CGFloat = 264
    static let rowHeight: CGFloat = 32
    /// The list's column (`MENU_W`).
    static let listWidth: CGFloat = 340
    /// The select that opens it (`SELECT_W`).
    public static let selectWidth: CGFloat = 272
    /// The whole picker: the list and the preview beside it, and the divider.
    public static let width: CGFloat = listWidth + previewWidth + 1
    /// Ours: the preview column beside the list.
    static let previewWidth: CGFloat = 320
    /// The specimen's line height outside the terminal slot (`draw_specimen`).
    static let specimenLineHeight: Double = 1.4
    /// Ours: the preview keeps within what the column holds, however large the font is set.
    static let previewSizes: ClosedRange<Double> = 11...14
}
