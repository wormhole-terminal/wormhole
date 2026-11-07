// What the title row's controls do for the selected terminal (`window/title/actions.rs`): reveal and copy its
// folder, type the path at its prompt, open the Changes tab, split, and arrange the row's parts.

import AppKit
import PanesModel
import PanesUI
import TerminalUI
import WorkspaceModel

extension WindowHub {
    func titleActions(for terminal: TerminalSummary) -> TitleActions {
        let folder = terminal.folder.map { ($0 as NSString).expandingTildeInPath }
        return TitleActions(
            revealFolder: { if let folder { NSWorkspace.shared.open(URL(fileURLWithPath: folder)) } },
            copyPath: { [unowned self] in
                guard let folder else { return }
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(folder, forType: .string)
                notices.post(.copied(.path))
            },
            insertPath: { [unowned self] in
                if let folder { terminals.existing(terminal.id)?.source.paste(Self.shellQuoted(folder)) }
            },
            pickBranch: { [unowned self] in actions.pickBranch(terminal.id) },
            showChanges: { [unowned self] in send(.showPanelTab(.changes)) },
            split: { [unowned self] in split($0) },
            canSplit: { [unowned self] in canSplit($0) },
            setPart: { [unowned self] in if titleRow.setShown($0, $1) { rowChanged() } },
            resetRow: { [unowned self] in titleRow = .default; rowChanged() },
            customizeRow: { [unowned self] in actions.openSettings() },
            canInsert: folder != nil)
    }

    /// The row's parts changed: the core lists the row from them (which parts are at each end, where the chevrons go).
    public func rowChanged() { send(.setTitleRow(titleRow.parts.map(\.rawValue))) }

    /// The path as one word for a shell: in single quotes, a quote inside as `'\''`.
    static func shellQuoted(_ path: String) -> String {
        "'" + path.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }
}

extension WindowHub {
    /// The title row's verbs for the selected terminal: what Reveal in Finder and Copy Path in the menus do.
    public var selectedTitleActions: TitleActions? { state.selectedTerminal.map(titleActions(for:)) }
}
