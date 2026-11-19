// What Settings → Git says of the panel (`changes_tree`, `group_untracked`, `diff_split`, `diff_wrap`, `tab_width`,
// `size_units`), kept once for every tab. The stores read the four the panel's own controls also change through it, so
// a click and a setting are one switch; the app feeds it from the settings and `persist` writes a click back.

import Observation
import PanelModel

@MainActor @Observable
public final class PanelPreferences {
    public var changesTree = true { didSet { echo("changes_tree", changesTree, oldValue) } }
    public var groupUntracked = true { didSet { echo("group_untracked", groupUntracked, oldValue) } }
    public var diffSplit = true { didSet { echo("diff_split", diffSplit, oldValue) } }
    public var diffWrap = false { didSet { echo("diff_wrap", diffWrap, oldValue) } }
    /// Columns a tab stands for in the diff and the file viewer.
    public var tabWidth = 4 { didSet { if tabWidth != oldValue { tabObservers.forEach { $0() } } } }
    /// The open diffs, whose sideways extent depends on the tab width (`BuildKey` in the GPUI app's rows).
    @ObservationIgnored var tabObservers: [@MainActor () -> Void] = []
    /// Sizes in the viewer are written in KiB and MiB instead of KB and MB.
    public var binarySizes = false { didSet { ByteSize.binary = binarySizes } }
    /// Writes a changed switch back to the settings (the app compares first, so a value that came from there is not echoed).
    @ObservationIgnored public var persist: @MainActor (_ key: String, _ value: Bool) -> Void = { _, _ in }

    public init() {}

    private func echo(_ key: String, _ value: Bool, _ old: Bool) { if value != old { persist(key, value) } }
}
