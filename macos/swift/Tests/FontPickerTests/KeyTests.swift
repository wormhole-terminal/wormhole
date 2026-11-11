import XCTest

@testable import FontPickerModel

/// The open list's keys, the recents and the preview's family.
@MainActor
final class KeyTests: XCTestCase {
    func testOpeningHighlightsTheFamilyInUseAndScrollsToIt() {
        let (model, _) = Fixtures.picker(family: "Menlo")
        model.isOpen = true
        XCTAssertEqual(model.highlighted, "family-list-Menlo")
        XCTAssertEqual(model.scrollTarget, model.highlighted)
    }

    func testArrowsSkipCaptionsAndStopAtTheEnds() {
        let (model, _) = Fixtures.picker(family: "JetBrains Mono")
        model.isOpen = true
        model.move(-1)
        XCTAssertEqual(model.highlighted, "family-list-JetBrains Mono", "already first")
        model.move(1)
        XCTAssertEqual(model.highlighted, "family-list-Fira Code", "the custom caption is passed over")
        model.moveToEdge(first: false)
        let last = model.entries.last { $0.isSelectable }
        XCTAssertEqual(model.highlighted, last?.id)
        model.move(1)
        XCTAssertEqual(model.highlighted, last?.id)
    }

    func testReturnPicksTheHighlightedRowAndClosesAndRemembersIt() {
        let (model, picked) = Fixtures.picker(family: "Menlo")
        model.isOpen = true
        model.move(1)
        model.commitHighlighted()
        XCTAssertEqual(picked.family, "Monaco")
        XCTAssertFalse(model.isOpen)
        XCTAssertEqual(model.library.recents.families(for: "terminal"), ["Monaco"])
    }

    func testRecentsAreAFewNewestFirstWithoutRepeatsAndOnlyOnAnEmptySearch() {
        let recents = MemoryFontRecents()
        let (model, _) = Fixtures.picker(library: Fixtures.library(recents: recents))
        for name in ["Menlo", "Monaco", "Iosevka", "Fira Code", "Menlo", "Monaco"] { recents.remember(name, for: "terminal") }
        XCTAssertEqual(recents.families(for: "terminal"), ["Monaco", "Menlo", "Fira Code", "Iosevka"])
        model.isOpen = true
        XCTAssertEqual(model.entries.first { $0.isSelectable == false }, .caption(.recent))
        let recent = model.entries.compactMap { entry -> String? in if case .family(let c, .recent?) = entry { c.name } else { nil } }
        XCTAssertEqual(recent, ["Monaco", "Menlo", "Fira Code", "Iosevka"])
        XCTAssertEqual(Set(model.entries.map(\.id)).count, model.entries.count, "ids stay unique with a family listed twice")
        model.query = "mo"
        XCTAssertFalse(model.entries.contains(.caption(.recent)))
    }

    func testThePreviewFollowsTheHighlightAndFallsBackToWhatTheRowComesTo() {
        let (model, _) = Fixtures.picker(slot: "code", family: "")
        XCTAssertEqual(model.previewFamily, "Menlo", "an empty family comes to the terminal font")
        model.isOpen = true
        model.hover(model.entries.first { $0.family == "Iosevka" })
        XCTAssertEqual(model.previewFamily, "Iosevka")
        XCTAssertNotEqual(model.scrollTarget, model.highlighted, "the pointer does not scroll the list")
        model.isOpen = false
        XCTAssertEqual(model.previewFamily, "Menlo")
    }

}
