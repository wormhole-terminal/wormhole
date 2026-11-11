// The colour table the shader resolves packed colours against: slots 0..255 are the indexed colours
// (0..15 the ANSI ones, which named slots 0..15 share), then foreground, background and cursor.
// The spike's theme is a fixed dark one; with the real FFI it comes from core's theme (`set_palette`).

import Foundation

public struct TerminalPalette: Sendable, Equatable {
    public static let slotCount = 269

    /// RGB, 0...255, in slot order.
    public private(set) var slots: [SIMD3<UInt8>]

    public var foreground: SIMD3<UInt8> { slots[256] }
    public var background: SIMD3<UInt8> { slots[257] }
    public var cursor: SIMD3<UInt8> { slots[258] }
    public var selectionBackground: SIMD3<UInt8> { slots[259] }
    /// Text on selected cells; when the theme sets none (`keepsTextInSelection`) the cell's own colour stays.
    public var selectionForeground: SIMD3<UInt8> { slots[260] }
    public let keepsTextInSelection: Bool
    /// The theme names a colour for the character under a block cursor (`PackedColor.cursorText`).
    public let hasCursorText: Bool
    /// The current find match's fill and its text (slots the shader resolves), and the tint every other match
    /// lays over its cell's background (blended on the CPU: it is translucent).
    public var findCurrent: SIMD3<UInt8> { slots[261] }
    public var findCurrentText: SIMD3<UInt8> { slots[262] }
    public let findMatch: SIMD3<UInt8>
    public let findMatchAlpha: Float
    /// A Probe's tones (`ProbeTone`), slots 263...267: the text a rule paints; a hook's tint of a block is blended on the CPU.
    public func tone(_ tone: ProbeTone) -> SIMD3<UInt8> { slots[PackedColor.toneBase + tone.rawValue] }
    /// How much of its tone a block a hook marked takes of the cell's own background (`TONE_TINT` in the GPUI palette): faint,
    /// so the text keeps its colours.
    static let toneTint: Float = 0.16

    /// The colours of find's matches (`Palette::find_match`, `find_current`, `find_current_text`; the app takes them
    /// from the `find.match*` design tokens).
    public struct FindColors: Sendable, Equatable {
        public var match: SIMD3<UInt8>
        public var matchAlpha: Float
        public var current: SIMD3<UInt8>
        public var currentText: SIMD3<UInt8>

        public init(match: SIMD3<UInt8>, matchAlpha: Float, current: SIMD3<UInt8>, currentText: SIMD3<UInt8>) {
            (self.match, self.matchAlpha, self.current, self.currentText) = (match, matchAlpha, current, currentText)
        }
    }

    /// `selection` and `selectionText` are the theme's selection colours; without a background one the
    /// foreground over the background at a third is used, readable under any text. Without `find`, a bare theme's
    /// rule: yellow (ANSI 3) at 32 % for the matches, solid for the current one, in black (ANSI 0).
    public init(
        ansi: [SIMD3<UInt8>], foreground: SIMD3<UInt8>, background: SIMD3<UInt8>, cursor: SIMD3<UInt8>,
        selection: SIMD3<UInt8>? = nil, selectionText: SIMD3<UInt8>? = nil, find: FindColors? = nil,
        tones: [SIMD3<UInt8>]? = nil, cursorText: SIMD3<UInt8>? = nil
    ) {
        precondition(ansi.count == 16)
        var table = ansi
        // The 6x6x6 colour cube and the 24-step grey ramp are fixed by xterm, not by a theme.
        let steps: [UInt8] = [0, 95, 135, 175, 215, 255]
        for r in 0..<6 {
            for g in 0..<6 {
                for b in 0..<6 { table.append(SIMD3(steps[r], steps[g], steps[b])) }
            }
        }
        for step in 0..<24 {
            let level = UInt8(8 + step * 10)
            table.append(SIMD3(level, level, level))
        }
        table.append(foreground)
        table.append(background)
        table.append(cursor)
        table.append(selection ?? Self.mix(foreground, over: background, amount: 0.33))
        table.append(selectionText ?? foreground)
        let find = find ?? FindColors(match: ansi[3], matchAlpha: 0.32, current: ansi[3], currentText: ansi[0])
        table.append(find.current)
        table.append(find.currentText)
        // Without the theme's tokens: the closest of its own colours (link blue, red, yellow, green, grey), as a bare theme does.
        table.append(contentsOf: tones ?? [4, 1, 3, 2, 8].map { ansi[$0] })
        // The character inside a block cursor: the theme's colour, else the cell's own background (`hasCursorText`).
        table.append(cursorText ?? background)
        hasCursorText = cursorText != nil
        findMatch = find.match
        findMatchAlpha = find.matchAlpha
        keepsTextInSelection = selectionText == nil
        slots = table
    }

