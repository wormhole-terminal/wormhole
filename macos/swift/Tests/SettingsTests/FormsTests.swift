import SettingsModel
import XCTest

/// A host whose profile and import answers the test controls: the drafts' own rules (what stays after a refusal, which
/// rows an apply names) are what these tests pin down, the rules behind them are core's and tested there.
@MainActor
private final class StubHost: SettingsHost {
    var refuse: String?
    var saved: [ProfileForm] = []
    var applied: (ids: [Int], shown: Int)?

    func options(for source: OptionSource) -> [SettingOption] { [] }
    func fontFamilies() -> [String] { [] }
    func perform(_ action: SettingsAction) {}
    func panelRecords() -> PanelRecords { .sample }
    func profileForm() -> ProfileForm? { saved.last ?? ProfileForm(id: "default", name: "Default", color: "blue") }
    func saveProfile(_ form: ProfileForm) -> String? {
        if let refuse { return refuse }
        saved.append(form)
        return nil
    }
    func importReview(_ id: String) async -> ImportReview? {
        ImportReview(items: [.init(label: "Font size", from: "15", value: "15"), .init(label: "Opacity", from: "0.9", value: "0.9", defaultOn: false)])
    }
    func applyImport(_ id: String, ticked: [Int], shown: Int) -> (text: String, failed: Bool) {
        applied = (ticked, shown)
        return ("Imported", false)
    }
}

@MainActor
final class FormsTests: XCTestCase {
    func testARefusedEditLeavesTheProfileAsItWasAndSaysWhy() {
        let host = StubHost()
        let draft = ProfileDraft(host: host)
        draft.edit { $0.color = "green" }
        XCTAssertEqual(draft.form?.color, "green")
        XCTAssertEqual(host.saved.count, 1)

        host.refuse = "A profile's name is 1 to 40 characters"
        draft.edit { $0.color = "red" }
        XCTAssertEqual(draft.form?.color, "green", "the swatch does not stay on a colour core refused")
        XCTAssertEqual(draft.problem, host.refuse)
    }

    func testAnImportNamesTheTickedRowsOfTheRowsShown() async throws {
        let host = StubHost()
        let draft = ImportDraft(host: host)
        draft.toggle("ghostty")
        try await Task.sleep(for: .milliseconds(100))
        XCTAssertEqual([draft.isTicked("ghostty", 0), draft.isTicked("ghostty", 1)], [true, false], "each row starts at its default")
        draft.tick("ghostty", 1, true)
        draft.apply("ghostty")
        XCTAssertEqual(host.applied?.ids, [0, 1])
        XCTAssertEqual(host.applied?.shown, 2)
        XCTAssertEqual(draft.outcome?.text, "Imported")
    }
}
