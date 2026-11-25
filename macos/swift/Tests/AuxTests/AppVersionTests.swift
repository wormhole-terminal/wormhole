import XCTest

@testable import AuxModel

final class AppVersionTests: XCTestCase {
    func testABundleThatNamesItsFullVersionIsThatVersion() {
        // The beta: the short version is the same for every beta and sorts above all of them.
        let info: [String: Any] = ["CFBundleShortVersionString": "2.0.0", "WormholeVersion": "2.0.0-beta.1"]
        XCTAssertEqual(AppInfo.version(in: info), "2.0.0-beta.1")
    }

    func testABundleWithoutTheKeyIsItsShortVersionAsBefore() {
        XCTAssertEqual(AppInfo.version(in: ["CFBundleShortVersionString": "1.6.3"]), "1.6.3")
        XCTAssertEqual(AppInfo.version(in: [:]), "0")
    }

    func testAboutSaysTheFullVersionOfABundle() throws {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("app-version-\(UUID().uuidString.prefix(8))")
        defer { try? FileManager.default.removeItem(at: folder) }
        let bundle = folder.appendingPathComponent("Beta.bundle")
        try FileManager.default.createDirectory(at: bundle.appendingPathComponent("Contents"), withIntermediateDirectories: true)
        let info: [String: Any] = [
            "CFBundleIdentifier": "dev.wormhole.test", "CFBundleName": "Wormhole Beta", "CFBundleShortVersionString": "2.0.0",
            "WormholeVersion": "2.0.0-beta.1",
        ]
        try PropertyListSerialization.data(fromPropertyList: info, format: .xml, options: 0)
            .write(to: bundle.appendingPathComponent("Contents/Info.plist"))
        let app = AppInfo.from(bundle: try XCTUnwrap(Bundle(url: bundle)))
        XCTAssertEqual(app.name, "Wormhole Beta")
        XCTAssertEqual(app.version, "2.0.0-beta.1")
        XCTAssertTrue(app.versionLine.hasPrefix("Version 2.0.0-beta.1"), app.versionLine)
    }
}
