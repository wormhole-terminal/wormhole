// Diff text as `git diff` prints it, for previews and screenshots. Shapes the parser must cope with:
// several hunks, a new file, a rename, a missing final newline and a binary file.

enum SampleDiffs {
    static let sidebar = """
        diff --git a/crates/wormhole/src/sidebar/list.rs b/crates/wormhole/src/sidebar/list.rs
        index 4e1c2a7..9b03d5f 100644
        --- a/crates/wormhole/src/sidebar/list.rs
        +++ b/crates/wormhole/src/sidebar/list.rs
        @@ -41,8 +41,10 @@ impl SidebarList {
             /// The rows in the order they are drawn.
             pub fn rows(&self) -> impl Iterator<Item = &Row> {
        -        self.rows.iter()
        +        self.rows.iter().filter(|row| row.visible())
             }

        -    fn select(&mut self, ix: usize) {
        -        self.selected = Some(ix);
        +    fn select(&mut self, ix: usize) -> bool {
        +        let changed = self.selected != Some(ix);
        +        self.selected = Some(ix);
        +        changed
             }
        @@ -88,5 +90,6 @@ impl SidebarList {
                 }
             }
        +    // Pins keep their place when a filter hides the rows around them.
             fn pinned_first(&self) -> Vec<usize> {
                 let mut order: Vec<usize> = (0..self.rows.len()).collect();
                 order.sort_by_key(|&ix| !self.rows[ix].pinned);

        """

    static let settings = """
        diff --git a/crates/wormhole-core/src/config/settings.rs b/crates/wormhole-core/src/config/settings.rs
        index 1a2b3c4..5d6e7f8 100644
        --- a/crates/wormhole-core/src/config/settings.rs
        +++ b/crates/wormhole-core/src/config/settings.rs
        @@ -12,4 +12,4 @@ pub struct Settings {
             pub font_size: f32,
        -    pub line_height: f32,
        +    pub line_height: Option<f32>,
             pub theme: String,
         }
        \\ No newline at end of file
        """

    static let newFile = """
        diff --git a/apple/swift/Sources/PanelModel/TaskList.swift b/apple/swift/Sources/PanelModel/TaskList.swift
        new file mode 100644
        index 0000000..c0ffee1
        --- /dev/null
        +++ b/apple/swift/Sources/PanelModel/TaskList.swift
        @@ -0,0 +1,5 @@
        +public struct TaskList {
        +    public var items: [PanelTask]
        +
        +    public mutating func toggle(_ id: UUID) {}
        +}
        """

    static let renamed = """
        diff --git a/docs/panel.md b/docs/notes-panel.md
        similarity index 88%
        rename from docs/panel.md
        rename to docs/notes-panel.md
        index 3f4a5b6..7c8d9e0 100644
        --- a/docs/panel.md
        +++ b/docs/notes-panel.md
        @@ -1,3 +1,3 @@
        -# Panel
        +# Notes panel

         The right column of the window.
        """

    static let binary = """
        diff --git a/packaging/icon.png b/packaging/icon.png
        index 2d3e4f5..6a7b8c9 100644
        Binary files a/packaging/icon.png and b/packaging/icon.png differ
        """
}
