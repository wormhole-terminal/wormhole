// The renderer's configuration from the options: font, size, line height and scale.

import AppKit
import Foundation
import TerminalMetal

extension Options {
    // MARK: the renderer's configuration

    /// JetBrains Mono at 14 pt with a 1.3 line, as the GPUI app has it, when the font folder can be found;
    /// Menlo otherwise (and then the caller says so).
    func rendererConfiguration() -> (RendererConfiguration, note: String?) {
        let scale = self.scale ?? Double(NSScreen.main?.backingScaleFactor ?? 2)
        if let family = fontFamily {
            return (RendererConfiguration(font: .system(family: family), pointSize: pointSize ?? 13, lineHeight: lineHeight, scale: scale), nil)
        }
        if let directory = fontDirectory ?? Self.findBundledFonts() {
            return (
                RendererConfiguration(
                    font: .jetBrainsMono(directory: directory), pointSize: pointSize ?? 14, lineHeight: lineHeight ?? 1.3, scale: scale),
                nil
            )
        }
        return (
            RendererConfiguration(font: .system(family: "Menlo"), pointSize: pointSize ?? 13, lineHeight: lineHeight, scale: scale),
            "JetBrains Mono not found (set WORMHOLE_FONT_DIR): using Menlo, so sizes differ from the GPUI app"
        )
    }

    /// The bundled fonts: the ones an app bundle ships in its resources, else the repository's, found by walking up from the
    /// executable and from the working folder (a dev build, which has no bundle).
    static func findBundledFonts() -> URL? {
        shippedFonts(in: Bundle.main.resourceURL)
            ?? findInRepository("crates/wormhole-core/assets/fonts/JetBrainsMono", containing: "JetBrainsMono-Regular.ttf")
    }

    /// `Fonts/JetBrainsMono` in an app's resources folder (`macos/scripts/bundle.sh` puts the four faces and the licence there),
    /// when the regular face is in it.
    static func shippedFonts(in resources: URL?) -> URL? {
        guard let folder = resources?.appendingPathComponent("Fonts/JetBrainsMono", isDirectory: true) else { return nil }
        let regular = folder.appendingPathComponent("JetBrainsMono-Regular.ttf")
        return FileManager.default.fileExists(atPath: regular.path) ? folder : nil
    }

    /// `relative` (a folder, or a file when `containing` is nil) inside the repository this was built from.
    static func findInRepository(_ relative: String, containing file: String? = nil) -> URL? {
        var starts = [URL(fileURLWithPath: FileManager.default.currentDirectoryPath)]
        if let executable = Bundle.main.executableURL { starts.append(executable.deletingLastPathComponent()) }
        for start in starts {
            var directory = start.standardizedFileURL
            for _ in 0..<12 {
                let candidate = directory.appendingPathComponent(relative)
                let marker = file.map { candidate.appendingPathComponent($0) } ?? candidate
                if FileManager.default.fileExists(atPath: marker.path) { return candidate }
                directory.deleteLastPathComponent()
            }
        }
        return nil
    }
}
