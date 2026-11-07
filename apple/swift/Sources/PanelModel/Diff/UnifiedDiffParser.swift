// Parses `git diff` output. Hunk headers say how many lines each side has, so the parser knows where a hunk
// ends instead of guessing from the first character: a removed line that reads `-- a` and a file header
// both start with dashes.

import Foundation

public enum UnifiedDiffParser {
    public static func parse(_ text: String) -> [DiffFile] {
        var files: [DiffFile] = []
        var hunk: (value: DiffHunk, oldLeft: Int, newLeft: Int, old: Int, new: Int)?

        func closeHunk() {
            guard let open = hunk, !files.isEmpty else { return }
            files[files.count - 1].hunks.append(open.value)
            hunk = nil
        }

        for line in text.split(separator: "\n", omittingEmptySubsequences: false) {
            if var open = hunk {
                if line.hasPrefix("\\") {
                    open.value.lines.append(DiffLine(kind: .noNewline, text: String(line.dropFirst(2))))
                    hunk = open
                    continue
                }
                if open.oldLeft > 0 || open.newLeft > 0 {
                    hunk = body(line, into: open)
                    continue
                }
                closeHunk()
            }
            if line.hasPrefix("diff --git ") {
                files.append(header(String(line.dropFirst(11))))
            } else if line.hasPrefix("@@"), !files.isEmpty, let opened = openHunk(line) {
                hunk = opened
            } else if !files.isEmpty {
                metadata(String(line), of: &files[files.count - 1])
            }
        }
        closeHunk()
        return files
    }

    private typealias OpenHunk = (value: DiffHunk, oldLeft: Int, newLeft: Int, old: Int, new: Int)

    private static func body(_ line: Substring, into open: OpenHunk) -> OpenHunk {
        var open = open
        // A context line whose trailing space an editor stripped arrives empty.
        let marker = line.first ?? " "
        let text = String(line.dropFirst())
        switch marker {
        case "+":
            open.value.lines.append(DiffLine(kind: .added, newNumber: open.new, text: text))
            open.new += 1
            open.newLeft -= 1
        case "-":
            open.value.lines.append(DiffLine(kind: .removed, oldNumber: open.old, text: text))
            open.old += 1
            open.oldLeft -= 1
        default:
            open.value.lines.append(
                DiffLine(kind: .context, oldNumber: open.old, newNumber: open.new, text: text))
            open.old += 1
            open.new += 1
            open.oldLeft -= 1
            open.newLeft -= 1
        }
        return open
    }

    /// `@@ -12,6 +12,8 @@ section`; a missing count means one line.
    private static func openHunk(_ line: Substring) -> OpenHunk? {
        let parts = line.split(separator: " ", maxSplits: 4, omittingEmptySubsequences: true)
        guard parts.count >= 4, parts[1].hasPrefix("-"), parts[2].hasPrefix("+"),
            let old = range(parts[1].dropFirst()), let new = range(parts[2].dropFirst())
        else { return nil }
        let section = parts.count == 5 ? String(parts[4]) : ""
        let hunk = DiffHunk(range: "\(parts[1]) \(parts[2])", section: section, lines: [])
        return (hunk, old.count, new.count, old.start, new.start)
    }

    private static func range(_ text: Substring) -> (start: Int, count: Int)? {
        let numbers = text.split(separator: ",", omittingEmptySubsequences: false).map { Int($0) }
        guard numbers.count <= 2, let start = numbers[0] else { return nil }
        return (start, numbers.count == 2 ? numbers[1] ?? 0 : 1)
    }

    /// The paths from `a/x b/x`, before rename lines refine them. A name containing " b/" is told apart
    /// by both halves being equal, which they are unless the file was renamed.
    private static func header(_ rest: String) -> DiffFile {
        let separator = rest.hasPrefix("\"") ? "\" \"" : " b/"
        let pieces = rest.components(separatedBy: separator)
        guard pieces.count > 1 else { return DiffFile(oldPath: strip(rest), newPath: strip(rest)) }
        let joined = { (parts: ArraySlice<String>) in parts.joined(separator: separator) }
        let prefix = separator == " b/" ? "b/" : "\""
        for split in 1..<pieces.count {
            let old = strip(joined(pieces[..<split])), new = strip(prefix + joined(pieces[split...]))
            if old == new { return DiffFile(oldPath: old, newPath: new) }
        }
        let last = pieces.count - 1
        return DiffFile(
            oldPath: strip(joined(pieces[..<last])), newPath: strip(prefix + joined(pieces[last...])))
    }

    private static func metadata(_ line: String, of file: inout DiffFile) {
        if line.hasPrefix("rename from ") {
            file.oldPath = String(line.dropFirst(12))
        } else if line.hasPrefix("rename to ") {
            file.newPath = String(line.dropFirst(10))
        } else if line.hasPrefix("new file mode") {
            file.oldPath = nil
        } else if line.hasPrefix("deleted file mode") {
            file.newPath = nil
        } else if line.hasPrefix("Binary files ") || line.hasPrefix("GIT binary patch") {
            file.isBinary = true
        }
    }

    /// Removes the `a/` or `b/` prefix and the quotes git puts around unusual names (their escapes stay).
    private static func strip(_ path: String) -> String {
        let path = path.trimmingCharacters(in: ["\""])
        return path.hasPrefix("a/") || path.hasPrefix("b/") ? String(path.dropFirst(2)) : path
    }
}
