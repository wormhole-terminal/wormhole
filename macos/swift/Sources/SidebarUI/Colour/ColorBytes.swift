// A colour as the core keeps a group's free pick: `#rrggbb`, three bytes.

import AppKit
import SwiftUI

extension Color {
    /// `#rrggbb` (the form the core writes a custom group colour in).
    init?(hex: String) {
        guard hex.hasPrefix("#"), hex.count == 7, let value = UInt32(hex.dropFirst(), radix: 16) else { return nil }
        self.init(.sRGB, red: Double(value >> 16 & 0xff) / 255, green: Double(value >> 8 & 0xff) / 255, blue: Double(value & 0xff) / 255)
    }

    /// The colour's sRGB bytes, as a picker's pick is sent to the core.
    var rgb8: (UInt8, UInt8, UInt8)? {
        guard let rgb = NSColor(self).usingColorSpace(.sRGB) else { return nil }
        func byte(_ value: CGFloat) -> UInt8 { UInt8((min(max(value, 0), 1) * 255).rounded()) }
        return (byte(rgb.redComponent), byte(rgb.greenComponent), byte(rgb.blueComponent))
    }
}
