// The picker panel's sizes: the system's native ones for a list of this kind, like a menu row (`platform/picker.rs`:
// the app's style scale is for what GPUI draws, this is system chrome).

import CoreGraphics
import DesignSystem
import PlatformModel

enum FolderPickerMetrics {
    static let width: CGFloat = 480
    static let row: CGFloat = 28
    static let maxRows = 9
    /// What the panel has besides rows: its padding, the field, and the hint line under the list.
    static let chrome: CGFloat = 76
    static let crumbs: CGFloat = 26
    static let field: CGFloat = 36
    static let hint: CGFloat = 24
    /// The glyph of a row's lead and the chevron's.
    static let icon: CGFloat = 16

    static func card(hasCrumbs: Bool) -> PickerCard {
        PickerCard(width: Double(width), chrome: Double(chrome + (hasCrumbs ? crumbs : 0)), row: Double(row), maxRows: maxRows)
    }
}
