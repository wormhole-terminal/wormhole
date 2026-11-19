// The Files viewer's side of the store: the file shown, read off the main actor, and read again when the tree's watch
// fires, so a file edited elsewhere shows its new text.

import PanelModel

extension FilesStore {
    public func view(_ path: String) {
        opened()
        viewing = path
        viewedStamp = nil
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

    /// `onlyIfChanged`: the watch woke the tree; a file that looks from outside as it did when it was read is not read
    /// again, and a read or a colouring on its way is left to finish.
    func readViewed(onlyIfChanged: Bool = false) {
        guard let root, let path = viewing else { return }
        if onlyIfChanged {
            Task { [backend] in
                let stamp = await backend.stamp(path, in: root)
                guard viewing == path, root == self.root else { return }
                if let stamp, stamp == viewedStamp { return }
                readViewed()
            }
            return
        }
        reading?.cancel()
        reading = Task { [backend, highlighter] in
            // Before the read, so a write racing it shows as a change next time.
            let stamp = await backend.stamp(path, in: root)
            guard !Task.isCancelled, viewing == path else { return }
            viewedStamp = stamp
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
