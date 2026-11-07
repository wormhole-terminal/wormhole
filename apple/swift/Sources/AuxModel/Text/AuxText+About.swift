// About, Licenses and the updater's words (`about/`, `settings-updates.ftl`).

import Foundation

extension AuxText {
    public static var aboutTitle: String { tr("about-title", "About Wormhole") }
    public static var licensesTitle: String { tr("licenses-title", "Licenses") }
    public static var licensesSearch: String { tr("licenses-search", "Search packages or licenses") }
    public static var licensesNoMatch: String { tr("licenses-no-match", "No package matches.") }
    public static var licenseNone: String { tr("licenses-none", "no license declared") }
    public static func licensesCount(shown: Int, total: Int) -> String {
        shown == total ? tr("licenses-count", "\(total) packages") : tr("licenses-count-of", "\(shown) of \(total) packages")
    }
    public static func licenseIs(_ license: String) -> String { tr("licenses-license", "License: \(license)") }
    public static var licenseNoneDeclared: String { tr("licenses-no-license", "No license declared") }
    public static var licenseNoFile: String {
        tr("licenses-no-file", "This package doesn\u{2019}t include a license file. The license above is what its Cargo.toml declares; see the repository for the full text.")
    }
    public static var licensesUnreadable: String { tr("licenses-unreadable", "The list of licenses could not be read.") }

    public static var updatesTitle: String { tr("settings-section-updates", "Updates") }
    public static var updatesIntro: String { tr("settings-section-updates.intro", "Signed releases, checked in the background.") }
    public static var updatesVersion: String { tr("settings-updates-version", "Version") }
    public static var updatesChannel: String { tr("settings-updates-update-channel", "Channel") }
    public static var channelStable: String { tr("settings-updates-stable", "Stable") }
    public static var channelPreview: String { tr("settings-updates-preview", "Preview") }
    public static var channelNote: String {
        tr("settings-updates-channel-description", "Preview also gets a build of every change once it passes the tests, and shows the feature flags")
    }
    public static var checkedJustNow: String { tr("settings-updates-checked-just-now", "Checked just now.") }
    public static var checkedMinuteAgo: String { tr("settings-updates-checked-a-minute-ago", "Checked a minute ago.") }
    public static var checkedHourAgo: String { tr("settings-updates-checked-an-hour-ago", "Checked an hour ago.") }
    public static func checked(ago: String) -> String { tr("settings-updates-checked-ago", "Checked \(ago).") }
    public static func downloadProgress(_ received: String, _ expected: String) -> String { tr("update-download-progress", "\(received) of \(expected)") }
    public static var notCheckedYet: String { tr("settings-updates-not-checked-yet", "Not checked yet.") }
    public static var checking: String { tr("settings-updates-checking", "Checking…") }
    public static var upToDate: String { tr("settings-updates-up-to-date", "Wormhole is up to date.") }
    public static var downloading: String { tr("settings-updates-downloading", "Downloading…") }
    public static var skipVersion: String { tr("settings-updates-skip-this-version", "Skip This Version") }
    public static var installUpdate: String { tr("settings-updates-install-update", "Install Update") }
    public static var checkNow: String { tr("settings-updates-check-now", "Check now") }
    public static var restartNow: String { tr("settings-updates-restart-now", "Restart now") }
    public static var restartWhenIdleButton: String { tr("settings-updates-restart-when-idle", "Restart when idle") }
    public static func available(_ version: String) -> String { tr("update-available", "Wormhole \(version) is available.") }
    public static func ready(_ version: String) -> String { tr("update-ready", "Wormhole \(version) is ready. Restart to use it.") }
    public static var securityUpdate: String { tr("update-security", "Security update") }
    public static var releaseNotes: String { tr("update-release-notes", "What\u{2019}s new") }
    public static var noNotes: String { tr("update-no-notes", "This release has no notes.") }
    public static func failed(_ why: String) -> String { tr("update-failed", "The update failed: \(why)") }
}
