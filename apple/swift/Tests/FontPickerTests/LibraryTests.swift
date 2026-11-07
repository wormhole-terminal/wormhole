import XCTest

@testable import FontPickerModel

/// What the library does with the system's answers, in the two steps GPUI shows ("Loading…", "Finding monospace…").
@MainActor
final class LibraryTests: XCTestCase {
    private struct Rules: FontRules {
        let bundledFamily = "JetBrains Mono"
        let bundledFiles: [URL] = []
        var custom = CustomFonts(files: [URL(fileURLWithPath: "/tmp/Wormhole-swift-x/Iosevka.ttf")], families: [CustomFamily(name: "Iosevka", monospace: true)])
        func scanCustomFonts() -> CustomFonts { custom }
        func choices(in catalog: FontCatalog, query: String, monospaceOnly: Bool) -> [FontChoice] {
            LocalFontRules().choices(in: catalog, query: query, monospaceOnly: monospaceOnly)
        }
    }

    func testLoadingListsFamiliesThenClassifiesThem() async {
        let system = StubSystemFonts(installed: [
            "Menlo": FontTraits(monospace: true, symbolic: false), "Arial": FontTraits(monospace: false, symbolic: false),
            "Zapfino": FontTraits(monospace: false, symbolic: true),
        ])
        let library = FontLibrary(rules: Rules(), system: system)
        XCTAssertTrue(library.catalog.isLoading)
        await library.load()
        let catalog = library.catalog
        XCTAssertEqual(catalog.installed, ["Arial", "Menlo", "Zapfino"])
        XCTAssertEqual(catalog.monospace, ["Menlo", "Iosevka", "JetBrains Mono"], "the bundled and custom monospace families are known without asking")
        XCTAssertEqual(catalog.symbolic, ["Zapfino"])
        XCTAssertEqual(catalog.customNames, ["Iosevka"])
        XCTAssertFalse(catalog.drawsOwnName("Zapfino"))
    }

    func testACustomFontOfTheBundledNameReplacesTheBundledOne() async {
        var rules = Rules()
        rules.custom = CustomFonts(families: [CustomFamily(name: "JetBrains Mono", monospace: true)])
        let library = FontLibrary(rules: rules, system: StubSystemFonts())
        await library.load()
        XCTAssertEqual(library.catalog.bundled, [])
        let listed = rules.choices(in: library.catalog, query: "", monospaceOnly: false)
        XCTAssertEqual(listed, [FontChoice(source: .custom, name: "JetBrains Mono")])
    }
}
