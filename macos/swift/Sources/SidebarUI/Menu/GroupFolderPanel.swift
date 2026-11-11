// The system's open panel for a group's folder: the group is pinned to what is chosen (`choose_group_folder`). The group's
// menu, its popover and the keyboard's group menu all ask it.

import AppKit
import WordsModel
import WorkspaceModel

public enum GroupFolderPanel {
    @MainActor public static func choose(for group: GroupID, send: @MainActor (WorkspaceAction) -> Void) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.prompt = Words.text("files-choose-prompt", fallback: "Choose")
        if panel.runModal() == .OK, let url = panel.url { send(.setGroupFolder(group, url.path)) }
    }
}
