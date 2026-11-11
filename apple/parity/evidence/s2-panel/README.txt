See driven.txt. Findings of the wave-1 review handled in this lane: M5 (hostile line offsets) fixed, test: PanelTests/PanelParityTests
testADrawnLineNeverPutsAnOffsetPastItsEnd and FilesBridgeTests/VisibleLineTests testHostileLinesStayInsideTheirDrawnText, Rust
visible_line tests in apple/ffi/src/visible.rs; S9 the rule moved to wormhole-core view::visible and is called through the FFI;
S10 git problem wording moved to wormhole-core git::problem (FFI git_problem); S11 the widest line is measured once when the diff is
read (DiffFile.widest); N5 the global rules are installed once in PlatformSetup (FilesBridge.installRules).
