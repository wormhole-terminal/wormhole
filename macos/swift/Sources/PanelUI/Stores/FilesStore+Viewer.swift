// The Files viewer's side of the store: the file shown, read off the main actor, and read again when the tree's watch
// fires, so a file edited elsewhere shows its new text.

import PanelModel

extension FilesStore {
    public func view(_ path: String) {
        opened()
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
        picture = nil
        // The file was what kept a hidden tab reading.
        if !isShown { stopReading() }
    }

    func readViewed() {
        guard let root, let path = viewing else { return }
        reading?.cancel()
        reading = Task { [backend, highlighter] in
            let read = await backend.read(path, in: root)
            guard !Task.isCancelled, viewing == path, content != read else { return }
            // The text shows at once; its colours follow in a second job, which a newer read supersedes.
            var shown: [String] = []
            if case let .text(text, _, _) = read { shown = ViewedLines.cut(text) }
            lines = shown
            spans = []
            picture = nil
            content = read
            if case let .image(data, _) = read {
                let decoded = await Task.detached { FilePicture.decode(data) }.value
                if !Task.isCancelled, viewing == path { picture = decoded }
                return
            }
            guard !shown.isEmpty else { return }
            let coloured = await Task.detached { highlighter.highlight(path: path, lines: shown) }.value
            if !Task.isCancelled, viewing == path { spans = coloured }
        }
    }
}
