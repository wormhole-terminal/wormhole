// agent-sheet.ftl: the control API's approval sheet (`agent_access/sheet.rs`).

import Foundation

extension AuxText {
    public static func accessTitle(_ name: String) -> String { tr("agent-sheet-title", "Allow \u{201C}\(name)\u{201D} to control Wormhole?") }
    public static var accessTitleScripts: String { tr("agent-sheet-title-scripts", "Allow scripts to control Wormhole?") }
    public static var wantsRead: String { tr("agent-sheet-wants-read", "It wants to read what terminals show.") }
    public static var wantsType: String { tr("agent-sheet-wants-type", "It wants to type into terminals.") }
    public static var wantsRun: String { tr("agent-sheet-wants-run", "It wants to run commands in terminals.") }
    public static var wantsOpen: String { tr("agent-sheet-wants-open", "It wants to open and arrange terminals and windows.") }
    public static var wantsClose: String { tr("agent-sheet-wants-close", "It wants to ask you to close a terminal.") }
    public static var accessDetail: String {
        tr("agent-sheet-detail", "Allowing lets this program read what terminals show, type into them, run commands and open windows. It can never close a terminal without asking you.")
    }
    public static var accessDetailScripts: String {
        tr("agent-sheet-detail-scripts", "Any program that is not a coding agent Wormhole knows counts as Scripts: a script, a tool, or a command you typed at your own prompt. Allowing it lets all of them in until Wormhole quits.")
    }
    public static func warningUnusual(_ name: String) -> String {
        tr("agent-sheet-warning-unusual", "This program is called \(name) but is not signed by its usual developer.")
    }
    public static var warningUnsigned: String { tr("agent-sheet-warning-unsigned", "It is not signed, so only this exact file is remembered.") }
    public static var warningAdHoc: String {
        tr("agent-sheet-warning-ad-hoc", "It has an ad hoc signature, so only this exact code is remembered: an update asks again.")
    }
    public static var warningChanged: String { tr("agent-sheet-warning-changed", "It has changed since you allowed it.") }
    public static var warningRoot: String { tr("agent-sheet-warning-root", "It runs as root.") }

    public static var labelPath: String { tr("agent-sheet-label-path", "Path") }
    public static var labelPid: String { tr("agent-sheet-label-pid", "PID") }
    public static var labelSignedBy: String { tr("agent-sheet-label-signed-by", "Signed by") }
    public static var labelStartedBy: String { tr("agent-sheet-label-started-by", "Started by") }
    public static var labelRunsIn: String { tr("agent-sheet-label-runs-in", "Runs in") }
    public static var labelRunsOn: String { tr("agent-sheet-label-runs-on", "Runs on") }
    public static var labelAskedBy: String { tr("agent-sheet-label-asked-by", "Asked by") }
    public static var labelTerminal: String { tr("agent-terminal-label-terminal", "Terminal") }
    public static var labelFolder: String { tr("agent-terminal-label-folder", "Folder") }

    public static func signedDeveloper(subject: String, team: String) -> String {
        tr("agent-sheet-signed-developer", "\(subject) (Developer ID, team \(team))")
    }
    public static func signedTeam(_ team: String) -> String { tr("agent-sheet-signed-team", "Developer ID, team \(team)") }
    public static var signedApple: String { tr("agent-sheet-signed-apple", "Apple") }
    public static var signedAdHoc: String { tr("agent-sheet-signed-ad-hoc", "Ad hoc signature") }
    public static var signedOther: String { tr("agent-sheet-signed-other", "Signed, not by a Developer ID") }
    public static var signedNone: String { tr("agent-sheet-signed-none", "Not signed") }
    public static var signedUnreadable: String { tr("agent-sheet-signed-unreadable", "Could not be read") }
    public static func inTerminal(_ title: String) -> String { tr("agent-sheet-in-terminal", "terminal \u{201C}\(title)\u{201D}") }
    public static func runsOn(_ name: String, signer: String) -> String { tr("agent-sheet-runs-on", "\(name), signed by \(signer)") }

    public static func terminalTitle(_ verb: String, name: String, terminal: String) -> String {
        "Allow \u{201C}\(name)\u{201D} to \(verb) \u{201C}\(terminal)\u{201D}?"
    }
    public static func terminalTitleScripts(_ verb: String, terminal: String) -> String {
        "Allow scripts to \(verb) \u{201C}\(terminal)\u{201D}?"
    }
    public static var terminalDetail: String {
        tr("agent-terminal-detail", "It did not open this terminal and does not run in it. \u{201C}Allow for This Session\u{201D} lets it use this terminal, and no other, while the program runs; \u{201C}Until Wormhole Quits\u{201D}, until Wormhole quits.")
    }
    public static var terminalDetailScripts: String {
        tr("agent-terminal-detail-scripts", "Scripts did not open this terminal. Allowing it lets scripts use this terminal, and no other, until Wormhole quits.")
    }

    public static var buttonDeny: String { tr("agent-sheet-button-deny", "Don\u{2019}t Allow") }
    public static var buttonSession: String { tr("agent-sheet-button-session", "Allow for This Session") }
    public static var buttonQuit: String { tr("agent-sheet-button-quit", "Until Wormhole Quits") }
    public static var buttonAlways: String { tr("agent-sheet-button-always", "Always") }
    public static var buttonDontKeep: String { tr("agent-sheet-button-dont-keep", "Don\u{2019}t Keep") }
    public static var buttonKeep: String { tr("agent-sheet-button-keep", "Keep") }

    public static func changeTitle(_ mode: String) -> String {
        tr("agent-change-title", "Agent access was changed outside Wormhole to \u{201C}\(mode)\u{201D}. Keep it?")
    }
    public static func changeDetail(previous: String) -> String {
        tr("agent-change-detail", "Something other than Wormhole\u{2019}s Settings edited the file, and the new mode lets more programs in. Until you answer, \u{201C}\(previous)\u{201D} stays in force; Don\u{2019}t Keep puts it back in the file.")
    }
}
