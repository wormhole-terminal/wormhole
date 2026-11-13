// The question a terminal opening in a group with no folder asks first (`group_folder.rs` `new_session_in_group`): use the
// folder the core offered, or not now (never asked again for this group); closing the sheet is "Not Now", as in GPUI. The
// box keeps only the terminals in the folder in the group, and says how many would leave. The core decides whether to offer;
// a system alert asks. Every way a terminal is made in a group goes through `settle`.

import AppKit
import WordsModel
import WorkspaceModel

public enum GroupFolderQuestion {
    public enum Answer: Sendable { case use(onlyInFolder: Bool), decline }

    /// What is asked: the folder offered, the group, and how many terminals are in other folders.
    public struct Offer: Sendable { public var folder: String, group: String, leaving: Int }

    /// What asks instead of the system alert: a test instance holds the question for its control folder (`group-ask`).
    @MainActor public static var asker: (@MainActor (Offer) async -> Answer)?

    /// Asks about the folder the core offers for `group`, if it offers one, and sends the answer (`setGroupFolder` and
    /// `setOnlyInFolder`, or `declineGroupFolder`). The caller then opens the terminal, whatever was answered.
    @MainActor public static func settle(
        group: GroupID, name: String, planner: WorkspaceStore, send: @MainActor (WorkspaceAction) -> Void
    ) async {
        guard let folder = planner.groupFolderOffer(group: group) else { return }
        let leaving = planner.groupFolderLeaving(group: group, folder: folder)
        switch await ask(folder: folder, group: name, leaving: leaving) {
        case .use(let only):
            send(.setGroupFolder(group, folder))
            if only { send(.setOnlyInFolder(group, true)) }
        case .decline: send(.declineGroupFolder(group))
        }
    }

    /// The words of the question, for the alert and for whatever holds it instead.
    public static func words(_ offer: Offer) -> (title: String, detail: String, use: String, decline: String, only: String) {
        let last = (offer.folder as NSString).lastPathComponent
        return (
            Words.text(
                "sidebar-folder-ask-title", ["folder": .text(last), "group": .text(offer.group)],
                fallback: "Use \u{201C}\(last)\u{201D} for \u{201C}\(offer.group)\u{201D}?"),
            offer.leaving == 0
                ? Words.text("sidebar-folder-ask-detail", ["path": .text(offer.folder)], fallback: "\(offer.folder)\n\nNew terminals in the group open there.")
                : Words.text(
                    "sidebar-folder-ask-detail-leaving", ["path": .text(offer.folder), "count": .whole(offer.leaving)],
                    fallback: "\(offer.folder)\n\nNew terminals in the group open there. \(offer.leaving) terminals are in other folders and would leave the group if the box is ticked."),
            Words.text("sidebar-folder-ask-use", fallback: "Use Folder"), Words.text("sidebar-folder-ask-decline", fallback: "Not Now"),
            Words.text("sidebar-folder-ask-only", fallback: "Only keep terminals in this folder"))
    }

    /// `folder` is the offered folder's whole path as the list shows it (`~/Projects/app`), `group` the group's name,
    /// `leaving` how many terminals are in other folders.
    @MainActor static func ask(folder: String, group: String, leaving: Int) async -> Answer {
        let offer = Offer(folder: folder, group: group, leaving: leaving)
        if let asker { return await asker(offer) }
        let words = words(offer)
        let alert = NSAlert()
        alert.alertStyle = .informational
        alert.messageText = words.title
        alert.informativeText = words.detail
        alert.addButton(withTitle: words.use)
        let decline = alert.addButton(withTitle: words.decline)
        decline.keyEquivalent = "\u{1b}"
        let only = NSButton(checkboxWithTitle: words.only, target: nil, action: nil)
        alert.accessoryView = only
        let answer = alert.runModal()
        return answer == .alertFirstButtonReturn ? .use(onlyInFolder: only.state == .on) : .decline
    }
}
