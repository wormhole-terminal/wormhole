import AppKit
import DesignSystem
import SwiftUI
import Testing
import WorkspaceModel

@testable import WindowUI

/// The one fill (`one_background`): which colour each column paints and which lines stay, over a theme store of the test's own.
@MainActor
@Suite struct OneFillTests {
    private func look(increaseContrast: Bool = false) -> WindowUI.WindowLook {
        let themes = ThemeStore(follows: false)
        themes.increaseContrast = increaseContrast
        return WindowUI.WindowLook(themes: themes)
    }

    @Test func itIsOnUntilTurnedOff() {
        #expect(WindowUI.WindowLook().oneBackground)
        #expect(WindowUI.WindowLook().backdropSummary.contains("one_fill=true"))
    }

    @Test func withItOnEveryColumnPaintsTheOneFillAndOffEachItsOwn() {
        let look = look()
        let colours = ChromeColours(tokens: look.themes.tokens)
        for own in [colours.sidebar, colours.panel, colours.terminal] {
            #expect(look.columnSurface(own, colours) == look.surface(colours.oneFill))
        }
        look.oneBackground = false
        #expect(look.columnSurface(colours.sidebar, colours) == look.surface(colours.sidebar))
        #expect(look.columnSurface(colours.panel, colours) == look.surface(colours.panel))
        #expect(look.columnSurface(colours.terminal, colours) == look.surface(colours.terminal))
        #expect(look.backdropSummary.contains("one_fill=false"))
    }

    @Test func withItOnNoLineIsDrawnWhateverColumnDividersSays() {
        let look = look()
        look.columnDividers = true
        #expect(!look.drawsColumnDividers && !look.drawsStatusRule)
        look.columnDividers = false
        #expect(!look.drawsColumnDividers && !look.drawsStatusRule)
    }

    @Test func underIncreaseContrastTheLinesComeBackBecauseTheTintIsGone() {
        let look = look(increaseContrast: true)
        look.columnDividers = false
        #expect(look.drawsColumnDividers && look.drawsStatusRule)
    }

    @Test func withItOffTheDividersFollowTheirSettingAndTheStatusBarKeepsItsRule() {
        let look = look()
        look.oneBackground = false
        look.columnDividers = true
        #expect(look.drawsColumnDividers && look.drawsStatusRule)
        look.columnDividers = false
        #expect(!look.drawsColumnDividers)
        #expect(look.drawsStatusRule, "the rule is the status bar's own, whatever Column dividers says")
    }

    @Test func liquidGlassKeepsWinningWhereItDoesToday() {
        let look = look()
        look.liquidGlass = true
        look.columnDividers = true
        guard look.glassOn else { return }  // before macOS 26 there is no glass to win with
        for one in [true, false] {
            look.oneBackground = one
            #expect(!look.drawsColumnDividers && !look.drawsStatusRule, "one fill \(one): the glass has no lines")
        }
    }

    // MARK: The window drawn

    private static let size = CGSize(width: 1200, height: 720)
    /// Points of the 1200 x 720 window where only a surface is drawn: the sidebar and the panel below their lists, the strip of the
    /// terminal's column between the sidebar and the card (the sidebar is 350 wide, the card inset 8), the status bar's middle.
    private static let sidebar = CGPoint(x: 100, y: 500)
    private static let strip = CGPoint(x: 354, y: 500)
    private static let panel = CGPoint(x: 1000, y: 500)
    private static let statusBar = CGPoint(x: 600, y: 706)
    private static let card = CGPoint(x: 600, y: 400)

