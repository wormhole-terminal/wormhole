@testable import StudioModel
import XCTest

/// The pairs, the ratios and the fixes are core's (`contrast_checks`, `contrast_issues`; core's tests hold the maths and
/// the bundled themes). What stays here is how the Studio reads core's one answer.
final class ContrastTests: XCTestCase {
    func testTheListsMarksAndTheEditorsChecksComeFromTheSameReport() {
        let low = ContrastCheck(fg: "text.faint", bg: "surface.window", underlay: "surface.window", min: 3, ratio: 1.4)
        let fine = ContrastCheck(fg: "text.primary", bg: "surface.window", underlay: "surface.window", min: 4.5, ratio: 12)
        let fill = ContrastCheck(fg: "text.on_primary", bg: "primary", underlay: "surface.window", min: 4.5, ratio: 6)
        let report = ContrastReport(checks: [fine, low, fill], issues: [ContrastIssue(check: low)])
        XCTAssertEqual(report.lowTokens, ["text.faint"])
        XCTAssertEqual(report.checks(of: "surface.window"), [fine, low], "a token is listed as the fill too")
        XCTAssertEqual(report.checks(of: "primary"), [fill])
        XCTAssertFalse(low.passes)
        XCTAssertTrue(fine.passes)
    }
}
