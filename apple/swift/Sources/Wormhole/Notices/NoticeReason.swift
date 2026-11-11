// What an error said, as the explanation behind a warning's ⓘ quotes it: one short line (core's `view::notice::reason`), and the
// data-folder warning's sentence with the profile and folder in it (words with arguments are the core's, not the Words seam's).

import Foundation

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
            "Wormhole can't open the data folder of the profile \(profile), \(folder): \(clean(error))"
        }
    }
#endif

extension AppController {
    /// The Notices feature (its flag), in the profile shown: off, the notice chip is not drawn and the pane's bar says what matters.
    var noticesOn: Bool {
        #if WITH_UNIFFI
            for flag in (settingsModel.host as? RustSettingsHost)?.flags() ?? [] where flag.key == "notices" { return flag.on }
        #endif
        return true
    }
}
