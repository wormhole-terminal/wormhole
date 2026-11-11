import AppKit
import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

/// Writes what the input overlays look like (selection, hovered link, composed text, hollow cursor, the scroll
/// bar) in a few themes to `WORMHOLE_VISUAL_DIR`, to be looked at; nothing is written without it. The themes
/// are the design system's own (`themes.json`, resolved by core).
@MainActor
@Suite struct VisualTests {
    nonisolated private static var directory: URL? { ProcessInfo.processInfo.environment["WORMHOLE_VISUAL_DIR"].map { URL(fileURLWithPath: $0) } }

    private static func palette(_ theme: [String: Any]) -> TerminalPalette? {
        guard let terminal = theme["terminal"] as? [String: Any] else { return nil }
        func rgb(_ value: Any?) -> SIMD3<UInt8>? {
            guard let c = value as? [Double], c.count >= 3 else { return nil }
            return SIMD3(UInt8((c[0] * 255).rounded()), UInt8((c[1] * 255).rounded()), UInt8((c[2] * 255).rounded()))
        }
        guard let ansi = (terminal["ansi"] as? [[Double]])?.compactMap({ rgb($0) }), ansi.count == 16, let fg = rgb(terminal["foreground"]),
            let bg = rgb(terminal["background"]), let cursor = rgb(terminal["cursor"])
        else { return nil }
        return TerminalPalette(
            ansi: ansi, foreground: fg, background: bg, cursor: cursor, selection: rgb(terminal["selectionBackground"]),
            selectionText: rgb(terminal["selectionForeground"]))
    }

    private static func themes(_ ids: [String]) throws -> [(String, TerminalPalette)] {
        let file = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("Sources/DesignSystem/Resources/themes.json")
        let json = try JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any]
        let all = (json?["themes"] as? [[String: Any]]) ?? []
        return ids.compactMap { id in all.first { $0["id"] as? String == id }.flatMap(palette).map { (id, $0) } }
    }

    @Test(.enabled(if: directory != nil)) func selectionLinkCompositionAndCursorInThreeThemes() throws {
        let directory = try #require(Self.directory)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let text = "~/wormhole $ git log --oneline\r a1439f7 selection follows the offset\r 85424c4 see https://example.com/docs now\r ok\r"
        for (id, palette) in try Self.themes(["tokyo_night", "wormhole.light", "wormhole.dark"]) {
            let harness = try ViewHarness(cols: 52, rows: 8, text: text, palette: palette)
            harness.drag(from: harness.point(row: 1, column: 1, half: 0.25), to: harness.point(row: 1, column: 18, half: 0.75))
            let link = harness.point(row: 2, column: 20)
            harness.view.mouseMoved(with: harness.mouse(.mouseMoved, at: link, flags: .command))
            harness.view.setMarkedText("にほんご", selectedRange: NSRange(location: 2, length: 2), replacementRange: NSRange(location: NSNotFound, length: 0))
            harness.view.drawFrame(force: true)
            try harness.view.renderer.renderToImage().writePNG(to: directory.appendingPathComponent("\(id)-focused.png"))
            harness.view.renderer.setFocused(false)
            try harness.view.renderer.renderToImage().writePNG(to: directory.appendingPathComponent("\(id)-unfocused.png"))
        }
    }

    @Test(.enabled(if: directory != nil)) func theScrollerInBothStylesAndAppearances() throws {
        let directory = try #require(Self.directory)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        for (name, appearance) in [("light", NSAppearance.Name.aqua), ("dark", .darkAqua)] {
            for style in [NSScroller.Style.overlay, .legacy] {
                let scroller = TerminalScroller()
                scroller.view.scrollerStyle = style
                scroller.view.isHidden = false
                scroller.adapt(toBackground: appearance == .darkAqua ? SIMD3(26, 26, 26) : SIMD3(247, 247, 247))
                scroller.layout(in: CGRect(x: 0, y: 0, width: 60, height: 200))
                scroller.update(history: 300, rows: 40, offset: 120)
                // Drawn straight into a bitmap, on the appearance's own background: a scroller in a view that is
                // never on screen does not draw itself.
                let rep = try #require(
                    NSBitmapImageRep(
                        bitmapDataPlanes: nil, pixelsWide: 120, pixelsHigh: 400, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                        isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
                rep.size = NSSize(width: 60, height: 200)
                let context = try #require(NSGraphicsContext(bitmapImageRep: rep))
                NSAppearance(named: appearance)!.performAsCurrentDrawingAppearance {
                    NSGraphicsContext.saveGraphicsState()
                    NSGraphicsContext.current = context
                    (appearance == .darkAqua ? NSColor(white: 0.1, alpha: 1) : NSColor(white: 0.97, alpha: 1)).setFill()
                    NSRect(x: 0, y: 0, width: 60, height: 200).fill()
                    let transform = NSAffineTransform()
                    transform.translateX(by: scroller.view.frame.minX, yBy: 0)
                    transform.concat()
                    scroller.view.draw(scroller.view.bounds)
                    NSGraphicsContext.restoreGraphicsState()
                }
                let png = try #require(rep.representation(using: .png, properties: [:]))
                try png.write(to: directory.appendingPathComponent("scroller-\(name)-\(style == .overlay ? "overlay" : "legacy").png"))
            }
        }
    }
}
