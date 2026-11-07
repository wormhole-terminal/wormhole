@testable import DesignSystem
import Foundation
@testable import StudioModel
import XCTest

@MainActor
final class ListTests: XCTestCase {
    func testEveryCatalogueTokenIsInExactlyOneBand() throws {
        let groups = TokenGroup.all(syntaxRoots: CoreKnowledge().syntaxRoots(among: ThemeLibrary.bundled.themes[0].tokens().syntaxKeys))
        let listed = groups.flatMap(\.tokens)
        for info in ThemeLibrary.bundled.catalog { XCTAssertEqual(listed.filter { $0 == info.name }.count, 1, info.name) }
        XCTAssertEqual(groups.map(\.name).suffix(3), ["Terminal", "Terminal ANSI", "Syntax"])
        let syntax = try XCTUnwrap(groups.last).tokens
        XCTAssertTrue(syntax.contains("syntax.keyword") && !syntax.contains("syntax.keyword.control"), "it follows keyword")
    }

    func testTheFilterKeepsWhatShowQueryAndAreaKeep() {
        let groups = [TokenGroup(name: "Chrome", tokens: ["sidebar.row.selected", "band"]), TokenGroup(name: "Text", tokens: ["text.link"])]
        let facts = TokenFilter.Facts(
            origin: { $0 == "band" ? .yours : .workedOut }, low: { $0 == "text.link" },
            inArea: { name, area in area == .sidebar && name.hasPrefix("sidebar") })
        var filter = TokenFilter()
        XCTAssertEqual(filter.visible(groups, facts: facts), groups)
        filter.show = .yours
        XCTAssertEqual(filter.visible(groups, facts: facts).map(\.tokens), [["band"]])
        filter.show = .lowContrast
        XCTAssertEqual(filter.visible(groups, facts: facts).map(\.tokens), [["text.link"]])
        filter = TokenFilter()
        filter.query = "ROW.SEL"
        XCTAssertEqual(filter.visible(groups, facts: facts).map(\.tokens), [["sidebar.row.selected"]])
        filter.query = "text"
        XCTAssertEqual(filter.visible(groups, facts: facts).map(\.name), ["Text"], "a band's own name keeps all its tokens")
        filter = TokenFilter()
        filter.area = .sidebar
        XCTAssertEqual(filter.visible(groups, facts: facts).map(\.tokens), [["sidebar.row.selected"]])
        XCTAssertTrue(filter.isFiltering)
    }

    func testRulesAreCoresAndFollowersAreFoundFromThem() {
        let knowledge = CoreKnowledge()
        XCTAssertEqual(knowledge.rule(for: "state.selected", appearance: .dark), "primary / 14%")
        XCTAssertEqual(knowledge.rule(for: "surface.panel", appearance: .light), "mix(surface.window, #000, 4%)")
        XCTAssertEqual(knowledge.rule(for: "metrics.radius_chip", appearance: .dark), "radius_sm")
        XCTAssertEqual(knowledge.rule(for: "agent.bell", appearance: .dark), "#ff9e64")
        XCTAssertNil(knowledge.rule(for: "terminal.background", appearance: .dark))
        let followers = knowledge.followers(of: "primary.hover", among: ["checkbox.checked", "primary", "agent.working"], appearance: .dark)
        XCTAssertEqual(followers, [])
        XCTAssertTrue(knowledge.followers(of: "primary", among: ["checkbox.checked", "state.selected", "surface.panel"], appearance: .dark)
            .contains("checkbox.checked"))
    }

    func testTheVersionPolicyThinsByAge() {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        func ago(_ seconds: Double) -> Date { now.addingTimeInterval(-seconds) }
        let saves: [(Date, Bool)] = [
            (ago(300), false), (ago(1200), false), (ago(3 * 3600 + 600), false), (ago(3 * 3600 + 300), false),
            (ago(5 * 3600), false), (ago(2 * 86_400), false), (ago(40 * 86_400), false), (ago(400 * 86_400), true),
        ]
        let keep = VersionPolicy.keep(saves, now: now)
        XCTAssertEqual(Array(keep[..<5]), [true, true, false, true, true])
        XCTAssertTrue(keep[5])
        XCTAssertEqual(Array(keep[6...]), [false, true], "too old goes, the pinned original stays")
        XCTAssertEqual(VersionPolicy.keep((0..<300).map { (ago(Double($0)), false) }, now: now).filter { $0 }.count, 120)
    }

    func testFileNames() {
        XCTAssertEqual(ThemeFileNames.fileName(for: "Soltari / Night"), "Soltari  Night.json")
        XCTAssertEqual(ThemeFileNames.exportName("Tokyo Night Custom"), "tokyo-night-custom.json")
        XCTAssertEqual(ThemeFileNames.exportName("  Dusk / 2 "), "dusk-2.json")
        XCTAssertEqual(ThemeFileNames.exportName("…"), "theme.json")
        XCTAssertEqual(ThemeFileNames.copyName("Dusk") { _ in false }, "Dusk copy")
        XCTAssertEqual(ThemeFileNames.copyName("Dusk") { $0 == "Dusk copy" || $0 == "Dusk copy 2" }, "Dusk copy 3")
    }
}
