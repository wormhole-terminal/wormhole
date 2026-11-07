// The question the + on a group with no folder asks before a terminal opens in it (`sidebar-folder-ask-*`): use the folder
// the core offered, not now (never asked again for this group), or cancel. The core decides whether to offer; a system
// alert asks, as in the GPUI app. The words are the catalog's English, here because the text table is not linked yet.

import AppKit

enum GroupFolderQuestion {
    enum Answer { case use, decline, cancel }

    /// `folder` is the offered folder's whole path as the list shows it (`~/Projects/app`), `group` the group's name.
    @MainActor static func ask(folder: String, group: String) -> Answer {
        let alert = NSAlert()
        alert.messageText = "Use \u{201C}\((folder as NSString).lastPathComponent)\u{201D} for \u{201C}\(group)\u{201D}?"
        alert.informativeText = "\(folder)\nNew terminals in the group open there."
        alert.addButton(withTitle: "Use Folder")
        alert.addButton(withTitle: "Not Now")
        alert.addButton(withTitle: "Cancel")
        switch alert.runModal() {
        case .alertFirstButtonReturn: return .use
        case .alertSecondButtonReturn: return .decline
        default: return .cancel
        }
    }
}
