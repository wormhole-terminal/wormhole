// About, Licenses and the updater's words (`about/`, `settings-updates.ftl`).

import Foundation

extension AuxText {
    public static var aboutTitle: String { tr("about-title", "About Wormhole") }
    public static var licensesTitle: String { tr("licenses-title", "Licenses") }
    public static var licensesSearch: String { tr("licenses-search", "Search packages or licenses") }
    public static var licensesNoMatch: String { tr("licenses-no-match", "No package matches.") }
    public static var licenseNone: String { tr("licenses-none", "no license declared") }
    public static func licensesCount(shown: Int, total: Int) -> String {
        shown == total
            ? tr("licenses-count", ["total": .whole(total)], fallback: total == 1 ? "1 package" : "\(total) packages")
            : tr("licenses-count-of", ["shown": .whole(shown), "total": .whole(total)], fallback: "\(shown) of \(total) packages")
    }
    public static func licenseIs(_ license: String) -> String { tr("licenses-license", ["license": .text(license)], fallback: "License: \(license)") }
    public static var licenseNoneDeclared: String { tr("licenses-no-license", "No license declared") }
    public static var licenseNoFile: String {
        tr("licenses-no-file", "This package doesn\u{2019}t include a license file. The license above is what its Cargo.toml declares; see the repository for the full text.")
    }
    public static var licensesUnreadable: String { tr("licenses-unreadable", "The list of licenses could not be read.") }

    public static var updatesIntro: String { tr("settings-section-updates.intro", "Signed releases, checked in the background.") }
    public static var updatesVersion: String { tr("settings-updates-version", "Version") }
    public static var updatesChannel: String { tr("settings-updates-update-channel", "Channel") }
    public static var channelStable: String { tr("settings-updates-stable", "Stable") }
    public static var channelPreview: String { tr("aux-updates-preview", "Preview") }
    public static var channelNote: String {
        tr("aux-updates-channel-description", "Preview also gets a build of every change once it passes the tests, and shows the feature flags")
    }
    public static var checkedJustNow: String { tr("settings-updates-checked-just-now", "Checked just now.") }
    public static var checkedMinuteAgo: String { tr("settings-updates-checked-a-minute-ago", "Checked a minute ago.") }
    public static var checkedHourAgo: String { tr("settings-updates-checked-an-hour-ago", "Checked an hour ago.") }
    public static func checked(ago: String) -> String { tr("aux-updates-checked-ago", ["ago": .text(ago)], fallback: "Checked \(ago).") }
    public static func downloadProgress(_ received: String, _ expected: String) -> String { tr("update-download-progress", ["received": .text(received), "expected": .text(expected)], fallback: "\(received) of \(expected)") }
    public static var notCheckedYet: String { tr("settings-updates-not-checked-yet", "Not checked yet.") }
    public static var checking: String { tr("settings-updates-checking", "Checking…") }
    public static var upToDate: String { tr("settings-updates-wormhole-is-up-to-date", "Wormhole is up to date.") }
    public static var downloading: String { tr("settings-updates-downloading", "Downloading…") }
    public static var skipVersion: String { tr("settings-updates-skip-this-version", "Skip This Version") }
    public static var installUpdate: String { tr("settings-updates-install-update", "Install Update") }
    public static var checkNow: String { tr("settings-updates-check-now", "Check now") }
    public static var restartNow: String { tr("settings-updates-restart-now", "Restart now") }
    public static var restartWhenIdleButton: String { tr("aux-updates-restart-when-idle", "Restart when idle") }
    public static func available(_ version: String) -> String { tr("update-available", ["version": .text(version)], fallback: "Wormhole \(version) is available.") }
    public static func ready(_ version: String) -> String { tr("update-ready", ["version": .text(version)], fallback: "Wormhole \(version) is ready. Restart to use it.") }
    public static var dontRestart: String { tr("settings-updates-dont-restart", "Don\u{2019}t restart") }
    public static func installedWaits(_ version: String) -> String {
        tr("settings-updates-installed-waits", ["version": .text(version)], fallback: "Version \(version) is installed. Restarts when your terminals are idle.")
    }
    public static func securityAvailable(_ version: String) -> String {
        tr("settings-updates-security-available", ["version": .text(version)], fallback: "Security update available: Wormhole \(version).")
    }
    public static var securityUpdate: String { tr("update-security", "Security update") }
    public static var releaseNotes: String { tr("update-release-notes", "What\u{2019}s new") }
    public static var noNotes: String { tr("update-no-notes", "This release has no notes.") }
    public static func failed(_ why: String) -> String { tr("update-failed", ["why": .text(why)], fallback: "The update failed: \(why)") }

    public static func version(_ number: String) -> String { tr("about-version", ["version": .text(number)], fallback: "Version \(number)") }
    public static var debugBuild: String { tr("about-debug-build", "(debug build)") }
    public static var noUpdateFeed: String { tr("update-no-feed", "This is a development build; it doesn't update itself.") }
}
