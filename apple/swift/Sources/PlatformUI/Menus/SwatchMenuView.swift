// The row of colour dots in a group's context menu, as Finder's tags in a file's: one click picks and closes the
// menu. The dots are drawn from theme tokens (`MenuStyle`), the empty ring is the dot that takes the colour away.

import AppKit
import PlatformModel

@MainActor
final class SwatchMenuView: NSView {
    private static let dot: CGFloat = 14, gap: CGFloat = 8, inset: CGFloat = 14, height: CGFloat = 28

    private let row: SwatchRow
    private let style: MenuStyle
    private let pick: (ContextAction) -> Void
    private var hovered: Int?

    init(row: SwatchRow, style: MenuStyle, pick: @escaping (ContextAction) -> Void) {
        self.row = row
        self.style = style
        self.pick = pick
        let width = Self.inset * 2 + CGFloat(row.swatches.count) * Self.dot + CGFloat(max(row.swatches.count - 1, 0)) * Self.gap
        super.init(frame: NSRect(x: 0, y: 0, width: width, height: Self.height))
        setAccessibilityRole(.radioGroup)
        setAccessibilityLabel("Colour")
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeInActiveApp, .inVisibleRect], owner: self))
    }

    required init?(coder: NSCoder) { nil }

    private func rect(_ index: Int) -> NSRect {
        NSRect(x: Self.inset + CGFloat(index) * (Self.dot + Self.gap), y: (Self.height - Self.dot) / 2, width: Self.dot, height: Self.dot)
    }

    private func index(at point: NSPoint) -> Int? {
        row.swatches.indices.first { rect($0).insetBy(dx: -Self.gap / 2, dy: -Self.gap).contains(point) }
    }

    override func draw(_ dirtyRect: NSRect) {
        for (index, swatch) in row.swatches.enumerated() {
            let frame = rect(index)
            if let color = swatch.token.flatMap(style.color) {
                color.setFill()
                NSBezierPath(ovalIn: frame).fill()
            } else {
                NSColor.secondaryLabelColor.setStroke()
                let ring = NSBezierPath(ovalIn: frame.insetBy(dx: 1, dy: 1))
                ring.lineWidth = 1
                ring.stroke()
            }
            if row.selected == index || hovered == index {
                NSColor.labelColor.withAlphaComponent(row.selected == index ? 0.9 : 0.4).setStroke()
                let ring = NSBezierPath(ovalIn: frame.insetBy(dx: -2.5, dy: -2.5))
                ring.lineWidth = 1.5
                ring.stroke()
            }
        }
    }

    override func mouseMoved(with event: NSEvent) {
        let now = index(at: convert(event.locationInWindow, from: nil))
        if now != hovered { hovered = now; needsDisplay = true }
    }

    override func mouseExited(with event: NSEvent) { hovered = nil; needsDisplay = true }

    override func mouseUp(with event: NSEvent) {
        guard let index = index(at: convert(event.locationInWindow, from: nil)) else { return }
        enclosingMenuItem?.menu?.cancelTracking()
        pick(row.command(index))
    }

    override func accessibilityChildren() -> [Any]? { nil }
}
