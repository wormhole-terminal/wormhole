// An SVG path's `d` string as a SwiftUI `Path`, so the agents' logos and Lucide's icons (the GPUI app draws both from
// bundled SVGs) are vector shapes tinted by a token at any size, with no image to recolour. Every command of SVG 1.1
// paths is read (`M L H V C S Q T A Z`, absolute and relative); arcs are cut into cubics by `SVGArc`.

import SwiftUI

enum SVGPath {
    static func path(_ d: String) -> Path {
        var scanner = Scanner(bytes: Array(d.utf8))
        var path = Path()
        var (current, start) = (CGPoint.zero, CGPoint.zero)
        // The last control point, mirrored by `S` and `T` when the command before was a curve of its kind.
        var (lastCubic, lastQuad): (CGPoint?, CGPoint?) = (nil, nil)
        var command: UInt8 = 0
        while let next = scanner.command(after: command) {
            command = next
            let relative = command >= 97
            let origin = relative ? current : .zero
            func point() -> CGPoint? {
                guard let x = scanner.number(), let y = scanner.number() else { return nil }
                return CGPoint(x: origin.x + x, y: origin.y + y)
            }
            var cubic: CGPoint?, quad: CGPoint?
            switch command | 0x20 {
            case UInt8(ascii: "m"):
                guard let to = point() else { return path }
                path.move(to: to)
                (current, start) = (to, to)
                // Further pairs after a move are lines.
                command = relative ? UInt8(ascii: "l") : UInt8(ascii: "L")
            case UInt8(ascii: "l"):
                guard let to = point() else { return path }
                path.addLine(to: to); current = to
            case UInt8(ascii: "h"):
                guard let x = scanner.number() else { return path }
                current.x = origin.x + x; path.addLine(to: current)
            case UInt8(ascii: "v"):
                guard let y = scanner.number() else { return path }
                current.y = origin.y + y; path.addLine(to: current)
            case UInt8(ascii: "c"):
                guard let c1 = point(), let c2 = point(), let to = point() else { return path }
                path.addCurve(to: to, control1: c1, control2: c2); (current, cubic) = (to, c2)
            case UInt8(ascii: "s"):
                guard let c2 = point(), let to = point() else { return path }
                path.addCurve(to: to, control1: reflect(lastCubic, about: current), control2: c2)
                (current, cubic) = (to, c2)
            case UInt8(ascii: "q"):
                guard let c = point(), let to = point() else { return path }
                path.addQuadCurve(to: to, control: c); (current, quad) = (to, c)
            case UInt8(ascii: "t"):
                guard let to = point() else { return path }
                let c = reflect(lastQuad, about: current)
                path.addQuadCurve(to: to, control: c); (current, quad) = (to, c)
            case UInt8(ascii: "a"):
                guard let rx = scanner.number(), let ry = scanner.number(), let turn = scanner.number(),
                    let large = scanner.flag(), let sweep = scanner.flag(), let to = point()
                else { return path }
                SVGArc.add(to: &path, from: current, to: to, radii: CGSize(width: rx, height: ry), turn: turn,
                           large: large, sweep: sweep)
                current = to
            case UInt8(ascii: "z"):
                path.closeSubpath(); current = start
            default:
                return path
            }
            (lastCubic, lastQuad) = (cubic, quad)
        }
        return path
    }

    private static func reflect(_ control: CGPoint?, about point: CGPoint) -> CGPoint {
        control.map { CGPoint(x: 2 * point.x - $0.x, y: 2 * point.y - $0.y) } ?? point
    }

    /// Reads numbers and commands from a path string. SVG packs them without separators (`1.5-2`, `.5.5`, and arc
    /// flags as single digits), so each read knows what it expects.
    private struct Scanner {
        let bytes: [UInt8]
        var at = 0

        init(bytes: [UInt8]) { self.bytes = bytes }

        mutating func skipSeparators() {
            while at < bytes.count, bytes[at] == 32 || bytes[at] == 44 || (9...13).contains(bytes[at]) { at += 1 }
        }

        /// The next command letter; a number straight after a command repeats the one before it.
        mutating func command(after previous: UInt8) -> UInt8? {
            skipSeparators()
            guard at < bytes.count else { return nil }
            let byte = bytes[at]
            if (byte | 0x20) >= 97 && (byte | 0x20) <= 122 && byte != UInt8(ascii: "e") && byte != UInt8(ascii: "E") {
                at += 1
                return byte
            }
            return previous == 0 ? nil : previous
        }

        mutating func number() -> CGFloat? {
            skipSeparators()
            let begin = at
            if at < bytes.count, bytes[at] == 43 || bytes[at] == 45 { at += 1 }
            var dot = false
            while at < bytes.count {
                let byte = bytes[at]
                if (48...57).contains(byte) { at += 1 } else if byte == 46, !dot { dot = true; at += 1 } else { break }
            }
            if at < bytes.count, bytes[at] | 0x20 == UInt8(ascii: "e") {
                var look = at + 1
                if look < bytes.count, bytes[look] == 43 || bytes[look] == 45 { look += 1 }
                if look < bytes.count, (48...57).contains(bytes[look]) {
                    while look < bytes.count, (48...57).contains(bytes[look]) { look += 1 }
                    at = look
                }
            }
            guard at > begin, let value = Double(String(decoding: bytes[begin..<at], as: UTF8.self)) else { return nil }
            return CGFloat(value)
        }

        mutating func flag() -> Bool? {
            skipSeparators()
            guard at < bytes.count, bytes[at] == 48 || bytes[at] == 49 else { return nil }
            defer { at += 1 }
            return bytes[at] == 49
        }
    }
}
