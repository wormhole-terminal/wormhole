// The colours programs set for themselves (OSC 4 an indexed entry, OSC 10, 11 and 12 the foreground, background and
// cursor, and their resets), as the frame carries them (`frame_colors.rs`, layout version 2): the whole table when it
// changed, drawn over the theme's palette, so every cell, the padding behind the grid and the cursor take them, as the
// GPUI app resolves them (`palette.rs` `resolve`).

import Foundation

/// One palette entry a program set: its slot (0...255 indexed, 256 foreground, 257 background, 258 cursor) and colour.
public struct ProgramColor: Sendable, Equatable {
    public var slot: Int
    public var rgb: SIMD3<UInt8>

    public init(slot: Int, rgb: SIMD3<UInt8>) { (self.slot, self.rgb) = (slot, rgb) }
}

enum ProgramColors {
    static let flag: UInt32 = 32

    /// The section's records (an empty list: every colour is the theme's again).
    static func read(_ words: inout FrameWords) throws(FrameError) -> [ProgramColor] {
        var colors: [ProgramColor] = []
        for _ in 0..<(try words.next()) {
            let (slot, packed) = (try words.next(), try words.next())
            colors.append(ProgramColor(slot: slot, rgb: SIMD3(UInt8(packed >> 16 & 0xFF), UInt8(packed >> 8 & 0xFF), UInt8(packed & 0xFF))))
        }
        return colors
    }
}
