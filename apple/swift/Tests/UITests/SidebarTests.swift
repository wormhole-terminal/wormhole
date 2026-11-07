import AppKit
import DesignSystem
import SwiftUI
import XCTest
// The section and row initialisers are the model's internal ones: the listing builds them, tests build them directly.
@testable import SidebarUI
@testable import WorkspaceModel

@MainActor
final class SidebarTests: XCTestCase {
    /// Where the PNGs go: `out/` is git-ignored; the SIDEBAR_PNG_DIR variable moves them elsewhere.
    private static var outDir: URL {
        let dir = ProcessInfo.processInfo.environment["SIDEBAR_PNG_DIR"] ?? FileManager.default.currentDirectoryPath + "/out/sidebar"
        return URL(fileURLWithPath: dir)
    }

    static let sample = WorkspaceState(
        terminals: [
            TerminalSummary(id: 1, title: "wormhole", subtitle: "~/Projects/wormhole", branch: "main", activity: .working, isPinned: true),
            TerminalSummary(id: 2, title: "api server", subtitle: "cargo test", activity: .needsInput, unread: 3),
            TerminalSummary(id: 3, title: "docs", subtitle: "~/Projects/docs", branch: "writing", activity: .finished),
            TerminalSummary(id: 4, title: "deploy", subtitle: "exit 1", activity: .failed, unread: 1200),
            TerminalSummary(id: 5, title: "scratch", activity: .idle),
        ],
        selected: 2)

    /// Rendered through a hosting view in an off-screen window, not `ImageRenderer`: that one draws a text
    /// field as a "prohibited" placeholder and a scroll view as nothing.
    private func render(_ view: some View, scheme: ColorScheme, name: String) throws -> NSBitmapImageRep {
        let size = NSSize(width: 260, height: 420)
        let host = NSHostingView(rootView: view.frame(width: size.width, height: size.height))
        let window = NSWindow(contentRect: NSRect(origin: .zero, size: size), styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: scheme == .dark ? .darkAqua : .aqua)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        let rep = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: host.bounds))
        host.cacheDisplay(in: host.bounds, to: rep)
        try FileManager.default.createDirectory(at: Self.outDir, withIntermediateDirectories: true)
        let png = try XCTUnwrap(rep.representation(using: .png, properties: [:]))
        try png.write(to: Self.outDir.appendingPathComponent("\(name)-\(scheme == .dark ? "dark" : "light").png"))
        return rep
    }

    func testRendersLightAndDark() throws {
        var actions: [WorkspaceAction] = []
        let view = SidebarView(state: Self.sample, send: { actions.append($0) })
        for scheme in [ColorScheme.light, .dark] {
            let rep = try render(view, scheme: scheme, name: "sidebar")
            XCTAssertEqual(rep.pixelsWide % 260, 0)
        }
        XCTAssertTrue(actions.isEmpty, "drawing sends nothing")
    }

    func testRendersGroupsFoldedAndOpen() throws {
        for scheme in [ColorScheme.light, .dark] { _ = try render(GroupsHarness(), scheme: scheme, name: "groups") }
    }

    func testListingFeedsOneRowPerTerminal() {
        let sections = SidebarListing.sections(for: Self.sample)
        XCTAssertEqual(sections.flatMap(\.rows).count, 5)
        XCTAssertEqual(Self.sample.selected, 2)
    }

    func testArrowNavigationStopsAtEndsAndSkipsFoldedSections() {
        let ids = RowNavigation.visibleIDs([
            SidebarSection(id: "a", title: "A", isCollapsed: false, rows: [row(1), row(2)], terminalCount: 2, alertCount: 0),
            SidebarSection(id: "b", title: "B", isCollapsed: true, rows: [row(3)], terminalCount: 1, alertCount: 0),
            SidebarSection(id: "c", title: nil, isCollapsed: false, rows: [row(4)], terminalCount: 1, alertCount: 0),
        ])
        XCTAssertEqual(ids, [1, 2, 4])
        XCTAssertEqual(RowNavigation.neighbor(of: 2, in: ids, step: 1), 4)
        XCTAssertEqual(RowNavigation.neighbor(of: 4, in: ids, step: 1), 4)
        XCTAssertEqual(RowNavigation.neighbor(of: 1, in: ids, step: -1), 1)
        XCTAssertEqual(RowNavigation.neighbor(of: nil, in: ids, step: 1), 1)
        XCTAssertEqual(RowNavigation.neighbor(of: nil, in: ids, step: -1), 4)
        XCTAssertNil(RowNavigation.neighbor(of: 1, in: [], step: 1))
    }

    func testCountBadgeCaps() {
        XCTAssertEqual(CountBadge.label(7), "7")
        XCTAssertEqual(CountBadge.label(999), "999")
        XCTAssertEqual(CountBadge.label(1000), "999+")
    }

    private func row(_ id: Int) -> SidebarRow { SidebarRow(terminal: TerminalSummary(id: id, title: "t\(id)"), number: nil) }
}

/// Titled sections, one folded, which `SidebarListing`'s first cut cannot produce yet. A view of its own because
/// the selection namespace may only be read inside a body.
private struct GroupsHarness: View {
    @Namespace private var selection

    var body: some View {
        let rows = SidebarTests.sample.terminals.enumerated().map { SidebarRow(terminal: $1, number: $0 + 1) }
        VStack(spacing: 0) {
            section("g1", "Backend", collapsed: false, rows[0..<2])
            section("g2", "Writing", collapsed: true, rows[2..<3])
            section("g3", nil, collapsed: false, rows[3..<5])
        }
        .padding(Space._2).background(Palette.sidebarBackground)
    }

    private func section(_ id: String, _ title: String?, collapsed: Bool, _ rows: ArraySlice<SidebarRow>) -> some View {
        SidebarSectionView(
            section: SidebarSection(id: id, title: title, isCollapsed: collapsed, rows: Array(rows), terminalCount: rows.count, alertCount: 0),
            selected: 2, selection: selection, send: { _ in })
    }
}
