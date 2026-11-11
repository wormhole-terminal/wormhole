import AppKit
import SwiftUI

/// A SwiftUI view in a real (never shown) window, so its pixels can be read back: `ImageRenderer` skips
/// the containers and the appearance a hosting view gets.
@MainActor
final class ThemeRender {
    let host: NSHostingView<AnyView>
    private let window: NSWindow

    init(_ view: some View, size: CGSize, dark: Bool) {
        host = NSHostingView(rootView: AnyView(view.frame(width: size.width, height: size.height)))
        host.frame = CGRect(origin: .zero, size: size)
        window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
        window.contentView = host
    }

    /// Lets SwiftUI apply what changed since the last frame (an Observation change is applied on the next turn).
    func settle() {
        RunLoop.current.run(until: Date().addingTimeInterval(0.15))
        host.layoutSubtreeIfNeeded()
    }

    func bitmap() throws -> NSBitmapImageRep {
        settle()
        let cached = try XCTUnwrapRep(host.bitmapImageRepForCachingDisplay(in: host.bounds))
        host.cacheDisplay(in: host.bounds, to: cached)
        return cached
    }

    func png(to url: URL) throws {
        let rep = try bitmap()
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        guard let data = rep.representation(using: .png, properties: [:]) else { throw CocoaError(.fileWriteUnknown) }
        try data.write(to: url)
    }

    /// The pixel at a point of the view (top left origin), converted to sRGB 0...1. The bitmap is Display P3;
    /// AppKit converts back, so a token drawn as sRGB reads as the same numbers within rounding.
    func pixel(x: Int, y: Int) throws -> NSColor {
        let rep = try bitmap()
        let scale = Double(rep.pixelsWide) / host.bounds.width
        guard let color = rep.colorAt(x: Int(Double(x) * scale), y: Int(Double(y) * scale))?.usingColorSpace(.sRGB)
        else { throw CocoaError(.fileReadUnknown) }
        return color
    }

    private func XCTUnwrapRep(_ rep: NSBitmapImageRep?) throws -> NSBitmapImageRep {
        guard let rep else { throw CocoaError(.fileWriteUnknown) }
        return rep
    }
}
