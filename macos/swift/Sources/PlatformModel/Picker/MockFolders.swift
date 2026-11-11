// A folder provider over a fixed list, for previews, tests and the offscreen renders until core's walk comes over
// the FFI. It matches names by containment and answers recent folders in their own section when nothing is typed.

import Foundation

public struct MockFolders: FolderProviding, FolderDecorating {
    public var folders: [String]
    public var recent: [String]

    public init(folders: [String], recent: [String] = []) { (self.folders, self.recent) = (folders, recent) }

    public static let sample = MockFolders(
        folders: ["Projects", "Documents", "Downloads", "Desktop", "Library", "Music", "Pictures", "src", "tools"],
        recent: ["~/Projects/terminal/wormhole", "~/Projects/terminal/web", "~/Downloads"])

    public func answer(_ query: PickerQuery) async -> PickerAnswer {
        var answer = PickerAnswer()
        let typed = query.text
        var rows = folders.filter { typed.isEmpty || $0.localizedCaseInsensitiveContains(typed) }.map { name -> PickerRow in
            var row = PickerRow(key: query.context.cwd + "/" + name, label: name)
            row.descend = name + "/"
            row.insert = name + "/"
            if let at = name.range(of: typed, options: .caseInsensitive), !typed.isEmpty {
                row.marks = [name.distance(from: name.startIndex, to: at.lowerBound)..<name.distance(from: name.startIndex, to: at.upperBound)]
            }
            return row
        }
        if typed.isEmpty {
            answer.sections = ["", "Recent"]
            rows += recent.map { path in
                var row = PickerRow(key: path, label: (path as NSString).lastPathComponent)
                row.icon = "clock.arrow.circlepath"
                row.detail = (path as NSString).deletingLastPathComponent
                row.insert = path
                row.section = 1
                return row
            }
        }
        answer.rows = rows
        answer.crumbs = [PickerCrumb(label: (query.context.cwd as NSString).lastPathComponent, text: "")]
        answer.up = typed.isEmpty ? nil : ""
        return answer
    }

    public func decorate(_ keys: [String]) async -> [PickerDecoration] {
        keys.map { PickerDecoration(key: $0, badge: $0.hasSuffix("Projects") ? "main" : nil, chevron: true) }
    }
}
