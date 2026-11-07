// Where everything sits inside the capsule: `platform/macos/notice_layout.rs` (AppKit's own sizes for a small
// capsule, not the app's style scale: system chrome, like a menu row).

import CoreGraphics

enum NoticeChipMetrics {
    static let height: CGFloat = 34
    /// Beside the symbol and the words.
    static let pad: CGFloat = 14
    static let icon: CGFloat = 15
    static let gap: CGFloat = 7
    /// A small button's square (its click target and the circle that lights under the pointer) and its picture.
    static let button: CGFloat = 20
    static let buttonIcon: CGFloat = 11
    static let buttonGap: CGFloat = 2
    /// Past the last button: less than `pad`, its circle already has room round the picture.
    static let endPad: CGFloat = 8
    /// Over this the capsule is cut and its words truncate: the window's margin (`WINDOW_MARGIN`) each side.
    static let windowMargin: CGFloat = 8
}
