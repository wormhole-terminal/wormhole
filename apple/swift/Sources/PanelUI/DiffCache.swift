// Parses a file's diff once while it is shown: hovering and folding redraw the pane many times, and the text of a
// large diff must not be parsed again for each.

import PanelModel

@MainActor
final class DiffCache {
    private var parsed: [ChangedFile.ID: DiffFile?] = [:]

    func file(for id: ChangedFile.ID, text: String?) -> DiffFile? {
        if let cached = parsed[id] { return cached }
        let file = text.flatMap { UnifiedDiffParser.parse($0).first }
        parsed[id] = file
        return file
    }
}
