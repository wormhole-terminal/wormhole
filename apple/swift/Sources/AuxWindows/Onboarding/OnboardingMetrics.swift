// The intro window's sizes as the GPUI app has them (`onboarding/steps.rs`, `welcome_page.rs`, `page.rs`).

import CoreGraphics

enum OnboardingMetrics {
    /// The step card: its width, least height, the margin it keeps from the window, the footer's height.
    static let sheetWidth: CGFloat = 432
    static let sheetMinHeight: CGFloat = 640
    static let sheetMargin: CGFloat = 32
    static let footerHeight: CGFloat = 56
    /// The footer's three dots: a dot, and the current one's bar.
    static let dot: CGFloat = 4
    static let dotBar: CGFloat = 16
    static let dropZoneHeight: CGFloat = 40
    /// The welcome page's column and the mark.
    static let welcomeWidth: CGFloat = 408
    static let mark: CGFloat = 96
    /// How far a part rises as it comes in, and the wormhole's light falling from the top of the page.
    static let rise: CGFloat = 16
    static let glowHeight: CGFloat = 0.6
    static let glowAlpha: Double = 0.06
    /// The Look step's theme cards.
    static let cardHeight: CGFloat = 72
    static let cardStrip: CGFloat = 24
    static let previewBar: CGFloat = 3
    static let previewPad: CGFloat = 6
    static let previewGap: CGFloat = 4
    /// The window it opens in.
    static let windowSize = CGSize(width: 960, height: 720)
    static let windowMin = CGSize(width: 640, height: 600)
}
