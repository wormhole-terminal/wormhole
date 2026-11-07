// The status item's Projects submenu: recent folders, each open project's actions, then Open Folder….

import Foundation

extension StatusMenu {
    static func projects(_ snapshot: StatusSnapshot, _ keymap: Keymap) -> [MenuNode] {
        var nodes: [MenuNode] = []
        if !snapshot.recent.isEmpty {
            nodes.append(.header("Recent"))
            nodes += snapshot.recent.map {
                .item(MenuItem(.ambient(.openProject(path: $0.path)), $0.name, subtitle: ($0.path as NSString).abbreviatingWithTildeInPath))
            }
        }
        nodes += actions(snapshot.actions)
        nodes += [.separator, .item(MenuItem(.ambient(.openFolder), "Open Folder…", key: keymap.key(.openFolder)))]
        return nodes
    }

    /// "Actions · api-server" and its actions, per project, in first-appearance order. Two projects with the same
    /// folder name are told apart by their path.
    static func actions(_ actions: [StatusSnapshot.Action]) -> [MenuNode] {
        var roots: [String] = []
        for action in actions where !roots.contains(action.root) { roots.append(action.root) }
        return roots.flatMap { root -> [MenuNode] in
            let list = actions.filter { $0.root == root }
            let name = list[0].project
            let repeats = Set(actions.filter { $0.project == name }.map(\.root)).count > 1
            return [.header(repeats ? "Actions · \(name) — \(root)" : "Actions · \(name)")]
                + list.map { .item(MenuItem(.ambient(.runAction(root: root, id: $0.id)), $0.title)) }
        }
    }
}
