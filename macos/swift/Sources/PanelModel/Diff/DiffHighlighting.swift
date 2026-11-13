// Colours a diff's lines. A hunk is read as the two files it lies between: the old side is its context and removed
// lines, the new side its context and added lines, each run through the highlighter in order, so a block comment or a
// string that spans lines is read the way the file reads it. A context line takes the new side's colours. A diff the core
// coloured from the whole files (every real one) is left alone: this is for the sample's diffs.

public enum DiffHighlighting {
    public static func apply(to file: inout DiffFile, with highlighter: some SyntaxHighlighting) {
        guard !file.isBinary, !file.noHighlight, !file.coloredByCore else { return }
        let path = file.path
        for h in file.hunks.indices {
            let lines = file.hunks[h].lines
            let old = lines.indices.filter { lines[$0].kind == .context || lines[$0].kind == .removed }
            let new = lines.indices.filter { lines[$0].kind == .context || lines[$0].kind == .added }
            let oldSpans = highlighter.highlight(path: path, lines: old.map { lines[$0].text })
            let newSpans = highlighter.highlight(path: path, lines: new.map { lines[$0].text })
            for (at, index) in old.enumerated() where lines[index].kind == .removed { file.hunks[h].lines[index].spans = oldSpans[at] }
            for (at, index) in new.enumerated() { file.hunks[h].lines[index].spans = newSpans[at] }
        }
    }
}
