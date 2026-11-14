// The title row's own menu (a right-click on the row, ⌃↩ on it) changes the setting, as GPUI's does (`title/parts.rs`
// `update_settings`): `title_row` in the window's own profile, so every window of that profile, Settings' list and the next
// launch follow. Where a part shown again goes is the core's rule (`Settings::show_title_part`, which Settings' list asks
// too). The window's row changes when the setting comes back to it (`HubSettings.applyTitleRow`), never before.

import PanesModel
import SettingsModel

@MainActor
enum TitleRowSetting {
    /// The settings a window writes: its profile's values and the core's rules over them (nil where there is no core).
    struct Target {
        let backend: any SettingsBackend
        let host: (any SettingsHost)?
    }

    /// The settings of a window in profile `window` while Wormhole is in `current`: the shown profile's own, or the other
    /// profile's (`other`, nil where its settings could not be opened: nothing is written then, never the shown one's).
    static func target(window: String, current: String, shown: Target, other: (String) -> Target?) -> Target? {
        window == current ? shown : other(window)
    }

    /// Shows or hides `part` (its stored name): through the core's rule where there is a core; the same place worked out here
    /// where there is none (a sample's settings), as Settings' list does.
    static func show(_ part: String, _ shown: Bool, in target: Target) {
        if target.host?.showTitlePart(part, shown: shown) == true { return }
        guard let named = TitlePart(rawValue: part) else { return }
        var row = TitleRow(stored(target.backend).compactMap(TitlePart.init(rawValue:)))
        guard row.setShown(named, shown) else { return }
        target.backend.set("title_row", to: .array(row.parts.map { .string($0.rawValue) }))
    }

    /// The row put back as it comes: the key at its default, the core's.
    static func reset(in target: Target) { target.backend.reset(["title_row"]) }

    /// The row as the profile stores it, its parts' names in order.
    static func stored(_ backend: any SettingsBackend) -> [String] {
        backend.values["title_row"]?.array?.compactMap(\.string) ?? TitleRow.default.parts.map(\.rawValue)
    }
}

extension AppController {
    /// The title row's menu of `session`'s window: what it ticks under Zen mode, and what choosing a part or Reset writes.
    func connectTitleRow(of session: WindowSession) {
        let hub = session.hub
        hub.actions.showTitlePart = { [weak self, weak session] part, shown in
            guard let target = session.flatMap({ self?.titleRowTarget(of: $0) }) else { return }
            TitleRowSetting.show(part, shown, in: target)
        }
        hub.actions.resetTitleRow = { [weak self, weak session] in
            guard let target = session.flatMap({ self?.titleRowTarget(of: $0) }) else { return }
            TitleRowSetting.reset(in: target)
        }
        hub.actions.ownTitleRow = { [weak self, weak session] in
            session.flatMap { self?.titleRowTarget(of: $0) }.map { TitleRowSetting.stored($0.backend) }
        }
    }

    private func titleRowTarget(of session: WindowSession) -> TitleRowSetting.Target? {
        TitleRowSetting.target(
            window: session.windows.profile, current: windowsHost.profile,
            shown: TitleRowSetting.Target(backend: settingsModel.backend, host: settingsModel.host),
            other: { id in profileLook(for: id).map { TitleRowSetting.Target(backend: $0.backend, host: $0.host) } })
    }
}
