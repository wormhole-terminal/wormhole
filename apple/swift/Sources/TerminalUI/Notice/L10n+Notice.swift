import Foundation

extension L10n {
    static func noticeCopiedLines(_ n: Int) -> String {
        n == 1 ? tr("notice-copied-lines-one", "Copied 1 line") : tr("notice-copied-lines-other", "Copied \(n) lines")
    }
    static func noticeCopiedCharacters(_ n: Int) -> String {
        n == 1 ? tr("notice-copied-characters-one", "Copied 1 character") : tr("notice-copied-characters-other", "Copied \(n) characters")
    }
    static func noticeCopiedOutput(_ n: Int) -> String {
        n == 1 ? tr("notice-copied-output-one", "Copied output, 1 line") : tr("notice-copied-output-other", "Copied output, \(n) lines")
    }
    static func noticeProgramLines(_ n: Int) -> String {
        n == 1 ? tr("notice-copied-program-lines-one", "A program copied 1 line") : tr("notice-copied-program-lines-other", "A program copied \(n) lines")
    }
    static func noticeProgramCharacters(_ n: Int) -> String {
        n == 1 ? tr("notice-copied-program-characters-one", "A program copied 1 character") : tr("notice-copied-program-characters-other", "A program copied \(n) characters")
    }
    static var noticeCopiedCommand: String { tr("notice-copied-command", "Copied command") }
    static var noticeCopiedPath: String { tr("notice-copied-path", "Copied path") }
    static var noticeCopiedError: String { tr("notice-copied-error", "Copied error") }
    static func noticeTyping(_ name: String) -> String { tr("notice-driven-typing", "\(name) is typing") }
    static func noticeReading(_ name: String) -> String { tr("notice-driven-reading", "\(name) is reading") }
    static var noticeScriptTyping: String { tr("notice-driven-scripts-typing", "A script is typing") }
    static var noticeScriptReading: String { tr("notice-driven-scripts-reading", "A script is reading") }
    static var noticeFailedCopyPath: String { tr("notice-failed-copy-path", "Not copied: the path has characters that would act when pasted") }
    static var noticeHistoryKept: String { tr("notice-failed-history-kept-by-host", "Cleared here only: this session host is too old to forget it") }
    static var noticePutConcealed: String { tr("notice-failed-put-concealed", "Not put at the prompt: part of it is concealed") }
    static var noticePutSameColor: String { tr("notice-failed-put-same-color", "Not put at the prompt: part of it is in its background's color") }
    static var noticePutInvisible: String { tr("notice-failed-put-invisible", "Not put at the prompt: it has characters you can't see") }
    static var noticePutLines: String { tr("notice-failed-put-lines", "Not put at the prompt: each line would run in this shell") }
    static var noticeHostFallback: String { tr("notice-warned-host-fallback", "This terminal ends when Wormhole quits") }
    static var noticeDataFolder: String { tr("notice-warned-data-folder", "Wormhole can't open a profile's data folder, so it saves nothing there") }
    static var noticeMoveToApplications: String { tr("notice-warned-move-to-applications", "Move Wormhole to Applications to keep terminals running when it quits") }
    static func noticeUpdateReady(_ version: String) -> String { tr("notice-warned-update-ready", "Wormhole \(version) is ready — Restart") }
    static var noticeInfoHostHeading: String { tr("notice-info-host-heading", "This terminal can't outlive Wormhole") }
    static var noticeInfoHostWhat: String {
        tr("notice-info-host-what", "Wormhole normally keeps each terminal running in a small helper, so your work carries on after you quit Wormhole and is waiting when you open it again. The helper for this terminal couldn't start, so the terminal runs inside Wormhole instead and closes when Wormhole quits.")
    }
    static var noticeInfoHostOthers: String { tr("notice-info-host-others", "Other terminals that already run in a helper are not affected.") }
    static var noticeInfoHostTry: String {
        tr("notice-info-host-try", "Finish what is running here before you quit. Each new terminal tries to start its helper again, and Wormhole says this once each time it starts.")
    }
    static var noticeInfoHostReason: String { tr("notice-info-host-reason", "What went wrong, if you want to report it:") }
    static var noticeInfoLogs: String { tr("notice-info-logs", "Open Logs Folder") }
    static var noticeInfoCopy: String { tr("notice-info-copy", "Copy reason") }
    static var noticeInfoCopied: String { tr("notice-info-copied", "Copied") }
    static var noticeClose: String { tr("notice-close", "Close") }
    static var noticeInfo: String { tr("notice-info", "More about this warning") }
}
