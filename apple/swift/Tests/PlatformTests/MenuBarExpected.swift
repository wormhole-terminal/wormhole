// The menu bar the Rust core designs (`keys/keymap/tests.rs` `the_menus_are_the_designed_ones`), copied as text, so
// the Swift builder is held to core's own output line by line, keys and checks included.

enum MenuBarExpected {
    static let frontWindow = """
        Wormhole
          About Wormhole
          Check for Updates…
          ---
          Settings…  ⌘,
          Theme Studio  ⌥⌘,
          ---
          Services ▸
          ---
          Hide Wormhole  ⌘H
          Hide Others  ⌥⌘H
          Show All
          ---
          Quit Wormhole  ⌘Q
        Shell
          New Window  ⌘N
          New Terminal  ⌘T
          New Terminal in No Group  ⇧⌘T
          New Terminal With ▸
            No Shells  (disabled)
          New Terminal in New Worktree…
          ---
          Split Right  ⌘D
          Split Down  ⇧⌘D
          Move Pane Out of Split
          ---
          Open Folder…  ⌘O
          Open Recent ▸
            No Recent Folders  (disabled)
          ---
          Profile ▸
            ✓ Default
            Work
            ---
            Open Window in ▸
              Default
              Work
            Move Window to ▸
              Work
            ---
            New Profile…
            Manage Profiles…
          ---
          Close Terminal  ⌘W
          Close Window  ⇧⌘W
        Edit
          Undo  ⌘Z
          Redo  ⇧⌘Z
          ---
          Cut  ⌘X
          Copy  ⌘C
          Paste  ⌘V
          Select All  ⌘A
          ---
          Clear Buffer  ⌘K
          ---
          Find…  ⌘F
          Find Next  ⌘G
          Find Previous  ⇧⌘G
          Use Selection for Find  ⌘E
          ---
          Previous Prompt  ⌘↑
          Next Prompt  ⌘↓
          Select Output
          Copy Output
          Copy Command
          Put Command at Prompt
        View
          Hide Sidebar  ⌃⌘S
          Hide Right Panel  ⌥⌘I
          Show Status Bar  ⌘/
          Show Minimap
          Exit Zen Mode  ⌃⌘Z
          ---
          Zoom Pane  ⇧⌘↩
          Layout ▸
            2 Columns
            2 Rows
            2×2 Grid
            Single Pane
          ---
          ✓ Changes  ⌃⌘1
          History  ⌃⌘2
          Notes  ⌃⌘3
          Usage  ⌃⌘4
          Project  ⌃⌘5
          Files  ⌃⌘6
          ---
          Show Terminals in Sidebar
          ---
          Bigger  ⌘+
          Smaller  ⌘-
          Actual Size  ⌘0
          ---
          Command Palette…  ⇧⌘P
          ---
          Enter Full Screen  ⌃⌘F
        Terminal
          Pin to Top
          Hide from Agents
          ---
          Focus Terminal / Panel  ⇧⌘E
          Focus Next Area  F6
          Focus Previous Area  ⇧F6
          New Note  ⌥⌘N
          Stash Prompt  ⌥⌘S
          Project Actions ▸
            No Project Actions  (disabled)
          ---
          Reveal in Finder
          Copy Path
          ---
          Insert Folder Path…  ⌘J
        Window
          Minimize  ⌘M
          Zoom
          ---
          Next Terminal  ⌃⌘↓
          Previous Terminal  ⌃⌘↑
          Next Group  ⌃⌥]
          Previous Group  ⌃⌥[
          Go to Waiting Terminal  ⌥⌘J
          Go to Finished Terminal  ⌥⇧⌘J
          Select Pane ▸
            Left  ⌥⌘←
            Right  ⌥⌘→
            Above  ⌥⌘↑
            Below  ⌥⌘↓
          ---
          ✓ sidebar polish  ⌘1
          diff perf  ⌘2
          zsh  ⌘3
          auth tokens  ⌘4
          rate limits  ⌘5
          ---
          Bring All to Front
        Help
          Keyboard Shortcuts  ⌥⌘/
          Welcome to Wormhole
          ---
          Licenses
          Show Logs in Finder
        Developer
          Inspector  ⌥⇧⌘I

        """
}
