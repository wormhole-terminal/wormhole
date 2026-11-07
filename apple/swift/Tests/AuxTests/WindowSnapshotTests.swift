import AppKit
import AuxModel
import DesignSystem
import SwiftUI
import XCTest
@testable import AuxWindows

/// Each secondary window drawn on sample data in three themes (two dark, one light) and with Increase Contrast once.
@MainActor
final class WindowSnapshotTests: XCTestCase {
    static let themes = ["wormhole.dark", "tokyo_night", "wormhole.light"]

    static var licensesURL: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appending(path: "crates/wormhole-core/assets/licenses.json")
    }

    private func each(_ name: String, size: CGSize, contrast: Bool = true, make: () throws -> some View) throws {
        for theme in Self.themes { try Snapshot.write(make(), name: name, theme: theme, size: size) }
        if contrast { try Snapshot.write(make(), name: name, theme: "wormhole.dark", contrast: true, size: size) }
    }

    func testLicenses() throws {
        let list = try FileLicenseSource(url: Self.licensesURL).load()
        XCTAssertGreaterThan(list.packages.count, 100)
        try each("licenses", size: LicensesMetrics.size) { LicensesView(browser: LicenseBrowser(list: list)) }
    }

    func testUpdate() throws {
        let size = CGSize(width: 520, height: 560)
        let phases: [(String, UpdatePhase)] = [
            ("available", .available(SampleUpdateBackend.release)), ("downloading", .downloading(received: 8_000_000, expected: 20_000_000)),
            ("installed", .installed(version: "1.7.0")), ("uptodate", .upToDate), ("failed", .failed("the signature did not verify")),
        ]
        for (name, phase) in phases {
            try each("update-\(name)", size: size, contrast: name == "available") {
                UpdateView(model: UpdateViewModel(backend: SampleUpdateBackend(phase: phase)), version: "1.6.3", sheetWindow: { nil })
            }
        }
    }

    func testHosts() throws {
        let backend = SampleHostsBackend()
        let model = HostsViewModel(backend: backend) { _ in }
        let expectation = expectation(description: "report")
        Task { await model.refresh(); expectation.fulfill() }
        wait(for: [expectation], timeout: 2)
        try each("hosts", size: CGSize(width: 940, height: 720)) { HostsView(model: model, sheetWindow: { nil }) }
    }

    func testTrust() throws {
        try each("trust", size: CGSize(width: 640, height: 480)) { TrustView(model: TrustViewModel(backend: SampleTrustBackend())) }
    }
}
