import XCTest
import Observation
@testable import SettingsUI

final class SettingsNavigationTests: XCTestCase {
    func testSearchFindsTranslatedWordsAndRequiresEveryTerm() async {
        await MainActor.run {
            let store = HostStore()
            store.apply(["words": [wordID("Scrollback"): "Défilement"]])
            let model = SettingsModel(store: store, page: "Terminal")
            model.search = "  defilement   terminal  "
            XCTAssertEqual(model.searchHits.flatMap(\.rows).map(\.key), ["Terminal:0:0"])
            model.search += " nonexistent"
            XCTAssertTrue(model.searchHits.isEmpty)
            model.search = " \n "
            XCTAssertFalse(model.isSearching)
        }
    }

    func testSearchUsesVisibleHostRowsAndExcludesHiddenPages() async {
        await MainActor.run {
            let store = HostStore()
            store.apply([
                "hidden": ["Motion"],
                "values": ["Sessions:3:0": "off"],
                "lists": ["Agents:0": [["id": "test-agent", "kind": "agent", "label": "My test agent"]]]
            ])
            let model = SettingsModel(store: store, page: "Terminal")
            model.search = "test agent"
            XCTAssertEqual(model.searchHits.flatMap(\.rows).map(\.key), ["Agents:0:test-agent"])
            model.search = "motion"
            XCTAssertFalse(model.searchHits.contains { $0.page.id == "Motion" })
            // Count depends on the Changed files switch, which is off in this snapshot.
            model.search = "Count"
            XCTAssertFalse(model.searchHits.flatMap(\.rows).contains { $0.key == "Sessions:3:1" })
        }
    }

    func testHistoryPublishesAvailabilityAndKeepsGroupSelection() async {
        let changed = expectation(description: "History availability updates the toolbar")
        await MainActor.run {
            let model = SettingsModel(store: HostStore(), page: "Terminal")
            withObservationTracking { _ = model.canGoBack } onChange: { changed.fulfill() }
            model.go("Appearance/Accent")
            XCTAssertEqual(model.sel, Sel(page: "Appearance", group: 1))
            model.go("Fonts/Code")
            model.search = "font"
            model.goBack()
            XCTAssertEqual(model.sel, Sel(page: "Appearance", group: 1))
            XCTAssertTrue(model.search.isEmpty)
            XCTAssertTrue(model.canGoForward)
            model.goForward()
            XCTAssertEqual(model.sel, Sel(page: "Fonts", group: 1))
            model.goBack()
            model.go("Notifications")
            XCTAssertFalse(model.canGoForward)
        }
        await fulfillment(of: [changed], timeout: 1)
    }
}
