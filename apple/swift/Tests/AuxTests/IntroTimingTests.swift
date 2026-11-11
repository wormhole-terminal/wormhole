import XCTest
@testable import AuxModel

final class IntroTimingTests: XCTestCase {
    func testEveryStyleEndsWithTheWindowAtRest() {
        for style in IntroStyle.allCases {
            let end = style.windowMotion(at: 1)
            XCTAssertEqual(end.scale, 1, accuracy: 1e-4, "\(style)")
            XCTAssertEqual(end.alpha, 1, "\(style)")
        }
    }

    func testThePageIsInBeforeTheRunIsOver() {
        for style in IntroStyle.allCases {
            XCTAssertLessThanOrEqual(style.enteredBy, style.seconds, "\(style)")
            XCTAssertEqual(style.entrance(at: style.enteredBy), .done, "\(style)")
        }
        for style in IntroStyle.allCases where style != .glow {
            XCTAssertEqual(style.entrance(at: style.windowArrives), .none, "\(style)")
        }
    }

    func testTheShipDocksOnTheLogoPointingRight() {
        let screen: SIMD2<Float> = [1440, 900]
        let logo = Landing(centre: [720, 330], size: 96)
        let docked = Voyage.ship(at: Voyage.land, screen: screen, landing: logo)
        XCTAssertEqual(docked.pos.x, logo.chevron.x, accuracy: 0.5)
        XCTAssertEqual(docked.pos.y, logo.chevron.y, accuracy: 0.5)
        XCTAssertEqual(docked.heading, Float.pi / 2, accuracy: 0.01)
        XCTAssertEqual(Voyage.ship(at: 0, screen: screen, landing: logo).pos.y > screen.y, true)
    }

    func testReduceMotionAlwaysGlowsAndThePortalHasNoShip() {
        for style in IntroStyle.onTrial { XCTAssertEqual(style.withMotion(reduceMotion: true), .glow) }
        XCTAssertFalse(Handover.portal.ship)
        XCTAssertEqual(Handover.cursor.wait(elapsed: 0, reduceMotion: false), Voyage.land)
        XCTAssertEqual(Handover.cursor.wait(elapsed: 0, reduceMotion: true), 0)
    }

    func testTheShaderGetsEveryMomentByName() {
        let source = ShaderConstants.source
        XCTAssertTrue(source.contains("constant float WORMHOLE_EXIT = 4.6;"), source)
        XCTAssertTrue(source.contains("constant float VOYAGE_LAND = 6.2;"), source)
    }
}
