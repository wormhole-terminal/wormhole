@testable import DesignSystem
@testable import StudioModel
import XCTest

final class ContrastTests: XCTestCase {
    func testWCAGRatios() {
        XCTAssertEqual(ContrastCheck.ratio(RGBA(0, 0, 0), RGBA(1, 1, 1)), 21, accuracy: 0.001)
        XCTAssertEqual(ContrastCheck.ratio(RGBA(1, 1, 1), RGBA(1, 1, 1)), 1, accuracy: 0.001)
        // #767676 on white is the textbook 4.54:1.
        XCTAssertEqual(ContrastCheck.ratio(RGBA(0x76 / 255, 0x76 / 255, 0x76 / 255), RGBA(1, 1, 1)), 4.54, accuracy: 0.01)
    }

    /// The pair list is core's: compare it with the Rust file while the FFI doesn't hand it over.
    func testThePairsAreTheOnesCoreChecks() throws {
        let rust = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("../../../../crates/wormhole-core/src/theme/tokens/contrast.rs").standardized
        guard let text = try? String(contentsOf: rust, encoding: .utf8) else { throw XCTSkip("the Rust sources are not beside this package") }
        let body = text.components(separatedBy: "pub const CONTRAST_PAIRS")[1].components(separatedBy: "impl ThemeTokens")[0]
        let rows = body.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }.filter { $0.hasPrefix("pair(") || $0.hasPrefix("on(") }
        XCTAssertEqual(rows.count, ContrastPair.all.count)
        for (row, pair) in zip(rows, ContrastPair.all) {
            let args = row.drop { $0 != "(" }.dropFirst().dropLast(2).split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces.union(.init(charactersIn: "\""))) }
            let isOn = row.hasPrefix("on(")
            XCTAssertEqual([pair.fg, pair.bg], [args[0], args[1]], row)
            XCTAssertEqual(pair.underlay, isOn ? args[2] : "surface.window", row)
            XCTAssertEqual(pair.min, Double(args[isOn ? 3 : 2]), row)
        }
    }

    func testEveryBundledThemeMeasuresAndKeepsItsTextReadableExceptTheTerminalsOwnPair() throws {
        for theme in ThemeLibrary.bundled.themes {
            let checks = ContrastCheck.all(in: theme.tokens())
            XCTAssertEqual(checks.count, ContrastPair.all.count, "\(theme.id) lacks a token a pair names")
            let failing = checks.filter { !$0.passes && !$0.fg.hasPrefix("terminal.") }.map { "\($0.fg) on \($0.bg) \($0.ratio)" }
            XCTAssertTrue(failing.isEmpty, "\(theme.id): \(failing)")
        }
    }

    func testTheTokensWithALowPairAreFoundForTheListsFilter() throws {
        let theme = try XCTUnwrap(ThemeLibrary.bundled.theme(id: "nord"))
        let weak = try XCTUnwrap(MockTokens.with(theme.tokens(), ["text.faint": RGBA(0.18, 0.2, 0.25)]))
        XCTAssertTrue(ContrastCheck.lowTokens(in: weak).contains("text.faint"))
        XCTAssertFalse(ContrastCheck.lowTokens(in: theme.tokens()).contains("text.faint"))
    }
}
