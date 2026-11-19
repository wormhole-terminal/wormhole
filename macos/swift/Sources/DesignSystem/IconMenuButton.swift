// An icon button that opens a menu (`buttons.rs` `IconButton` with a menu): the shared button's face and colours round a
// SwiftUI menu (muted, `text.primary` on `band.hover` under the pointer, faint while disabled, no press look), so a pane's
// "…" and Open in…'s chevron answer the pointer as every other icon button does. `narrow` is a chevron's face beside a
// button (`narrow`: `width` wide, the hit's height, a 12 pt glyph); `chevron` turns the glyph over while its menu is open
// (`menu_chevron`). A SwiftUI menu says nothing of being open: AppKit says a menu is tracked, and the pointer over the
// button says whose.

import AppKit
import SwiftUI

public struct IconMenuButton<Glyph: View, Items: View>: View {
    /// The tooltip; VoiceOver hears it without its key.
    public let label: String
    private let items: () -> Items
    private let glyph: (_ colour: Color, _ size: CGFloat) -> Glyph
    var face = IconButtonFace.control
    var narrowWidth: CGFloat?
    var turns = false
    @State private var hovering = false
    @State private var open = false
    @Environment(\.isEnabled) private var enabled
    @Environment(\.tokens) private var tokens
    /// Where the view that asked for it was built, for inspect mode.
    let file: StaticString
    let line: UInt

    /// A drawing of the caller's as the glyph, given the button's colour of the moment and the face's glyph size.
    public init(
        label: String, file: StaticString = #fileID, line: UInt = #line, @ViewBuilder items: @escaping () -> Items,
        @ViewBuilder glyph: @escaping (_ colour: Color, _ size: CGFloat) -> Glyph
    ) {
        (self.label, self.items, self.glyph, self.file, self.line) = (label, items, glyph, file, line)
    }

    /// A row's 22 pt face with a 14 pt glyph (`in_row`).
    public func inRow() -> Self { with { $0.face = .row } }

    /// A chevron beside a button: a face `width` wide and the hit's height, with a 12 pt glyph (`narrow`).
    public func narrow(_ width: CGFloat) -> Self { with { $0.narrowWidth = width } }

    /// The glyph turned over while the menu is open (`menu_chevron`); Animate icons off or Reduce Motion swaps it at once.
    public func chevron() -> Self { with { $0.turns = true } }

    private func with(_ change: (inout Self) -> Void) -> Self {
        var copy = self
        change(&copy)
        return copy
    }

    public var body: some View {
        let colour = !enabled ? tokens.textFaint : hovering ? tokens.textPrimary : tokens.textMuted
        let size = narrowWidth == nil ? face.glyph : IconButtonMark.narrowGlyph
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        Menu(content: items) {
            glyph(colour, size)
                .menuChevron(open: turns && open)
                .foregroundStyle(colour)
                .frame(width: narrowWidth ?? face.size, height: narrowWidth == nil ? face.size : Metric.hitMin)
                .background(hovering && enabled ? tokens.bandHover : .clear, in: shape)
                .frame(width: narrowWidth ?? Metric.hitMin, height: Metric.hitMin)
                .contentShape(Rectangle())
        }
        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden).fixedSize()
        .onHover { hovering = $0 }
        .onReceive(NotificationCenter.default.publisher(for: NSMenu.didBeginTrackingNotification)) { _ in if hovering { open = true } }
        .onReceive(NotificationCenter.default.publisher(for: NSMenu.didEndTrackingNotification)) { _ in open = false }
        .marked("icon.button", style: "hit hitMin · face controlSmall or narrow · radius control · glyph glyph", file: file, line: line)  // text: inspect mode's note, in the scales' names
        .tooltipName(label)
    }
}

extension IconMenuButton where Glyph == IconButtonGlyph {
    /// An SF Symbol as the glyph (a pane's "…").
    public init(symbol: String, label: String, file: StaticString = #fileID, line: UInt = #line, @ViewBuilder items: @escaping () -> Items) {
        self.init(label: label, file: file, line: line, items: items) { _, size in IconButtonGlyph(symbol: symbol, lucide: nil, size: size) }
    }

    /// The Lucide glyph GPUI's button draws (Open in…'s chevron).
    public init(lucide: Lucide, label: String, file: StaticString = #fileID, line: UInt = #line, @ViewBuilder items: @escaping () -> Items) {
        self.init(label: label, file: file, line: line, items: items) { _, size in IconButtonGlyph(symbol: "", lucide: lucide, size: size) }
    }
}