    /// The window drawn offscreen with opaque surfaces (Reduce Transparency), so each colour is the theme's own, exactly.
    private func render(oneBackground: Bool, increaseContrast: Bool = false) throws -> NSBitmapImageRep {
        let themes = ThemeStore(follows: false)
        themes.mode = .dark
        themes.reduceTransparency = true
        themes.increaseContrast = increaseContrast
        let look = WindowUI.WindowLook(themes: themes)
        look.statusBar = true
        look.oneBackground = oneBackground
        let terminals = [
            TerminalSummary(id: 1, title: "zsh", folder: "~/Projects/wormhole", branch: "main"),
            TerminalSummary(id: 2, title: "agent", folder: "~/Projects/wormhole", branch: "exp/swift-ui", activity: .working),
        ]
        let state = WorkspaceState(terminals: terminals, selected: 1)
        let backend = MockWorkspaceBackend(
            items: state.terminals.map { .loose(MockTerminal($0)) }, selected: state.selected,
            chrome: ChromeState(sidebarVisible: true, panelVisible: true, panelTab: .changes))
        let hub = WindowHub(store: WorkspaceStore(backend: backend), terminals: TerminalViews { _ in nil })
        let hosting = NSHostingView(rootView: WorkspaceRootView(hub: hub, look: look))
        hosting.sizingOptions = []
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: Self.size), styleMask: [.titled, .fullSizeContentView], backing: .buffered, defer: false)
        window.titlebarAppearsTransparent = true
        window.contentView = hosting
        hosting.frame = NSRect(origin: .zero, size: Self.size)
        hosting.layoutSubtreeIfNeeded()
        let bitmap = try #require(hosting.bitmapImageRepForCachingDisplay(in: hosting.bounds))
        hosting.cacheDisplay(in: hosting.bounds, to: bitmap)
        return bitmap
    }

    /// The colour at a point of the window (in points: the bitmap is in pixels, twice as many on a Retina Mac).
    private func rgb(_ bitmap: NSBitmapImageRep, _ point: CGPoint) -> [Int] {
        let scale = CGFloat(bitmap.pixelsWide) / Self.size.width
        let colour = bitmap.colorAt(x: Int(point.x * scale), y: Int(point.y * scale))?.usingColorSpace(.sRGB)
        return [colour?.redComponent, colour?.greenComponent, colour?.blueComponent].map { Int(((($0 ?? -1) * 255).rounded())) }
    }

    private func same(_ a: [Int], _ b: [Int]) -> Bool { zip(a, b).allSatisfy { abs($0 - $1) <= 1 } }

    @Test func withItOnTheFourPartsAreOneColourAndNoLineLiesBetweenThem() throws {
        let bitmap = try render(oneBackground: true)
        let fill = rgb(bitmap, Self.sidebar)
        for (name, point) in [("strip", Self.strip), ("panel", Self.panel), ("status bar", Self.statusBar)] {
            #expect(same(rgb(bitmap, point), fill), "\(name) is the sidebar's colour")
        }
        #expect(!same(rgb(bitmap, Self.card), fill), "the card stays the one raised surface")
        // A row across the window below the lists: the sidebar, the strip, the card, the strip, the panel. Only the card may differ.
        let line = (0..<Int(Self.size.width)).filter { !(352...838).contains($0) }
        #expect(line.allSatisfy { same(rgb(bitmap, CGPoint(x: CGFloat($0), y: Self.sidebar.y)), fill) }, "no divider between the columns")
        let bar = (0..<Int(Self.size.width)).filter { $0 > 24 && $0 < Int(Self.size.width) - 24 }
        #expect(bar.allSatisfy { same(rgb(bitmap, CGPoint(x: CGFloat($0), y: 692)), rgb(bitmap, CGPoint(x: CGFloat($0), y: 706))) }, "no rule above the status bar")
    }

    @Test func withItOffEachColumnKeepsItsOwnColourAndTheLinesStay() throws {
        let bitmap = try render(oneBackground: false)
        #expect(!same(rgb(bitmap, Self.strip), rgb(bitmap, Self.sidebar)), "the terminal's column is not the sidebar's colour")
        let divider = [349, 350, 351].map { rgb(bitmap, CGPoint(x: CGFloat($0), y: Self.sidebar.y)) }
        #expect(divider.contains { !same($0, rgb(bitmap, Self.sidebar)) }, "the sidebar's divider")
    }

    @Test func underIncreaseContrastTheLinesComeBackAndTheFillStaysOne() throws {
        let bitmap = try render(oneBackground: true, increaseContrast: true)
        let fill = rgb(bitmap, Self.sidebar)
        for point in [Self.strip, Self.panel, Self.statusBar] { #expect(same(rgb(bitmap, point), fill)) }
        let divider = [349, 350, 351].map { rgb(bitmap, CGPoint(x: CGFloat($0), y: Self.sidebar.y)) }
        #expect(divider.contains { !same($0, fill) }, "the sidebar's divider is back")
        let rule = [691, 692, 693].map { rgb(bitmap, CGPoint(x: 600, y: CGFloat($0))) }
        #expect(rule.contains { !same($0, fill) }, "and the status bar's rule")
    }
}
