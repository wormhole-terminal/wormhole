import Foundation
import SettingsModel
import Testing

@testable import Wormhole

/// The title row's menu writes `title_row` in the settings of the window's own profile (`title/parts.rs` `update_settings`):
/// never another profile's, and nothing when that profile's settings cannot be opened.
@MainActor
struct TitleRowSettingTests {
    /// A host with the core behind it, as `RustSettingsHost` is: `show_title_part` writes through the handle it shares with its
    /// own profile's backend (here: that backend), and says whether the list changed. Only hiding is needed here.
    @MainActor final class CoreHost: SettingsHost {
        let backend: MockSettingsBackend
        private(set) var asked: [String] = []

        init(_ backend: MockSettingsBackend) { self.backend = backend }

        func options(for source: OptionSource) -> [SettingOption] { [] }
        func perform(_ action: SettingsAction) {}
        func panelRecords() -> PanelRecords { .sample }

        func showTitlePart(_ part: String, shown: Bool) -> Bool {
            asked.append(part)
            let row = TitleRowSetting.stored(backend)
            guard !shown, row.contains(part) else { return false }
            backend.set("title_row", to: .array(row.filter { $0 != part }.map(JSONValue.string)))
            return true
        }
    }

    @Test func aPartChosenInTheMenuIsWrittenForTheWindowsOwnProfile() {
        let shown = MockSettingsBackend(), work = MockSettingsBackend()
        let shownHost = CoreHost(shown), workHost = CoreHost(work)
        let shownTarget = TitleRowSetting.Target(backend: shown, host: shownHost)
        let workTarget = TitleRowSetting.Target(backend: work, host: workHost)
        let other: (String) -> TitleRowSetting.Target? = { $0 == "work" ? workTarget : nil }

        // A window of the profile Wormhole is in writes that profile's row.
        let mine = TitleRowSetting.target(window: "default", current: "default", shown: shownTarget, other: other)
        if let mine { TitleRowSetting.show("ports", false, in: mine) }
        #expect(!TitleRowSetting.stored(shown).contains("ports"))
        #expect(TitleRowSetting.stored(work).contains("ports"), "the other profile's row is left as it was")

        // A window of another profile writes its own, never the shown one's.
        let theirs = TitleRowSetting.target(window: "work", current: "default", shown: shownTarget, other: other)
        if let theirs { TitleRowSetting.show("branch", false, in: theirs) }
        #expect(!TitleRowSetting.stored(work).contains("branch"))
        #expect(TitleRowSetting.stored(shown).contains("branch"))
        #expect(shownHost.asked == ["ports"] && workHost.asked == ["branch"], "each part went through its own profile's core")

        // A profile whose settings cannot be opened is written nowhere.
        #expect(TitleRowSetting.target(window: "gone", current: "default", shown: shownTarget, other: other) == nil)

        // Reset puts the row back, in that profile only.
        TitleRowSetting.reset(in: workTarget)
        #expect(TitleRowSetting.stored(work).contains("branch"))
        #expect(!TitleRowSetting.stored(shown).contains("ports"))
    }

    @Test func withoutTheCoreThePartsPlaceIsWorkedOutAndWritten() {
        let backend = MockSettingsBackend()
        let target = TitleRowSetting.Target(backend: backend, host: MockSettingsHost())
        TitleRowSetting.show("ports", false, in: target)
        #expect(!TitleRowSetting.stored(backend).contains("ports"))
        // Shown again, it goes back where it is by default: after the agent's gauge.
        TitleRowSetting.show("ports", true, in: target)
        let row = TitleRowSetting.stored(backend)
        #expect(row.firstIndex(of: "ports") == row.firstIndex(of: "agent_gauge").map { $0 + 1 })
        #expect(backend.writes == ["title_row", "title_row"])
    }
}
