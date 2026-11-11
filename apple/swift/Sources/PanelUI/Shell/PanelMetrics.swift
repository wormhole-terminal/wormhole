// What the right panel's cards and rows measure, one number each, from `style/scale.rs`, `style/metrics.rs`
// (the panel's cards are the sidebar's regular ones) and `style/bands.rs`. Colours and corner radii of the
// theme come from the tokens; only what the GPUI app fixes in code is here.

import CoreGraphics

enum PanelMetrics {
    /// The room between the tab bar and the first card (`PANEL_LIST_TOP`).
    static let listTop: CGFloat = 4
    /// A card: its distance from the panel's sides, its padding, its corner and the gap under it.
    static let cardMargin: CGFloat = 10
    static let cardPad: CGFloat = 4
    static let cardRadius: CGFloat = 12
    static let cardGap: CGFloat = 8
    /// How far in a band's or row's content starts inside the card's padding (`CARD_INSET`).
    static let inset: CGFloat = 8
    /// The gap between the parts of a row (`gap_2`).
    static let gap: CGFloat = 8
    /// The face of a band's icon button, and of a row's hover action (`BAND_BUTTON`, `ROW_BUTTON`).
    static let bandButton: CGFloat = 24
    static let rowButton: CGFloat = 22
    static let bandGlyph: CGFloat = 16
    /// The kind letter's column, so the line counts above each other line up (`KIND_LETTER_W`).
    static let kindLetter: CGFloat = 12
    /// The share of a row a file name may take before it truncates in the middle (`FILE_NAME_MAX`).
    static let fileNameShare: CGFloat = 0.55
    /// Space left under the last card so its last row can scroll clear of the edge.
    static let tail: CGFloat = 16
    /// The checkbox of a next step, and its mark (`style/checkbox.rs`).
    static let checkbox: CGFloat = 14
    static let checkMark: CGFloat = 10
    /// A note's text lines: 14 pt on 20, and how many an open note shows before it fades (`NOTE_OPEN_MAX_LINES`).
    static let noteLine: CGFloat = 20
    static let noteMaxLines = 12
    /// What 14 pt text needs between lines to sit on the 20 pt line.
    static let noteLineGap: CGFloat = 3
    /// A two-line commit row's lines (`NAME_LINE`, `META_LINE`) and the pad above and below.
    static let nameLine: CGFloat = 20
    static let metaLine: CGFloat = 16
}