    private static func mix(_ top: SIMD3<UInt8>, over bottom: SIMD3<UInt8>, amount: Float) -> SIMD3<UInt8> {
        let t = SIMD3<Float>(top), b = SIMD3<Float>(bottom)
        let m = b + (t - b) * amount
        return SIMD3(UInt8(m.x.rounded()), UInt8(m.y.rounded()), UInt8(m.z.rounded()))
    }

    /// The spike's own dark theme (not core's; see the file's note).
    public static let spike = TerminalPalette(
        ansi: [
            SIMD3(0x1d, 0x1f, 0x21), SIMD3(0xcc, 0x66, 0x66), SIMD3(0xb5, 0xbd, 0x68), SIMD3(0xf0, 0xc6, 0x74),
            SIMD3(0x81, 0xa2, 0xbe), SIMD3(0xb2, 0x94, 0xbb), SIMD3(0x8a, 0xbe, 0xb7), SIMD3(0xc5, 0xc8, 0xc6),
            SIMD3(0x96, 0x98, 0x96), SIMD3(0xde, 0x93, 0x5f), SIMD3(0xa3, 0xbe, 0x8c), SIMD3(0xeb, 0xcb, 0x8b),
            SIMD3(0x8f, 0xa1, 0xb3), SIMD3(0xc9, 0x9e, 0xd8), SIMD3(0x96, 0xcb, 0xfe), SIMD3(0xff, 0xff, 0xff),
        ],
        foreground: SIMD3(0xc5, 0xc8, 0xc6), background: SIMD3(0x1d, 0x1f, 0x21), cursor: SIMD3(0xf0, 0xc6, 0x74))

    /// A non-current match's background over a cell's own: the tint blended, as truecolour.
    public func tinted(_ background: UInt32) -> UInt32 {
        let under = SIMD3<Float>(resolve(background)), over = SIMD3<Float>(findMatch)
        let mixed = under + (over - under) * findMatchAlpha
        return PackedColor.rgb(UInt8(mixed.x.rounded()), UInt8(mixed.y.rounded()), UInt8(mixed.z.rounded()))
    }

    /// A cell's background under a hook's tint for its block: the tone over the background, as truecolour.
    public func tinted(_ background: UInt32, tone: ProbeTone) -> UInt32 {
        let under = SIMD3<Float>(resolve(background)), over = SIMD3<Float>(self.tone(tone))
        let mixed = under + (over - under) * Self.toneTint
        return PackedColor.rgb(UInt8(mixed.x.rounded()), UInt8(mixed.y.rounded()), UInt8(mixed.z.rounded()))
    }

    /// The table as the shader reads it: `float4` per slot, premultiplied alpha 1, sRGB values 0...1.
    public func floats() -> [SIMD4<Float>] {
        slots.map { SIMD4(Float($0.x) / 255, Float($0.y) / 255, Float($0.z) / 255, 1) }
    }

    /// Resolve a packed colour on the CPU (the screenshot tests, the mock's checks).
    public func resolve(_ packed: UInt32) -> SIMD3<UInt8> {
        switch packed >> 24 {
        case 2: return SIMD3(UInt8(packed >> 16 & 0xFF), UInt8(packed >> 8 & 0xFF), UInt8(packed & 0xFF))
        default: return slots[Int(packed & 0xFFFF) % Self.slotCount]
        }
    }
}
