// The status item's glyph, drawn rather than taken from SF Symbols (`platform/macos/status_item/icon.rs`): the
// terminal outline, filled while something works, with the needs-you dot, the failed dot or the finished check as a
// badge on its top-right corner behind a cut-out ring, and the four frames of the working animation, in which the
// prompt's line types itself out. The canvas is the same in every state with the box centred in it, so the glyph
// never shifts as the status changes, and no taller than the glyph, so the menu bar never scales it down.
//
// Without a mark the image is a template, so AppKit tints it for a light or dark bar and the highlighted state; with
// one it can't be (the mark keeps its colour) and the outline is drawn in `labelColor`.

import AppKit
import PlatformModel

@MainActor
public enum StatusIconImage {
    static let canvas = NSSize(width: 21, height: 16)
    /// The terminal box, centred in the canvas (identical in every state).
    static let box = NSRect(x: 3.75, y: 2.5, width: 13.5, height: 11)
    static let radius = 2.6, stroke = 1.9
    /// The cut-out ring that sets a badge off from what is under it, the dot, and the check's own stroke.
    static let ring = 1.3, dot = 6.0, check = 2.6

    public struct Colors {
        public var attention: NSColor, done: NSColor, danger: NSColor
        public init(attention: NSColor, done: NSColor, danger: NSColor) { (self.attention, self.done, self.danger) = (attention, done, danger) }

        public init(style: MenuStyle) {
            self.init(
                attention: style.color("agent.attention") ?? .systemOrange, done: style.color("agent.done") ?? .systemGreen,
                danger: style.color("status.danger") ?? .systemRed)
        }
    }

    /// One state of the item. `filled`: something works; `frame`: the animation's frame (nil: the full prompt).
    public static func image(filled: Bool, frame: Int?, mark: StatusMark, colors: Colors, description: String) -> NSImage {
        let marked = mark != .none
        // Flipped, so the coordinates read like an SVG's (y down).
        let image = NSImage(size: canvas, flipped: true) { _ in
            glyph(filled: filled, frame: frame, ink: marked ? .labelColor : .black)
            switch mark {
            case .needsYou: badgeDot(colors.attention)
            case .failed: badgeDot(colors.danger)
            case .finished: badgeCheck(colors.done)
            case .none: break
            }
            return true
        }
        image.isTemplate = !marked
        image.accessibilityDescription = description
        return image
    }

    private static func compositing(_ operation: NSCompositingOperation) {
        NSGraphicsContext.current?.compositingOperation = operation
    }

    private static func line(_ points: [NSPoint], width: CGFloat) -> NSBezierPath {
        let path = NSBezierPath()
        for (index, point) in points.enumerated() { index == 0 ? path.move(to: point) : path.line(to: point) }
        path.lineWidth = width
        path.lineCapStyle = .round
        path.lineJoinStyle = .round
        return path
    }

    /// A rounded box with a `>` prompt and its line, the prompt centred in it. Working: the box is filled and the
    /// prompt cut out of it; an animation frame draws the line 0–3 long.
    private static func glyph(filled: Bool, frame: Int?, ink: NSColor) {
        ink.set()
        let body = NSBezierPath(roundedRect: box, xRadius: radius, yRadius: radius)
        let left = box.minX, mid = box.minY + box.height / 2
        let chevron = line([NSPoint(x: left + 3.15, y: mid - 2.4), NSPoint(x: left + 5.85, y: mid), NSPoint(x: left + 3.15, y: mid + 2.4)], width: stroke)
        let length = frame.map(Double.init) ?? 3
        let prompt = length > 0 ? line([NSPoint(x: left + 7.15, y: mid + 2.4), NSPoint(x: left + 7.15 + length, y: mid + 2.4)], width: stroke) : nil
        if filled {
            body.fill()
            compositing(.destinationOut)
        } else {
            body.lineWidth = stroke
            body.stroke()
        }
        chevron.stroke()
        prompt?.stroke()
        compositing(.sourceOver)
    }

    /// The amber dot (needs input) or the danger one (failed), tucked onto the box's top-right corner.
    private static func badgeDot(_ color: NSColor) {
        let center = NSPoint(x: box.maxX - 0.1, y: dot / 2 + ring)
        func circle(_ r: CGFloat) -> NSBezierPath { NSBezierPath(ovalIn: NSRect(x: center.x - r, y: center.y - r, width: 2 * r, height: 2 * r)) }
        compositing(.destinationOut)
        circle(dot / 2 + ring).fill()
        compositing(.sourceOver)
        color.set()
        circle(dot / 2).fill()
    }

    private static func badgeCheck(_ color: NSColor) {
        let center = NSPoint(x: box.maxX - 0.3, y: 4.4)
        let points = [(-2.8, 0.2), (-0.9, 2.1), (2.7, -2.3)].map { NSPoint(x: center.x + $0.0, y: center.y + $0.1) }
        compositing(.destinationOut)
        line(points, width: check + 2 * ring).stroke()
        compositing(.sourceOver)
        color.set()
        line(points, width: check).stroke()
    }
}
