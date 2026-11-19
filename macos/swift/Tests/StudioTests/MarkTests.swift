import AppKit
@testable import DesignSystem
@testable import StudioModel
@testable import StudioUI
import SwiftUI
import XCTest

/// The gallery's marks: what the selected token is drawn on is outlined element by element (`Env::mark`), the elements are
/// the ones `GalleryUses` lists for it. (A click on a metric's box selecting it is driven in a live instance: a button in a
/// window that was never ordered in takes no event.)
@MainActor
final class MarkTests: XCTestCase {
    private let size = CGSize(width: 160, height: 140)

    /// Two elements stacked, `a` and `b`, each `side` square: the bitmap of what is drawn with `drawn` marked.
    private func render(_ view: some View, size: CGSize) throws -> NSBitmapImageRep {
        let host = NSHostingView(rootView: view.frame(width: size.width, height: size.height).background(Color.white))
        host.frame = CGRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: .aqua)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.2))
        let rep = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: host.bounds))
        host.cacheDisplay(in: host.bounds, to: rep)
        return rep
    }

    /// The pixels in `rect` (in points) that are not white; the bitmap has a pixel per device pixel, so more than a point's.
    private func inked(_ rep: NSBitmapImageRep, in rect: CGRect) -> Int {
        var count = 0
        let scale = CGFloat(rep.pixelsWide) / size.width
        for y in Int(rect.minY * scale)..<Int(rect.maxY * scale) {
            for x in Int(rect.minX * scale)..<Int(rect.maxX * scale) {
                guard let colour = rep.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else { continue }
                if colour.redComponent < 0.9 || colour.greenComponent < 0.9 || colour.blueComponent < 0.9 { count += 1 }
            }
        }
        return count
    }

    func testOnlyTheElementsTheSelectedTokenIsDrawnOnAreOutlined() throws {
        let stack = VStack(spacing: 48) {
            Color.clear.frame(width: 60, height: 24).marked("a")
            Color.clear.frame(width: 60, height: 24).marked("b")
        }
        // The elements sit centred: a at y 22 to 46, b at y 94 to 118; the outline reaches a few points past each.
        let a = CGRect(x: 40, y: 16, width: 80, height: 36), b = CGRect(x: 40, y: 88, width: 80, height: 36)
        // What a bare render leaves in each region to compare the marked ones with.
        let none = try render(stack.environment(\.tokenMarks, TokenMarks()), size: size)
        let (restA, restB) = (inked(none, in: a), inked(none, in: b))
        let onlyA = try render(stack.environment(\.tokenMarks, TokenMarks(drawn: ["a"])), size: size)
        XCTAssertGreaterThan(inked(onlyA, in: a), restA + 20)
        XCTAssertEqual(inked(onlyA, in: b), restB, "b is not drawn with the selected token")
        let both = try render(stack.environment(\.tokenMarks, TokenMarks(drawn: ["a", "b"])), size: size)
        XCTAssertGreaterThan(inked(both, in: b), restB + 20)
    }

    func testSelectingATokenHandsTheGalleryTheElementsGalleryUsesNames() throws {
        let fixtures = try StudioFixtures()
        defer { fixtures.remove() }
        let document = try fixtures.opening("Nord")
        for token in ["tile.sheen", "syntax.keyword", "terminal.ansi.3", "sidebar.row.selected", "text.primary"] {
            let expected = GalleryUses.elements.filter { $0.draws(with: token) }.map(\.id)
            XCTAssertEqual(document.uses(of: token).map(\.id), expected, token)
        }
        XCTAssertEqual(document.uses(of: "tile.sheen").map(\.id), ["status.tile"])
        XCTAssertEqual(document.uses(of: "syntax.keyword").map(\.id), ["diff", "code"], "the diff's lines are highlighted code")
        XCTAssertTrue(document.uses(of: "terminal.ansi.3").map(\.id).contains("terminal"))
        XCTAssertGreaterThan(document.uses(of: "text.primary").count, 5, "a token many elements use outlines each of them")
    }

    /// Every component the table lists is marked by a view, and every mark is a listed component: the check `gallery.py`
    /// makes, here where `swift test` runs.
    func testEveryListedComponentIsMarkedAndEveryMarkIsListed() throws {
        let sources = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../Sources").standardized
        guard let files = FileManager.default.enumerator(at: sources, includingPropertiesForKeys: nil) else {
            throw XCTSkip("the Swift sources are not beside the tests")
        }
        let pattern = try NSRegularExpression(pattern: #"\.marked\("([^"]+)"[,)]"#)
        var marked = Set<String>()
        for case let url as URL in files where url.pathExtension == "swift" && url.lastPathComponent != "GalleryUses.swift" {
            let text = try String(contentsOf: url, encoding: .utf8)
            for match in pattern.matches(in: text, range: NSRange(text.startIndex..., in: text)) {
                marked.insert((text as NSString).substring(with: match.range(at: 1)))
            }
        }
        XCTAssertEqual(marked, Set(GalleryUses.elements.map(\.id)))
    }
}
