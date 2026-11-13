@testable import DesignSystem
import SwiftUI
import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

/// Icon motion (`style/icon_motion`): the decision whether it plays, the icons that have one, and that every motion's
/// picture at rest is the plain icon, which is how an icon is drawn before and after it plays.
@MainActor
final class IconMotionTests: XCTestCase {
    func testMotionPlaysOnlyWithAnimateIconsOnAndReduceMotionOff() {
        XCTAssertTrue(IconMotion.plays(animations: true, reduceMotion: false))
        XCTAssertFalse(IconMotion.plays(animations: false, reduceMotion: false), "Animate icons off")
        XCTAssertFalse(IconMotion.plays(animations: true, reduceMotion: true), "Reduce Motion")
        XCTAssertFalse(IconMotion.plays(animations: false, reduceMotion: true))
    }

    func testTheSameIconAnswersTheSameWayWhateverItIsDrawnWith() {
        XCTAssertEqual(IconMotion.of(symbol: Icon.add.rawValue), .quarter)
        XCTAssertEqual(IconMotion.of(.plus), .quarter)
        XCTAssertEqual(IconMotion.of(symbol: Icon.refresh.rawValue), .spin(clockwise: true))
        XCTAssertEqual(IconMotion.of(.refreshCw), .spin(clockwise: true))
        XCTAssertEqual(IconMotion.of(symbol: Icon.reset.rawValue), .spin(clockwise: false))
        XCTAssertEqual(IconMotion.of(symbol: Icon.copy.rawValue), .copied)
        XCTAssertEqual(IconMotion.of(.copy), .copied)
        XCTAssertNil(IconMotion.of(symbol: Icon.trash.rawValue))
        XCTAssertNil(IconMotion.of(.chevronDown))
    }

    /// The pixels of `view` in white on black, 24 points square, and the largest step between two pictures' channels.
    private func pixels(_ view: some View) throws -> [UInt8] {
        let render = ThemeRender(view.foregroundStyle(.white).frame(width: 24, height: 24).background(.black), size: CGSize(width: 24, height: 24), dark: true)
        let rep = try render.bitmap()
        return Array(UnsafeBufferPointer(start: rep.bitmapData, count: rep.bytesPerRow * rep.pixelsHigh))
    }

    /// Two pictures are one drawing when no channel differs by more than rounding's `slack` (a path stroked in pieces anti-aliases
    /// its joins a little differently).
    private func assertSame(_ a: [UInt8], _ b: [UInt8], slack: Int = 6, _ message: String) {
        let step = zip(a, b).map { abs(Int($0) - Int($1)) }.max() ?? 0
        XCTAssertEqual(a.count, b.count, message)
        XCTAssertLessThanOrEqual(step, slack, message)
    }

    func testAMotionsPictureAtRestIsThePlainIcon() throws {
        let plain = { (icon: Lucide) in try self.pixels(LucideView(icon: icon, size: 24)) }
        for icon in [Lucide.gitCompare, .commit, .gauge, .fileText, .folder] {
            assertSame(try pixels(TabGlyph(icon: icon, size: 24, run: 0)), try plain(icon), "\(icon) at rest")
        }
        assertSame(try pixels(LucideView(icon: .copy, size: 24).iconMotion(.copied, run: 0)), try plain(.copy), slack: 0, "copy")
        assertSame(try pixels(LucideView(icon: .plus, size: 24).iconMotion(.quarter, run: 0)), try plain(.plus), slack: 0, "plus")
        assertSame(
            try pixels(LucideView(icon: .columns2, size: 24).iconMotion(.newPane(.right), run: 0)), try plain(.columns2), slack: 0, "split right")
        assertSame(
            try pixels(IconSwap(key: Lucide.rows2) { LucideView(icon: $0, size: 24) }), try plain(.rows2), slack: 0, "a swapped icon at rest")
    }

    func testAChevronTurnedOverIsTheOtherChevron() throws {
        assertSame(
            try pixels(LucideView(icon: .chevronDown, size: 24).menuChevron(open: true)), try pixels(LucideView(icon: .chevronUp, size: 24)),
            "the chevron turned over")
    }

