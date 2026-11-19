// What an error said, as the explanation behind a warning's ⓘ quotes it: one short line (core's `view::notice::reason`), and the
// data-folder warning's sentence with the profile and folder in it (words with arguments are the core's, not the Words seam's).

import Foundation
import WordsModel

#if WITH_UNIFFI
    import SettingsBridge
    import WormholeBindings

    enum NoticeReason {
        static func clean(_ error: String) -> String { noticeReason(error: error) }

        static func dataFolder(profile: String, folder: String, error: String) -> String {
            noticeDataFolderDetail(profile: profile, folder: folder, error: clean(error))
        }
    }
#else
    enum NoticeReason {
        static func clean(_ error: String) -> String { error.split(whereSeparator: \.isWhitespace).joined(separator: " ") }

        static func dataFolder(profile: String, folder: String, error: String) -> String {
            Words.text(
                "notice-warned-data-folder-detail", ["profile": .text(profile), "folder": .text(folder), "error": .text(clean(error))],
                fallback: "Wormhole can't open the data folder of the profile \(profile), \(folder): \(clean(error))")
        }
    }
#endif

extension AppController {
    /// The feature flags that are on for `session`'s window: its own profile's, whichever profile Wormhole is in.
    func flagsOn(_ session: WindowSession) -> Set<String> {
        #if WITH_UNIFFI
            if options.isReal { return profileFlags.on(profile: session.windows.profile) }
        #endif
        return Set(settingsModel.host.flags().filter(\.on).map(\.key))
    }

    /// The features that are off in `session`'s window's own profile: what hides their actions from menus, palette and sheet.
    func featuresOff(_ session: WindowSession) -> Set<String> {
        #if WITH_UNIFFI
            if options.isReal { return profileFlags.featuresOff(profile: session.windows.profile) }
        #endif
        return []
    }

    /// `profile`'s settings were read again: its windows take the feature switches it has now (the panel's tabs are the
    /// core workspace's), and the menus, which leave out what is off, are built again.
    func featuresMoved(in profile: String) {
        for session in sessions where session.windows.profile == profile {
            HubSettings.applyFeatures(off: featuresOff(session), to: session.hub)
        }
        menus?.windowsChanged()
    }

    /// The Notices feature (its flag) in the window's profile: off, the notice chip is not drawn and the pane's bar says what matters.
    func noticesOn(_ session: WindowSession) -> Bool {
        let flags = flagsOn(session)
        return flags.contains("notices") || !settingsModel.host.flags().contains { $0.key == "notices" }
    }
}
