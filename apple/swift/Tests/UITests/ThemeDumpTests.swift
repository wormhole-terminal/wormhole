import DesignSystem
import Foundation
import XCTest

/// `themes.json` is written from `wormhole-core`. That it still is what Rust resolves is checked on the Rust
/// side (`cargo test -p wormhole-ffi --test theme_dump`, and `apple/scripts/dump-themes.sh --check`), since
/// the Swift sources may not spawn processes (`rules.py`); this checks the file is one the loader reads.
final class ThemeDumpTests: XCTestCase {
    private let resource = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        .appendingPathComponent("../../Sources/DesignSystem/Resources/themes.json").standardized

    func testTheBundledFileIsTheFormatTheLoaderReads() throws {
        XCTAssertNoThrow(try ThemeLibrary(json: Data(contentsOf: resource)))
        let wrong = Data(#"{"format": 99, "defaultLook": {"opacity": 1, "blur": false}, "defaultDark": "", "defaultLight": "", "catalog": [], "themes": []}"#.utf8)
        XCTAssertThrowsError(try ThemeLibrary(json: wrong))
    }
}
