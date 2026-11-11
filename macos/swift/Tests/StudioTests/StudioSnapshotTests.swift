import StudioModel
import XCTest

@MainActor
final class StudioSnapshotTests: XCTestCase {
    func testTheStudioInDarkAndLightThemes() throws {
        for (theme, slug) in [("Nord", "nord"), ("GitHub Light", "github-light"), ("Wormhole Dark", "wormhole-dark")] {
            try StudioSnapshot.write(theme: theme, name: "studio-\(slug)")
        }
    }

    func testIncreaseContrastAndAWeakEdit() throws {
        try StudioSnapshot.write(theme: "Wormhole Light", contrast: true, name: "studio-wormhole-light-hc")
        try StudioSnapshot.write(theme: "Nord", token: "text.muted", name: "studio-weak-muted") { $0.set("text.muted", to: "#3b4252") }
    }

    func testSelectingATokenOutlinesWhatDrawsWithIt() throws {
        try StudioSnapshot.write(theme: "Nord", token: "sidebar.row.selected", area: .sidebar, name: "studio-sidebar-area")
        try StudioSnapshot.write(theme: "Nord", token: "metrics.radius", name: "studio-metric")
    }
}
