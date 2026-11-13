// What the approval sheet says and offers for a request (`agent_access/sheet.rs` `display`, `terminal_display`,
// `change_display`): the title, the detail with its warnings, the monospace block of the program's facts, and the
// buttons, first the one that backs out.

import Foundation

public enum AccessButton: Sendable {
    case deny, session, quit, always, keep, dontKeep

    public var label: String {
        switch self {
        case .deny: AuxText.buttonDeny
        case .session: AuxText.buttonSession
        case .quit: AuxText.buttonQuit
        case .always: AuxText.buttonAlways
        case .keep: AuxText.buttonKeep
        case .dontKeep: AuxText.buttonDontKeep
        }
    }
}

public struct AccessDisplay: Equatable, Sendable {
    public var title: String
    public var detail: String
    /// Monospace, label and value in two columns; empty for none.
    public var accessory: String
    public var buttons: [AccessButton]

    public static func of(_ request: AccessRequest) -> AccessDisplay {
        if let terminal = request.terminal { return terminalDisplay(request, terminal) }
        let scripts = request.kind == .scripts
        let wants = switch request.capability {
        case .read: AuxText.wantsRead
        case .type: AuxText.wantsType
        case .run: AuxText.wantsRun
        case .open: AuxText.wantsOpen
        case .close: AuxText.wantsClose
        }
        var detail = wants + "\n\n" + (scripts ? AuxText.accessDetailScripts : AuxText.accessDetail)
        for warning in warnings(request) { detail += "\n\n" + warning }
        return AccessDisplay(
            title: scripts ? AuxText.accessTitleScripts : AuxText.accessTitle(request.label), detail: detail,
            accessory: facts(request), buttons: buttons(request))
    }

    /// The question about a mode edited outside the app: keep it or put the old one back.
    public static func modeChanged(previous: String, requested: String) -> AccessDisplay {
        AccessDisplay(
            title: AuxText.changeTitle(requested), detail: AuxText.changeDetail(previous: previous), accessory: "",
            buttons: [.dontKeep, .keep])
    }

    static func buttons(_ request: AccessRequest) -> [AccessButton] {
        var buttons: [AccessButton] = [.deny]
        // Scripts has no process to tie it to (`resolved.process.is_some()`).
        if request.program != nil, request.kind != .scripts { buttons.append(.session) }
        buttons.append(.quit)
        // A terminal is not a stable thing to approve for good: only the question about the program has Always.
        if request.canBeRemembered, request.terminal == nil { buttons.append(.always) }
        return buttons
    }

    private static func terminalDisplay(_ request: AccessRequest, _ terminal: AccessRequest.Terminal) -> AccessDisplay {
        let scripts = request.kind == .scripts
        var rows = [(AuxText.labelTerminal, terminal.name)]
        if let folder = terminal.folder { rows.append((AuxText.labelFolder, folder)) }
        return AccessDisplay(
            title: scripts
                ? AuxText.terminalTitleScripts(request.capability, terminal: terminal.name)
                : AuxText.terminalTitle(request.capability, name: request.label, terminal: terminal.name),
            detail: scripts ? AuxText.terminalDetailScripts : AuxText.terminalDetail, accessory: columns(rows),
            buttons: buttons(request))
    }

    private static func warnings(_ request: AccessRequest) -> [String] {
        var lines: [String] = []
        if request.unusualSigner {
            let name = request.program.map { ($0.path as NSString).lastPathComponent } ?? request.label
            lines.append(AuxText.warningUnusual(name))
        }
        switch request.kind {
        case .unsigned: lines.append(AuxText.warningUnsigned)
        case .adHoc: lines.append(AuxText.warningAdHoc)
        default: break
        }
        if request.changedSinceAllowed { lines.append(AuxText.warningChanged) }
        if request.elevated { lines.append(AuxText.warningRoot) }
        return lines
    }

    private static func facts(_ request: AccessRequest) -> String {
        guard let program = request.program else { return "" }
        var rows = [
            (request.kind == .scripts ? AuxText.labelAskedBy : AuxText.labelPath, program.path),
            (AuxText.labelPid, String(program.pid)),
        ]
        let signer = signerText(program.signer)
        if let interpreter = program.interpreter {
            rows.append((AuxText.labelRunsOn, AuxText.runsOn(interpreter, signer: signer)))
        } else {
            rows.append((AuxText.labelSignedBy, signer))
        }
        if let startedBy = program.startedBy { rows.append((AuxText.labelStartedBy, startedBy)) }
        if let runsIn = program.runsIn { rows.append((AuxText.labelRunsIn, AuxText.inTerminal(runsIn))) }
        return columns(rows)
    }

    static func signerText(_ signer: AccessRequest.Signer) -> String {
        switch signer.kind {
        case .developerID:
            switch (signer.subject, signer.team) {
            case (let subject?, let team?): AuxText.signedDeveloper(subject: subject, team: team)
            case (nil, let team?): AuxText.signedTeam(team)
            default: AuxText.signedOther
            }
        case .appleSystem: AuxText.signedApple
        case .adHoc: AuxText.signedAdHoc
        case .other: AuxText.signedOther
        case .unsigned: AuxText.signedNone
        case .unreadable: AuxText.signedUnreadable
        }
    }

    /// "Path  /usr/bin/x": the labels padded to the longest, so a monospace block lines up.
    private static func columns(_ rows: [(String, String)]) -> String {
        let width = rows.map { $0.0.count }.max() ?? 0
        return rows.map { $0.0.padding(toLength: width, withPad: " ", startingAt: 0) + "  " + $0.1 }.joined(separator: "\n")
    }
}
