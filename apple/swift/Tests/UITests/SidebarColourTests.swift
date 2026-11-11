@testable import DesignSystem
import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

/// The tests of `wormhole-core` `theme/folder_color.rs`, `view/sidebar/group_color.rs` and `tokens/color.rs`, run on the
/// Swift port over every bundled theme (the library holds the same tokens the Rust tests walk) and both contrast sets.
final class SidebarColourTests: XCTestCase {
    private let library = ThemeLibrary.bundled
    private var allTokens: [(name: String, tokens: ThemeTokens)] {
        library.themes.flatMap { theme in [(theme.id, theme.tokens()), (theme.id + "+contrast", theme.tokens(increaseContrast: true))] }
    }

    private func apart(_ a: Double, _ b: Double) -> Double {
        let d = abs(a - b).truncatingRemainder(dividingBy: 360)
        return min(d, 360 - d)
    }

    private func rgba(_ tokens: ThemeTokens, _ name: String) throws -> RGBA { try XCTUnwrap(tokens.rgba(named: name), name) }

    func testTheMathMatchesTheKnownValues() {
        let (white, black) = (RGBA(1, 1, 1), RGBA(0, 0, 0))
        XCTAssertEqual(RGBA.contrast(white, black), 21, accuracy: 1e-9)
        XCTAssertEqual(white.oklab.l, 1, accuracy: 1e-4)
        XCTAssertEqual(RGBA(1, 0, 0).oklch.l, 0.6279, accuracy: 1e-3, "OKLab red")
        XCTAssertEqual(RGBA(1, 0, 0).oklch.hue, 29.23, accuracy: 0.05)
        for hex in [RGBA(0.1, 0.4, 0.8), RGBA(0.9, 0.7, 0.2), RGBA(0.5, 0.5, 0.5)] {
            let (l, c, h) = hex.oklch
            let back = RGBA.oklch(l, c, h)
            XCTAssertEqual(back.red, hex.red, accuracy: 2e-3)
            XCTAssertEqual(back.green, hex.green, accuracy: 2e-3)
            XCTAssertEqual(back.blue, hex.blue, accuracy: 2e-3)
        }
        XCTAssertEqual(RGBA.oklch(0, 0.2, 100).hex, "#000000", "chroma vanishes at the extremes")
        XCTAssertEqual(RGBA.oklch(1, 0.2, 100).hex, "#ffffff")
    }

    func testReadableMovesOnlyAsFarAsItTakes() {
        let background = RGBA(0.05, 0.05, 0.1)
        let dim = RGBA(0.2, 0.2, 0.3)
        let moved = dim.readable(on: background, ratio: 4.5)
        XCTAssertGreaterThanOrEqual(RGBA.contrast(moved, background), 4.5 - 0.01)
        XCTAssertLessThan(RGBA.contrast(moved, background), 4.6, "the least move")
        XCTAssertEqual(moved.oklch.hue, dim.oklch.hue, accuracy: 6, "hue kept")
        XCTAssertEqual(RGBA(1, 1, 1).readable(on: background, ratio: 4.5), RGBA(1, 1, 1), "already readable stays")
    }

    func testANameIsTheHueItSaysInEveryThemeAndTheEightAreToldApart() throws {
        for (name, tokens) in allTokens {
            let all = GroupColor.allCases.filter { $0 != .grey }.sorted { $0.hue < $1.hue }.map { ($0, $0.mark(tokens).oklch) }
            for (colour, lch) in all {
                XCTAssertLessThan(apart(lch.hue, colour.hue), 8, "\(name): \(colour) is \(lch.hue) deg")
                XCTAssertGreaterThanOrEqual(lch.chroma, 0.05, "\(name): \(colour) is \(lch.chroma)")
            }
            for (a, b) in zip(all, all.dropFirst() + [all[0]]) {
                XCTAssertGreaterThanOrEqual(apart(a.1.hue, b.1.hue), 25, "\(name): \(a.0) and \(b.0)")
            }
        }
    }

