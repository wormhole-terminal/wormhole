import XCTest

@testable import FontPickerModel

/// The list as core's rules make it: the same cases as `theme/fonts/mod.rs`'s tests, through the Swift mirror.
@MainActor
final class ListTests: XCTestCase {
    private func names(_ model: FontPickerModel) -> [String] { model.entries.compactMap { $0.family }.filter { !$0.isEmpty } }

    /// A picker that has been opened, as the list only exists for an open one.
    private func opened(_ slot: String = "terminal", family: String = "Menlo", library: FontLibrary = Fixtures.library()) -> FontPickerModel {
        let (model, _) = Fixtures.picker(slot: slot, family: family, library: library)
        model.isOpen = true
        return model
    }

    func testHiddenFamiliesAreLeftOutAndEachFamilyIsListedOnce() {
        let model = opened()
        model.showsAll = true
        let listed = names(model)
        XCTAssertFalse(listed.contains(".SF NS"))
        XCTAssertEqual(listed.filter { $0 == "Fira Code" }.count, 1, "installed and custom: once")
        XCTAssertEqual(listed.filter { $0 == "JetBrains Mono" }.count, 1, "installed and bundled: once")
    }

    func testBundledThenCustomThenInstalledWithCaptions() {
        let model = opened()
        model.showsAll = true
        XCTAssertEqual(
            model.entries.map(\.id).filter { $0.hasPrefix("caption") },
            ["caption-source(FontPickerModel.FontSource.bundled)", "caption-source(FontPickerModel.FontSource.custom)", "caption-source(FontPickerModel.FontSource.installed)"])
        let sources = model.entries.compactMap { entry -> FontSource? in if case .family(let c, _) = entry { c.source } else { nil } }
        XCTAssertEqual(sources, sources.sorted())
        XCTAssertEqual(names(model).prefix(3), ["JetBrains Mono", "Fira Code", "Iosevka"])
    }

    func testTerminalListsMonospaceUntilAllIsOnAndInterfaceListsAll() {
        let terminal = opened()
        XCTAssertFalse(names(terminal).contains("Arial"))
        XCTAssertTrue(names(terminal).contains("Menlo"))
        terminal.showsAll = true
        XCTAssertTrue(names(terminal).contains("Arial"))
        let interface = opened("interface", family: "")
        XCTAssertTrue(names(interface).contains("Arial"), "no monospace filter for the interface font")
    }

    func testNothingIsFilteredWhileMonospaceIsStillBeingFound() {
        let model = opened(library: Fixtures.library(Fixtures.catalog(classified: false)))
        XCTAssertTrue(names(model).contains("Arial"))
        XCTAssertEqual(model.status, FontWords.findingMonospace)
    }

    func testSearchMatchesEveryWordAndRanksPrefixesFirst() {
        let model = opened()
        model.showsAll = true
        model.query = "mon"
        XCTAssertEqual(names(model), ["JetBrains Mono", "Monaco", "Monaspace Neon"], "bundled group first, then the prefix matches")
        model.query = "  neon MONA "
        XCTAssertEqual(names(model), ["Monaspace Neon"])
        model.query = "zzz"
        XCTAssertTrue(names(model).isEmpty)
        XCTAssertNotNil(model.emptyNote)
    }

    func testTheInheritRowMatchesItsLabelOnly() {
        let model = opened("code", family: "")
        XCTAssertEqual(model.entries.first, .inherit("Same as terminal"))
        model.query = "terminal"
        XCTAssertEqual(model.entries, [.inherit("Same as terminal")])
        model.query = "menlo"
        XCTAssertFalse(model.entries.contains(.inherit("Same as terminal")))
    }

    func testTheCountCountsFamiliesOnly() {
        let model = opened("code", family: "")
        XCTAssertEqual(model.status, FontWords.count(model.familyCount))
        XCTAssertEqual(model.familyCount, names(model).count)
    }
}
