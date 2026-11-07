// A colour as the Rust token resolution made it: straight (not premultiplied) sRGB, 0...1 per channel,
// kept as the floats the GPUI app draws so a token here is the same colour there.

import AppKit
import SwiftUI

public struct RGBA: Sendable, Hashable {
    public let red: Double
    public let green: Double
    public let blue: Double
    public let alpha: Double

    public init(_ red: Double, _ green: Double, _ blue: Double, _ alpha: Double = 1) {
        (self.red, self.green, self.blue, self.alpha) = (red, green, blue, alpha)
    }

    public var color: Color { Color(.sRGB, red: red, green: green, blue: blue, opacity: alpha) }
    public var nsColor: NSColor { NSColor(srgbRed: red, green: green, blue: blue, alpha: alpha) }
    public var cgColor: CGColor { nsColor.cgColor }

    /// The 8-bit channels a hex string would show, as the Rust side rounds them.
    public var hex: String {
        let byte = { (value: Double) in Int((min(max(value, 0), 1) * 255).rounded()) }
        let text = String(format: "#%02x%02x%02x", byte(red), byte(green), byte(blue))
        return alpha < 1 ? text + String(format: "%02x", byte(alpha)) : text
    }

    /// `self` over an opaque `background`, as the screen composites it.
    public func over(_ background: RGBA) -> RGBA {
        RGBA(
            red * alpha + background.red * (1 - alpha), green * alpha + background.green * (1 - alpha),
            blue * alpha + background.blue * (1 - alpha), 1)
    }
}

extension RGBA: Decodable {
    public init(from decoder: any Decoder) throws {
        var channels = try decoder.unkeyedContainer()
        self.init(
            try channels.decode(Double.self), try channels.decode(Double.self),
            try channels.decode(Double.self), try channels.decode(Double.self))
    }
}