    /// The shared segmented control wears an outline under Increase Contrast and none otherwise.
    func testSegmentedControlIsOutlinedOnlyUnderIncreaseContrast() throws {
        let store = ThemeStore.shared
        let before = store.increaseContrast
        defer { store.increaseContrast = before }
        func shown(_ contrast: Bool) throws -> [UInt8] {
            store.increaseContrast = contrast
            let view = Segmented([("Split", 1), ("Unified", 2)], selection: .constant(1)).padding(8).background(.black)
            let rep = try ThemeRender(view, size: CGSize(width: 160, height: 80), dark: true).bitmap()
            return Array(UnsafeBufferPointer(start: rep.bitmapData, count: rep.bytesPerRow * rep.pixelsHigh))
        }
        XCTAssertNotEqual(try shown(true), try shown(false))
    }

    /// A copy icon that starts its motion as it appears: what a click does through `IconRun.start`.
    private struct Started: View {
        let run = IconRun()
        var body: some View {
            LucideView(icon: .copy, size: 24).iconMotion(.copied, run: run.count).onAppear { run.start() }
        }
    }

    /// The pixels of a hosted `Started` after `seconds` of the run loop, with Animate icons and Reduce Motion as given.
    private func started(after seconds: TimeInterval, animations: Bool, reduceMotion: Bool) throws -> [UInt8] {
        let view = Started().environment(\.iconAnimations, animations).environment(\.reduceMotion, reduceMotion)
            .foregroundStyle(.white).frame(width: 24, height: 24).background(.black)
        let render = ThemeRender(view, size: CGSize(width: 24, height: 24), dark: true)
        RunLoop.current.run(until: Date().addingTimeInterval(seconds))
        let rep = try render.bitmap()
        return Array(UnsafeBufferPointer(start: rep.bitmapData, count: rep.bytesPerRow * rep.pixelsHigh))
    }

    func testAStartPlaysTheMotionAndWithAnimateIconsOffOrReduceMotionItDoesNot() throws {
        let rest = try pixels(LucideView(icon: .copy, size: 24))
        // 0.6 s into the 1.4 s run the check has taken the copy icon's place.
        XCTAssertNotEqual(try started(after: 0.6, animations: true, reduceMotion: false), rest, "plays")
        assertSame(try started(after: 0.6, animations: false, reduceMotion: false), rest, slack: 0, "Animate icons off")
        assertSame(try started(after: 0.6, animations: true, reduceMotion: true), rest, slack: 0, "Reduce Motion")
        assertSame(try started(after: 2.0, animations: true, reduceMotion: false), rest, slack: 0, "ends on the copy icon")
    }

    /// A tab glyph that starts as it appears, as a tab becoming active does.
    private struct TabProbe: View {
        let icon: Lucide
        let run = IconRun()
        var body: some View { TabGlyph(icon: icon, size: 24, run: run.count).onAppear { run.start() } }
    }

    /// Every tab glyph that has a motion shows something other than its icon while it runs, and is the icon again at the end.
    func testEachTabGlyphPlaysThenRestsOnItsIcon() throws {
        for icon in [Lucide.gitCompare, .commit, .gauge, .fileText, .folder] {
            let view = TabProbe(icon: icon).foregroundStyle(.white).frame(width: 24, height: 24).background(.black)
            let render = ThemeRender(view, size: CGSize(width: 24, height: 24), dark: true)
            let rest = try pixels(LucideView(icon: icon, size: 24))
            var differed = false
            for _ in 0..<12 {
                RunLoop.current.run(until: Date().addingTimeInterval(0.04))
                let rep = try render.bitmap()
                differed = differed || Array(UnsafeBufferPointer(start: rep.bitmapData, count: rep.bytesPerRow * rep.pixelsHigh)) != rest
            }
            XCTAssertTrue(differed, "\(icon) plays")
            RunLoop.current.run(until: Date().addingTimeInterval(0.8))
            let rep = try render.bitmap()
            assertSame(Array(UnsafeBufferPointer(start: rep.bitmapData, count: rep.bytesPerRow * rep.pixelsHigh)), rest, "\(icon) ends on its icon")
        }
    }

    /// The group colour under the title row, as a line under Increase Contrast: seen against the window in every theme.
    func testTheTitleLineIsSeenAgainstTheWindowInEveryTheme() throws {
        for theme in ThemeLibrary.bundled.themes {
            for contrast in [false, true] {
                let tokens = theme.tokens(increaseContrast: contrast)
                let window = try XCTUnwrap(tokens.rgba(named: "surface.window")).withAlpha(1)
                for color in GroupColor.allCases {
                    XCTAssertGreaterThanOrEqual(
                        RGBA.contrast(GroupWash.titleLine(color.mark(tokens), tokens), window), 3.0 - 0.02, "\(theme.id) \(color) contrast \(contrast)")
                }
            }
        }
    }
}
