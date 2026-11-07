import SwiftUI
import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

final class SidebarIconTests: XCTestCase {
    private func box(_ d: String) -> CGRect { SVGPath.path(d).boundingRect }

    func testLinesAndRelativeMoves() {
        XCTAssertEqual(box("M10 10h10v10H10z"), CGRect(x: 10, y: 10, width: 10, height: 10))
        XCTAssertEqual(box("m1 2 3 4"), CGRect(x: 1, y: 2, width: 3, height: 4), "pairs after a move are lines")
        XCTAssertEqual(box("M0 0L4 0 4 4"), CGRect(x: 0, y: 0, width: 4, height: 4), "a command repeats")
        XCTAssertEqual(box("M0,0 L-2.5-1.5"), CGRect(x: -2.5, y: -1.5, width: 2.5, height: 1.5), "signs separate numbers")
        XCTAssertEqual(box("M.5.5 1e1 1"), CGRect(x: 0.5, y: 0.5, width: 9.5, height: 0.5), "dots and exponents")
    }

    func testArcsIncludingPackedFlags() {
        // A half turn from (0,0) to (10,0), clockwise on screen: it goes over the top, to y = -5.
        let half = box("M0 0a5 5 0 0 1 10 0")
        XCTAssertEqual(half.minY, -5, accuracy: 0.01)
        XCTAssertEqual(half.width, 10, accuracy: 0.01)
        // The same with its flags written as Lucide writes them, packed.
        XCTAssertEqual(box("M0 0a5 5 0 01 10 0").minY, -5, accuracy: 0.01)
        // The other sweep goes under.
        XCTAssertEqual(box("M0 0a5 5 0 0 0 10 0").maxY, 5, accuracy: 0.01)
        // Too small a radius grows until it spans the points.
        XCTAssertEqual(box("M0 0A1 1 0 0 1 10 0").minY, -5, accuracy: 0.01)
        // A large arc of a circle through two points is the bigger part of it.
        XCTAssertGreaterThan(box("M5 0A5 5 0 1 1 5.001 0").height, 9.9)
    }

    func testCurvesAndTheirReflections() {
        XCTAssertEqual(box("M0 0C0 10 10 10 10 0").maxY, 7.5, accuracy: 0.01)
        XCTAssertEqual(box("M0 0q5 10 10 0").maxY, 5, accuracy: 0.01)
        // S mirrors the last control point: the second hump goes the other way.
        let wave = box("M0 0c2 -4 4 -4 5 0s3 4 5 0")
        XCTAssertLessThan(wave.minY, -2)
        XCTAssertGreaterThan(wave.maxY, 2)
    }

    func testEveryBundledLogoAndIconParsesWithinItsBox() throws {
        XCTAssertEqual(AgentLogo.drawings.count, 12)
        for (key, drawing) in AgentLogo.drawings {
            let bounds = drawing.paths.map(box).reduce(CGRect.null) { $0.union($1) }
            XCTAssertFalse(bounds.isNull, key)
            XCTAssertGreaterThan(bounds.width, 12, key)
            XCTAssertTrue(CGRect(x: -0.5, y: -0.5, width: 25, height: 25).contains(bounds), "\(key) is \(bounds)")
        }
        for (icon, paths) in Lucide.drawings {
            let bounds = paths.map(box).reduce(CGRect.null) { $0.union($1) }
            XCTAssertTrue(CGRect(x: 0, y: 0, width: 24, height: 24).contains(bounds), "\(icon) is \(bounds)")
        }
    }

    func testEveryAgentHasALogoOrALetterPair() {
        for agent in AgentID.allCases {
            XCTAssertEqual(agent.monogram.count, 2, agent.name)
            XCTAssertEqual(agent.monogram, agent.monogram.uppercased())
        }
        XCTAssertEqual(AgentID.allCases.filter { AgentLogo.key($0) == nil }, [.aider, .crush, .droid, .auggie, .fm], "the registry's agents without a logo")
        XCTAssertEqual(AgentID.openCode.name, "OpenCode")
        XCTAssertNotNil(AgentLogo.key(.openCode))
    }

    func testTheBrandColoursAreTheRegistrys() {
        XCTAssertEqual(AgentID.claude.tilePaint, .solid(0xd97757))
        XCTAssertEqual(AgentID.kiro.tilePaint, .solid(0x9046ff))
        XCTAssertEqual(AgentID.cursor.tilePaint, .black)
        XCTAssertEqual(AgentID.allCases.filter { $0.tilePaint != .black }.count, 7)
    }
}
