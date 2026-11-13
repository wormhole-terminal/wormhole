// The question a terminal opening in a group with no folder asks first (`group_folder.rs` `new_session_in_group`): use the
// folder the core offered, or not now (never asked again for this group); closing the sheet is "Not Now", as in GPUI. The
// box keeps only the terminals in the folder in the group, and says how many would leave. The core decides whether to offer;
// a system alert asks. Every way a terminal is made in a group goes through `settle`.

import AppKit
import WordsModel
import WorkspaceModel

public enum GroupFolderQuestion {
    enum Answer { case use(onlyInFolder: Bool), decline }

    /// Asks about the folder the core offers for `group`, if it offers one, and sends the answer (`setGroupFolder` and
    /// `setOnlyInFolder`, or `declineGroupFolder`). The caller then opens the terminal, whatever was answered.
    @MainActor public static func settle(
        group: GroupID, name: String, planner: WorkspaceStore, send: @MainActor (WorkspaceAction) -> Void
    ) {
        guard let folder = planner.groupFolderOffer(group: group) else { return }
        let leaving = planner.groupFolderLeaving(group: group, folder: folder)
        switch ask(folder: folder, group: name, leaving: leaving) {
        case .use(let only):
            send(.setGroupFolder(group, folder))
            if only { send(.setOnlyInFolder(group, true)) }
        case .decline: send(.declineGroupFolder(group))
        }
    }

    /// `folder` is the offered folder's whole path as the list shows it (`~/Projects/app`), `group` the group's name,
    /// `leaving` how many terminals are in other folders.
    @MainActor static func ask(folder: String, group: String, leaving: Int) -> Answer {
        let alert = NSAlert()
        alert.alertStyle = .informational
        let last = (folder as NSString).lastPathComponent
        alert.messageText = Words.text(
            "sidebar-folder-ask-title", ["folder": .text(last), "group": .text(group)],
            fallback: "Use \u{201C}\(last)\u{201D} for \u{201C}\(group)\u{201D}?")
        alert.informativeText =
            leaving == 0
            ? Words.text("sidebar-folder-ask-detail", ["path": .text(folder)], fallback: "\(folder)\n\nNew terminals in the group open there.")
            : Words.text(
                "sidebar-folder-ask-detail-leaving", ["path": .text(folder), "count": .whole(leaving)],
                fallback: "\(folder)\n\nNew terminals in the group open there. \(leaving) terminals are in other folders and would leave the group if the box is ticked.")
        alert.addButton(withTitle: Words.text("sidebar-folder-ask-use", fallback: "Use Folder"))
        let decline = alert.addButton(withTitle: Words.text("sidebar-folder-ask-decline", fallback: "Not Now"))
        decline.keyEquivalent = "\u{1b}"
        let only = NSButton(
            checkboxWithTitle: Words.text("sidebar-folder-ask-only", fallback: "Only keep terminals in this folder"), target: nil, action: nil)
        alert.accessoryView = only
        let answer = alert.runModal()
        return answer == .alertFirstButtonReturn ? .use(onlyInFolder: only.state == .on) : .decline
    }
}
