// What a profile's settings say of each of its windows, which the window's hub holds itself: the title row's order, the
// app "Open in" starts, what the core's workspace counts by, and the right panel's switches. Applied to the windows of
// the profile the values are of, never to another profile's.

import PanesModel
import PanesUI
import SettingsModel
import WindowUI
import WorkspaceModel

@MainActor
enum HubSettings {
    /// Everything at once, for a window made after the settings were applied (or moved into the profile).
    static func dress(_ hub: WindowHub, _ values: [String: JSONValue], zen: WindowShown?) {
        applyTitleRow(values, zen: zen, to: [hub])
        if values["open_in"] != nil { applyOpenIn(values, to: [hub]) }
        applySidebarRules(values, to: [hub])
        PanelSettings.apply(values, to: [hub])
    }

    /// Which features the window's profile has on (`FeatureMask`): the core's workspace offers only their panel tabs.
    static func applyFeatures(off: Set<String>, to hub: WindowHub) {
        for feature in FeatureKey.allCases { hub.send(.setFeature(feature, !off.contains(feature.rawValue))) }
    }

    /// What the core's workspace counts by: ⌘1–9 staying with their terminal or following the list as shown
    /// (`sidebar_numbers`), and which work a row's ±N is made of (`sidebar_changes_plain`, `sidebar_changes_shared`).
    static func applySidebarRules(_ values: [String: JSONValue], to hubs: [WindowHub]) {
        let follow = values["sidebar_numbers"]?.string == "list"
        let rules = WorkspaceAction.setChangeRules(
            plain: values["sidebar_changes_plain"]?.string ?? "finishing", shared: values["sidebar_changes_shared"]?.string ?? "each")
        for hub in hubs {
            hub.send(.setNumbersFollowList(follow))
            hub.send(rules)
        }
    }

    static func applyOpenIn(_ values: [String: JSONValue], to hubs: [WindowHub]) {
        let chosen = values["open_in"]?.string ?? ""
        hubs.forEach { $0.openIn = SystemOpenIn(chosen: { chosen }) }
    }

    /// The row is the Zen preset's while Zen mode is on, else the person's own.
    static func applyTitleRow(_ values: [String: JSONValue], zen: WindowShown?, to hubs: [WindowHub]) {
        guard let keys = zen?.titleRow ?? values["title_row"]?.array?.compactMap(\.string) else { return }
        let row = TitleRow(keys.compactMap(TitlePart.init(rawValue:)))
        for hub in hubs {
            hub.zenActive = zen != nil
            guard hub.titleRow != row else { continue }
            hub.titleRow = row
            hub.rowChanged()
        }
    }
}
