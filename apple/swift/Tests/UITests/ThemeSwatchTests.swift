import DesignSystem
import SwiftUI
import XCTest

/// Writes one swatch sheet per bundled theme (its own appearance), and the defaults again under Increase
/// Contrast, to a folder to look at: `THEME_SWATCHES` or `out/theme-swatches`.
@MainActor
final class ThemeSwatchTests: XCTestCase {
    private var directory: URL {
        URL(fileURLWithPath: ProcessInfo.processInfo.environment["THEME_SWATCHES"]
            ?? FileManager.default.currentDirectoryPath + "/out/theme-swatches")
    }

    func testWriteASheetPerTheme() throws {
        let library = ThemeLibrary.bundled
        var sheets = library.themes.map { ($0, false) }
        sheets += [library.defaultDark, library.defaultLight].compactMap { library.theme(id: $0) }.map { ($0, true) }
        for (theme, contrast) in sheets {
            let sheet = ThemeSwatchSheet(theme: theme, contrast: contrast, catalog: library.catalog)
            // ImageRenderer keeps the tokens' sRGB numbers exactly; a hosting view's bitmap is Display P3 and
            // washes them out. The sheet has no scroll view or text field, which is all it can't draw.
            let renderer = ImageRenderer(content: sheet.environment(\.colorScheme, theme.appearance.colorScheme))
            renderer.scale = 2
            let cgImage = try XCTUnwrap(renderer.cgImage)
            let png = try XCTUnwrap(NSBitmapImageRep(cgImage: cgImage).representation(using: .png, properties: [:]))
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let url = directory.appendingPathComponent("\(theme.id)\(contrast ? "-contrast" : "").png")
            try png.write(to: url)
            XCTAssertTrue(FileManager.default.fileExists(atPath: url.path), theme.id)
        }
    }
}
