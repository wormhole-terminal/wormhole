// The group head's one multi-step action: a terminal in the group. The core says whether the group has a folder worth asking
// about; the person's answer is two more core actions (`setGroupFolder`, `declineGroupFolder`), then the terminal opens.

import SwiftUI
import WorkspaceModel

extension SidebarView {
    func newInGroup(_ group: GroupID) {
        if let folder = dropPlanner?.groupFolderOffer(group: group) {
            let name = state.groups.first { $0.id == group }?.name ?? ""
            switch GroupFolderQuestion.ask(folder: folder, group: name) {
            case .use: send(.setGroupFolder(group, folder))
            case .decline: send(.declineGroupFolder(group))
            case .cancel: return
            }
        }
        send(.newTerminalInGroup(group))
    }
}
