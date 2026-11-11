// What the mock draws: synthetic output shaped like the loads the renderer will meet, deterministic for a
// seed so a screenshot can be compared run to run. They are stand-ins, not `ttystorm`: their numbers
// say what the renderer does with these shapes, never what it does under the real program.

import Foundation

/// A small deterministic generator (SplitMix64); `SystemRandomNumberGenerator` would make runs differ.
struct SplitMix64: RandomNumberGenerator {
    private var state: UInt64
    init(seed: UInt64) { state = seed }
    mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }
}

public enum MockScenario: String, Sendable, CaseIterable {
    /// One new line of coloured words a step, scrolling the whole screen: every row changes every step.
    case text
    /// Every cell new every step: half blocks in two truecolors each, like a full-screen animation
    /// (`ttystorm`'s wormhole and plasma draw this way).
    case flood
    /// Every cell new every step with a random character: the worst case for a word cache.
    case noise
    /// Falling katakana in double-width cells, a few cells changing per row (`ttystorm matrix`).
    case matrix
    /// A gallery of wide characters, emoji, box drawing, blocks, braille and every attribute, static.
    case unicode
    /// The cursor sweeping the screen writing as it goes (`ttystorm`'s cursor storm).
    case cursor
    /// Double-width and double-height rows, ligatures, blinking text: what the gallery leaves out, static.
    case styles
    /// The gallery on top and scrolling text in a region below it: partial damage, the way a TUI does it.
    case mixed

    /// Rows above the scroll region in `mixed`.
    static let galleryRows = 16

    func prepare(_ screen: inout MockScreen) {
        switch self {
        case .unicode, .mixed:
            MockGallery.draw(into: &screen)
            if self == .mixed {
                screen.cursorRow = min(screen.rows - 1, Self.galleryRows)
                screen.cursorColumn = 0
            }
        case .styles:
            MockStyles.draw(into: &screen)
            screen.cursorVisible = false
        case .text:
            screen.cursorRow = screen.rows - 1
        case .cursor:
            screen.cursorShape = .beam
        case .flood, .noise, .matrix:
            screen.cursorVisible = false
        }
    }

    /// Advance one step (`tick` counts them).
    func step(_ screen: inout MockScreen, rng: inout SplitMix64, tick: Int) {
        switch self {
        case .text: MockText.line(&screen, rng: &rng, top: 0)
        case .flood: MockFlood.step(&screen, rng: &rng, tick: tick, halfBlocks: true)
        case .noise: MockFlood.step(&screen, rng: &rng, tick: tick, halfBlocks: false)
        case .matrix: MockMatrix.step(&screen, rng: &rng, tick: tick)
        case .unicode: MockGallery.spinner(&screen, tick: tick)
        case .styles: break
        case .cursor: MockCursorStorm.step(&screen, rng: &rng, tick: tick)
        case .mixed:
            MockGallery.spinner(&screen, tick: tick)
            MockText.line(&screen, rng: &rng, top: Self.galleryRows)
        }
    }
}

enum MockText {
    static let words = [
        "terminal", "wormhole", "render", "cell", "frame", "glyph", "atlas", "metal", "quad", "damage",
        "scroll", "cursor", "shell", "prompt", "fn", "let", "struct", "async", "await", "return", "0x2f",
        "/usr/bin/env", "--flag", "error:", "warning:", "ok", "42", "3.14159", "match", "impl",
    ]

    static func line(_ screen: inout MockScreen, rng: inout SplitMix64, top: Int) {
        let bottom = screen.rows - 1
        screen.cursorRow = max(screen.cursorRow, top)
        screen.cursorColumn = 0
        var column = 0
        let limit = screen.cols - 1
        while column < limit {
            let word = words[Int(rng.next() % UInt64(words.count))]
            if column + word.count >= limit { break }
            let pick = rng.next() % 16
            screen.pen = PackedCell(
                fg: pick < 8 ? PackedColor.foreground : PackedColor.named(Int(pick % 7) + 1),
                flags: pick == 15 ? .bold : pick == 14 ? .underline : [])
            screen.write(word + " ", top: top, bottom: bottom)
            column += word.count + 1
        }
        screen.pen = PackedCell()
        screen.write("\n", top: top, bottom: bottom)
    }
}

