// The Files viewer's side of the store: the file shown, read off the main actor, and read again when the tree's watch
// fires, so a file edited elsewhere shows its new text.

import PanelModel

extension FilesStore {
    public func view(_ path: String) {
        viewing = path
        content = .reading
        readViewed()
    }

    public func closeViewer() {
        reading?.cancel()
        viewing = nil
        content = .reading
        lines = []
        spans = []
    }

    func readViewed() {
        guard let root, let path = viewing else { return }
        reading?.cancel()
        reading = Task { [backend, highlighter] in
            let read = await backend.read(path, in: root)
            guard !Task.isCancelled, viewing == path, content != read else { return }
            // The text shows at once; its colours follow in a second job, which a newer read supersedes.
            var shown: [String] = []
            if case let .text(text, _, _) = read { shown = text.split(separator: "\n", omittingEmptySubsequences: false).map(String.init) }
            lines = shown
            spans = []
            content = read
            guard !shown.isEmpty else { return }
            let coloured = await Task.detached { highlighter.highlight(path: path, lines: shown) }.value
            if !Task.isCancelled, viewing == path { spans = coloured }
        }
    }
}
