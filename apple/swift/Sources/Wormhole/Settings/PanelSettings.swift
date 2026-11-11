// What Settings → Git and Fonts say of the right panel: the Changes list's tree and grouping, the diff's layout and
// wrapping (a click on the panel's own controls writes them back), the tab width, the units of a size, and the code font
// (the terminal's family while none is named, `FontSlot::Code`).

import DesignSystem
import PanelModel
import PanelUI
import SettingsModel
import WindowUI

#if WITH_UNIFFI
    import WormholeBindings
#endif

@MainActor
enum PanelSettings {
    static func apply(_ values: [String: JSONValue], to hubs: [WindowHub]) {
        for hub in hubs {
            let panel = hub.panel.preferences
            panel.changesTree = values["changes_tree"]?.bool ?? true
            panel.groupUntracked = values["group_untracked"]?.bool ?? true
            panel.diffSplit = values["diff_split"]?.bool ?? true
            panel.diffWrap = values["diff_wrap"]?.bool ?? false
            panel.tabWidth = values["tab_width"]?.number.map { max(1, Int($0)) } ?? 4
            panel.binarySizes = values["size_units"]?.string == "binary"
            hub.panel.worktrees.setOptions(
                base: values["worktree_base"]?.string, cloneBuildFolders: values["worktree_clone_build_folders"]?.bool ?? false)
        }
    }

    /// The panel's own switches write to the settings; a value that came from there is not written again.
    static func link(_ hub: WindowHub, to model: SettingsModel) {
        hub.panel.preferences.persist = { [weak model] key, value in
            if let model, model.value(key).bool != value { model.set(key, .bool(value)) }
        }
    }

    static func codeFont(_ values: [String: JSONValue]) -> CodeFont {
        let own = values["code_font_family"]?.string?.trimmingCharacters(in: .whitespaces) ?? ""
        let family = own.isEmpty ? values["font_family"]?.string : own
        return CodeFont(family: family, size: values["code_font_size"]?.number ?? 13)
    }
}