enum MockFlood {
    static let glyphs = Array("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ#@%&*+=-:.".unicodeScalars)

    static func step(_ screen: inout MockScreen, rng: inout SplitMix64, tick: Int, halfBlocks: Bool) {
        let t = Double(tick) * 0.07
        for r in 0..<screen.rows {
            for c in 0..<screen.cols {
                let x = Double(c) / Double(screen.cols), y = Double(r) / Double(screen.rows)
                let v = sin(x * 9 + t) + sin(y * 7 - t * 1.3) + sin((x + y) * 5 + t * 0.7)
                let hue = (v + 3) / 6
                let bg = wheel(hue, level: 0.45)
                let fg = wheel(hue + 0.5, level: 1.0)
                let glyph = halfBlocks ? Unicode.Scalar(0x2580)! : glyphs[Int(rng.next() % UInt64(glyphs.count))]
                screen[r, c] = PackedCell(scalar: glyph.value, fg: fg, bg: bg)
            }
        }
    }

    /// A packed truecolor from a position on a colour wheel.
    static func wheel(_ position: Double, level: Double) -> UInt32 {
        let p = position - floor(position)
        func channel(_ offset: Double) -> UInt8 {
            let v = 0.5 + 0.5 * cos((p + offset) * 2 * .pi)
            return UInt8(max(0, min(255, v * level * 255)))
        }
        return PackedColor.rgb(channel(0), channel(-1.0 / 3), channel(-2.0 / 3))
    }
}

enum MockMatrix {
    static let katakana = Array("ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄ".unicodeScalars) + Array("アイウエオカキクケコサシスセソタチツテトナニヌネノ".unicodeScalars)

    static func step(_ screen: inout MockScreen, rng: inout SplitMix64, tick: Int) {
        // One drop per pair of columns (each wide cell takes two); heads and speeds follow from the column
        // and the tick, so nothing is stored between steps.
        let slots = screen.cols / 2
        for slot in 0..<slots {
            let column = slot * 2
            let speed = 1 + slot % 3
            let period = screen.rows + 12
            let head = ((tick * speed) / 2 + slot * 7) % period
            for trail in 0..<10 {
                let row = head - trail
                guard row >= 0, row < screen.rows else { continue }
                let level = trail == 0 ? 255 : max(30, 220 - trail * 22)
                let scalar = katakana[Int(rng.next() % UInt64(katakana.count))].value
                let fg = trail == 0 ? PackedColor.rgb(200, 255, 200) : PackedColor.rgb(0, UInt8(level), 40)
                if CellWidth.of(scalar) == 2 {
                    screen.put(scalar, row: row, column: column, fg: fg)
                } else {
                    screen.put(scalar, row: row, column: column, fg: fg)
                    screen.put(0x20, row: row, column: column + 1)
                }
            }
            let cleared = head - 10
            if cleared >= 0, cleared < screen.rows {
                screen.put(0x20, row: cleared, column: column)
                screen.put(0x20, row: cleared, column: column + 1)
            }
        }
    }
}

enum MockCursorStorm {
    static func step(_ screen: inout MockScreen, rng: inout SplitMix64, tick: Int) {
        for _ in 0..<64 {
            let pick = rng.next()
            screen.pen = PackedCell(fg: PackedColor.named(Int(pick >> 8) % 7 + 1))
            screen.cursorRow = Int(pick >> 16) % screen.rows
            screen.cursorColumn = Int(pick >> 32) % screen.cols
            screen.write(String(UnicodeScalar(UInt8(0x21 + Int(pick >> 48) % 90))))
        }
        screen.pen = PackedCell()
    }
}
