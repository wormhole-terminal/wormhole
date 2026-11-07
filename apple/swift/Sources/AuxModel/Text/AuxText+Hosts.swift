// hosts-window.ftl: the Session Hosts window and the questions about ending a host.

import Foundation

extension AuxText {
    public static var hostsTitle: String { tr("hosts-window-title", "Session Hosts") }
    public static var hostsLoading: String { tr("hosts-loading", "Looking for session hosts…") }
    public static func hostsEmpty(_ folder: String) -> String { tr("hosts-empty", "No session hosts in \(folder).") }
    public static var hostsOffNote: String {
        tr("hosts-off-note", "Persistent terminals are off, so new terminals start inside the app. Hosts left over from earlier are listed below.")
    }
    public static func hostsHeaderApp(version: String, protocol p: String) -> String {
        tr("hosts-header-app", "Wormhole \(version), protocol \(p)")
    }
    public static var hostsFolder: String { tr("hosts-header-folder", "Hosts folder") }
    public static var hostsFlag: String { tr("hosts-header-flag", "Persistent terminals") }
    public static var hostsOn: String { tr("hosts-header-flag-on", "On") }
    public static var hostsOff: String { tr("hosts-header-flag-off", "Off") }
    public static var hostsBinary: String { tr("hosts-header-binary", "Host program") }
    public static var hostsBinaryMissing: String { tr("hosts-header-binary-missing", "Not found beside the app") }
    public static var hostsBinaryDisposable: String {
        tr("hosts-header-binary-disposable", "None: the app runs from a disk image or a temporary copy")
    }
    public static var hostsAttached: String { tr("hosts-standing-attached", "Attached") }
    public static var hostsDetached: String { tr("hosts-standing-detached", "Detached") }
    public static var hostsOrphan: String { tr("hosts-standing-orphan", "Orphan") }
    public static var hostsStale: String { tr("hosts-standing-stale", "Stale") }
    public static var hostsOrphans: String { tr("hosts-count-orphan", "Orphans") }
    public static var hostsDetachedNote: String { tr("hosts-standing-detached-note", "Running, no terminal of this app holds it") }
    public static var hostsOrphanNote: String { tr("hosts-standing-orphan-note", "Running, and no saved terminal names it") }
    public static func hostsAttachedTo(window: Int, title: String) -> String {
        tr("hosts-standing-attached-to", "Window \(window), \u{201C}\(title)\u{201D}")
    }
    public static var hostsEnd: String { tr("hosts-action-end", "End") }
    public static var hostsForget: String { tr("hosts-action-forget", "Forget") }
    public static var hostsCopy: String { tr("hosts-action-copy", "Copy details") }
    public static var hostsCopyAll: String { tr("hosts-action-copy-all", "Copy all") }
    public static var hostsCopied: String { tr("hosts-action-copied", "Copied") }
    public static var hostsUnknown: String { tr("hosts-unknown", "Unknown") }

    public static func hostEndTitle(_ id: String) -> String { tr("hosts-end-title", "End session host \(id)?") }
    public static func hostEndDetailAttached(_ title: String) -> String {
        tr("hosts-end-detail-attached", "The program in the terminal \u{201C}\(title)\u{201D} gets a hangup and the terminal ends.")
    }
    public static var hostEndDetail: String { tr("hosts-end-detail", "The program in it gets a hangup and ends.") }
    public static var hostEndConfirm: String { tr("hosts-end-confirm", "End Host") }
    public static var hostCloseDetail: String {
        tr(
            "hosts-close-detail",
            "Its session host is still running, but this app can\u{2019}t show it. Ending the host ends the program in it; leaving it running keeps it in the Session Hosts window.")
    }
    public static var hostCloseEnd: String { tr("hosts-close-end", "End the Host") }
    public static var hostCloseLeave: String { tr("hosts-close-leave", "Leave It Running") }
}

// The facts' names, in the order a card lists them.
extension AuxText {
    public static var hostFieldHostPid: String { tr("hosts-field-host-pid", "Host process") }
    public static var hostFieldShellPid: String { tr("hosts-field-shell-pid", "Shell process") }
    public static var hostFieldProgram: String { tr("hosts-field-program", "Running now") }
    public static var hostFieldMemory: String { tr("hosts-field-memory", "Memory") }
    public static var hostFieldVersion: String { tr("hosts-field-version", "Host version") }
    public static var hostFieldProtocol: String { tr("hosts-field-protocol", "Protocol") }
    public static var hostFieldStarted: String { tr("hosts-field-started", "Started") }
    public static var hostFieldAnswer: String { tr("hosts-field-answer", "Answers") }
    public static var hostFieldLink: String { tr("hosts-field-link", "Connection") }
    public static var hostFieldFolder: String { tr("hosts-field-folder", "Folder") }
    public static var hostFieldSocket: String { tr("hosts-field-socket", "Socket") }
    public static var hostFieldMeta: String { tr("hosts-field-meta", "Meta file") }
}
