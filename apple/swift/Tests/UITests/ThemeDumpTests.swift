import DesignSystem
import Foundation
import XCTest

/// `themes.json` is written from `wormhole-core`; this fails when the committed copy no longer is what
/// Rust makes. It runs only where it can: `cargo` on the PATH and a warm build (`CARGO_TARGET_DIR`, or
/// `apple/target`), so a cold checkout isn't made to compile the app's core inside `swift test`.
/// `apple/scripts/dump-themes.sh --check` is the same check by hand, and the way to refresh the file.
final class ThemeDumpTests: XCTestCase {
    private let apple = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .appendingPathComponent("../../..").standardized

    private func cargo() -> URL? {
        let path = ProcessInfo.processInfo.environment["PATH"] ?? ""
        return path.split(separator: ":").map { URL(fileURLWithPath: String($0)).appendingPathComponent("cargo") }
            .first { FileManager.default.isExecutableFile(atPath: $0.path) }
    }

    private var warm: Bool {
        ProcessInfo.processInfo.environment["CARGO_TARGET_DIR"] != nil
            || FileManager.default.fileExists(atPath: apple.appendingPathComponent("target").path)
    }

    func testThemesJSONIsWhatRustProduces() throws {
        guard let cargo = cargo(), warm else { throw XCTSkip("no cargo or no warm Rust target: run apple/scripts/dump-themes.sh --check") }
        let fresh = FileManager.default.temporaryDirectory.appendingPathComponent("themes-\(UUID().uuidString).json")
        defer { try? FileManager.default.removeItem(at: fresh) }
        let run = Process()
        run.executableURL = cargo
        run.currentDirectoryURL = apple
        run.arguments = ["run", "--quiet", "--offline", "-p", "wormhole-ffi", "--example", "dump_themes", "--", fresh.path]
        run.standardOutput = FileHandle.nullDevice
        try run.run()
        run.waitUntilExit()
        XCTAssertEqual(run.terminationStatus, 0, "dump_themes failed")
        let committed = try Data(contentsOf: apple.appendingPathComponent("swift/Sources/DesignSystem/Resources/themes.json"))
        XCTAssertTrue(try Data(contentsOf: fresh) == committed, "themes.json is stale: run apple/scripts/dump-themes.sh")
    }

    func testTheBundledFileIsTheFormatTheLoaderReads() throws {
        let data = try Data(contentsOf: apple.appendingPathComponent("swift/Sources/DesignSystem/Resources/themes.json"))
        XCTAssertNoThrow(try ThemeLibrary(json: data))
        let wrong = Data(#"{"format": 99, "defaultDark": "", "defaultLight": "", "catalog": [], "themes": []}"#.utf8)
        XCTAssertThrowsError(try ThemeLibrary(json: wrong))
    }
}
