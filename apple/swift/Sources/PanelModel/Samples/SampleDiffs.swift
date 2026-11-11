// Parsed diffs for previews, screenshots and tests (the real ones are core's `git::diff`): several hunks, a new file,
// a rename, a missing final newline and a binary file.

extension DiffHunk {
    /// A hunk written as git prints its lines (" keep", "+add", "-gone"), numbered from the header's `-a +b` starts.
    init(_ header: String, _ printed: String...) {
        let starts = header.split(separator: " ").dropFirst().prefix(2).map { Int($0.dropFirst().split(separator: ",")[0]) ?? 1 }
        var (old, new) = (starts[0], starts[1])
        let lines = printed.map { line -> DiffLine in
            let text = String(line.dropFirst())
            switch line.first {
            case "+": defer { new += 1 }; return DiffLine(kind: .added, newNumber: new, text: text)
            case "-": defer { old += 1 }; return DiffLine(kind: .removed, oldNumber: old, text: text)
            default: defer { (old, new) = (old + 1, new + 1) }; return DiffLine(kind: .context, oldNumber: old, newNumber: new, text: text)
            }
        }
        self.init(header: header, lines: lines)
    }
}

enum SampleDiffs {
    static let sidebar = DiffFile(
        oldPath: "crates/wormhole/src/sidebar/list.rs", newPath: "crates/wormhole/src/sidebar/list.rs",
        hunks: [
            DiffHunk(
                "@@ -41,8 +41,10 @@ impl SidebarList {",
                "     /// The rows in the order they are drawn.", "     pub fn rows(&self) -> impl Iterator<Item = &Row> {",
                "-        self.rows.iter()", "+        self.rows.iter().filter(|row| row.visible())", "     }", " ",
                "-    fn select(&mut self, ix: usize) {", "-        self.selected = Some(ix);",
                "+    fn select(&mut self, ix: usize) -> bool {", "+        let changed = self.selected != Some(ix);",
                "+        self.selected = Some(ix);", "+        changed", "     }"),
            DiffHunk(
                "@@ -88,5 +90,6 @@ impl SidebarList {", "         }", "     }",
                "+    // Pins keep their place when a filter hides the rows around them.", "     fn pinned_first(&self) -> Vec<usize> {",
                "         let mut order: Vec<usize> = (0..self.rows.len()).collect();",
                "         order.sort_by_key(|&ix| !self.rows[ix].pinned);"),
        ])

    static let settings: DiffFile = {
        var hunk = DiffHunk(
            "@@ -12,4 +12,4 @@ pub struct Settings {", "     pub font_size: f32,", "-    pub line_height: f32,",
            "+    pub line_height: Option<f32>,", "     pub theme: String,", " }")
        hunk.lines[hunk.lines.count - 1].noNewline = true
        return DiffFile(
            oldPath: "crates/wormhole-core/src/config/settings.rs", newPath: "crates/wormhole-core/src/config/settings.rs", hunks: [hunk])
    }()

    static let newFile = DiffFile(
        oldPath: nil, newPath: "apple/swift/Sources/PanelModel/TaskList.swift",
        hunks: [
            DiffHunk(
                "@@ -0,0 +1,5 @@", "+public struct TaskList {", "+    public var items: [PanelTask]", "+",
                "+    public mutating func toggle(_ id: UUID) {}", "+}")
        ])

    static let renamed = DiffFile(
        oldPath: "docs/panel.md", newPath: "docs/notes-panel.md",
        hunks: [DiffHunk("@@ -1,3 +1,3 @@", "-# Panel", "+# Notes panel", " ", " The right column of the window.")])

    static let binary = DiffFile(oldPath: "packaging/icon.png", newPath: "packaging/icon.png", isBinary: true)
}