    func testAColourIsOpaqueAndSeenOnTheWindowAndTheSidebar() throws {
        for (name, tokens) in allTokens {
            for colour in GroupColor.allCases {
                let shown = colour.mark(tokens)
                XCTAssertEqual(shown.alpha, 1)
                for surface in ["surface.window", "surface.panel"] {
                    let ratio = RGBA.contrast(shown, try rgba(tokens, surface).withAlpha(1))
                    XCTAssertGreaterThanOrEqual(ratio, GroupWash.markContrast - 0.05, "\(name): \(colour) on \(surface) is \(ratio)")
                }
            }
        }
    }

    func testTextOnAGroupsWashReadsInEveryTheme() throws {
        for (name, tokens) in allTokens {
            let card = try XCTUnwrap(GroupWash.backdrop(tokens))
            for colour in GroupColor.allCases {
                let wash = colour.wash(tokens).withAlpha(GroupWash.peak).over(card)
                for (token, ratio) in GroupWash.text {
                    let got = RGBA.contrast(try rgba(tokens, token).over(wash), wash)
                    XCTAssertGreaterThanOrEqual(got, ratio - 0.05, "\(name): \(token) on a \(colour) wash is \(got)")
                }
            }
        }
    }

    func testAWashKeepsItsColourAndOnlyGivesUpLightness() throws {
        var moved = 0
        for (name, tokens) in allTokens {
            for colour in GroupColor.allCases where colour != .grey {
                let (mark, wash) = (colour.mark(tokens), colour.wash(tokens))
                if mark == wash { continue }
                moved += 1
                XCTAssertLessThan(apart(mark.oklch.hue, wash.oklch.hue), 12, "\(name): \(colour)")
                XCTAssertGreaterThan(wash.oklch.chroma, 0.4 * mark.oklch.chroma, "\(name): \(colour)")
            }
        }
        XCTAssertGreaterThan(moved, 0, "some colour on some theme is too bright or dark for its wash")
    }

    func testTheColoursFollowTheThemeAndItsLightOrDarkSide() throws {
        func blue(_ id: String) throws -> RGBA { GroupColor.blue.mark(try XCTUnwrap(library.theme(id: id)).tokens()) }
        XCTAssertLessThan(try blue("catppuccin_latte").luminance, try blue("catppuccin_mocha").luminance, "a light theme gets the darker blue")
        XCTAssertNotEqual(try blue("nord").hex, try blue("dracula").hex)
        XCTAssertLessThan(try blue("nord").oklch.chroma, try blue("dracula").oklch.chroma, "Nord is muted, Dracula is not")
    }

    func testAGreyPaletteStillGivesColoursThatDiffer() throws {
        let base = try XCTUnwrap(library.theme(id: "wormhole.dark")).tokens()
        let grey = ThemeTokens(
            terminal: TerminalPalette(
                foreground: base.terminal.foreground, background: base.terminal.background, cursor: base.terminal.cursor,
                cursorText: nil, selectionBackground: base.terminal.selectionBackground, selectionForeground: nil,
                ansi: base.terminal.ansi.enumerated().map { $0 >= 1 && $0 <= 6 ? RGBA(0.54, 0.54, 0.54) : $1 }),
            metrics: base.metrics, ui: base.ui, syntax: base.syntax)
        let (cyan, green) = (GroupColor.cyan.mark(grey).oklch, GroupColor.green.mark(grey).oklch)
        XCTAssertGreaterThanOrEqual(cyan.chroma, GroupWash.chromaMin - 0.01)
        XCTAssertGreaterThanOrEqual(green.chroma, GroupWash.chromaMin - 0.01)
        XCTAssertGreaterThanOrEqual(apart(cyan.hue, green.hue), 25)
    }

    func testAColourOfOnesOwnIsMovedOnlyWhereTheThemeWouldHideIt() throws {
        for (name, tokens) in allTokens {
            for custom in [RGBA(0, 0, 0), RGBA(1, 1, 1), RGBA(0.5, 0.5, 0.5), RGBA(1, 1, 0), RGBA(0, 0, 1), RGBA(1, 0, 0)] {
                let shown = GroupWash.legible(custom, tokens)
                for surface in ["surface.window", "surface.panel"] {
                    XCTAssertGreaterThanOrEqual(RGBA.contrast(shown, try rgba(tokens, surface).withAlpha(1)), 2.95, "\(name): \(custom.hex) on \(surface)")
                }
            }
        }
    }
}
