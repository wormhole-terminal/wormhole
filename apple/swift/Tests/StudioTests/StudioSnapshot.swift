// Renders the Studio window offscreen (an NSHostingView in a window that is never shown; ImageRenderer draws
// text fields and scroll views wrongly) to a PNG, in a theme with or without Increase Contrast.

import AppKit
@testable import DesignSystem
import StudioModel
@testable import StudioUI
import SwiftUI
import XCTest

@MainActor
enum StudioSnapshot {
    static let directory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["STUDIO_SNAPSHOTS"]
            ?? NSTemporaryDirectory() + "wormhole-swift-shots/studio")

    /// A Studio on `theme` (the name of a bundled theme), selected on `token`, drawn at `size`.
    @discardableResult
    static func write(
        theme name: String, token: String? = nil, contrast: Bool = false, area: StudioArea = .all, name file: String,
        size: CGSize = CGSize(width: 1344, height: 900), edit: (StudioDocument) -> Void = { _ in }
    ) throws -> URL {
        let fixtures = try StudioFixtures()
        defer { fixtures.remove() }
        let store = ThemeStore.shared
        store.increaseContrast = contrast
        let session = try StudioSession(theme: name, environment: fixtures.environment, store: store)
        session.document.selected = token ?? "primary"
        session.document.filter.area = area
        edit(session.document)
        let appearance = session.document.appearance == .dark ? NSAppearance.Name.darkAqua : .aqua
        let host = NSHostingView(rootView: StudioView(session: session, store: store).frame(width: size.width, height: size.height))
        host.frame = CGRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: appearance)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.6))
        host.layoutSubtreeIfNeeded()
        let rep = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: host.bounds))
        host.cacheDisplay(in: host.bounds, to: rep)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("\(file).png")
        try XCTUnwrap(rep.representation(using: .png, properties: [:])).write(to: url)
        session.document.close()
        store.increaseContrast = false
        return url
    }
}
