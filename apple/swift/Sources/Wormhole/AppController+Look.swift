// How the window looks, from the command line, Settings and the control folder: the theme, the appearance, the
// status bar and the window's own translucency. All of it is one live value in `ThemeStore` and `WindowLook`, so
// every command does the same as the Settings control and nothing is redrawn by hand.

import AppKit
import DesignSystem
import WindowUI
#if WITH_UNIFFI
    import UniFFISource
    import WormholeBindings
#endif

extension AppController {
    var themes: ThemeStore { ThemeStore.shared }

    /// What `--theme`, `--appearance` and `--status-bar` asked for, before the window opens.
    func applyLookOptions() {
        if let id = options.theme { themes.setTheme(id) }
        if let mode = options.appearance { themes.mode = mode }
        look.statusBar = options.statusBar
    }

    /// The person's own theme files and colour picks: the real loader (core's) reads `<data folder>/themes`, so a
    /// user theme shows up beside the bundled ones. Without the FFI the bundled themes are all there is.
    func configureThemes() {
        #if WITH_UNIFFI
            themes.resolver = FfiThemeResolver()
            themes.themesDirectory = URL(fileURLWithPath: (try? activeProfile().dir) ?? dataDirectory.path).appendingPathComponent("themes")
        #endif
    }
}

extension AppController {
    /// The control folder's look commands; false when `command` is not one of them.
    func runLook(command: String, argument: String) -> Bool {
        func on(_ flag: inout Bool) { flag = argument == "on" ? true : argument == "off" ? false : !flag }
        switch command {
        case "theme":
            guard themes.library.theme(id: argument) != nil else { reply("error: no theme \(argument)"); return true }
            themes.setTheme(argument)
        case "appearance":
            guard let mode = ThemeMode(rawValue: argument) else { reply("error: appearance is light, dark or system"); return true }
            themes.mode = mode
        case "status-bar": on(&look.statusBar)
        case "tabs": on(&look.tabsOnTop)
        case "gaps": on(&look.splitGaps)
        case "blur": on(&look.blur)
        case "glass": on(&look.liquidGlass)
        case "dividers": on(&look.columnDividers)
        case "opacity":
            guard let value = Double(argument), (0.3...1).contains(value) else { reply("error: opacity is 0.3 to 1"); return true }
            look.opacity = value
        case "reduce-transparency", "increase-contrast":
            // The system's own switches are followed live; this stands in for them where a check cannot flip the setting.
            if command == "reduce-transparency" { on(&themes.reduceTransparency) } else { on(&themes.increaseContrast) }
        default: return false
        }
        reply("\(command) \(argument)")
        return true
    }

    /// One line of what the look is now, for `state`.
    var lookState: String {
        "theme=\(themes.theme.id) appearance=\(themes.appearance.rawValue) mode=\(themes.mode.rawValue) "
            + "opacity=\(look.opacity) blur=\(look.blur) glass=\(look.liquidGlass) status_bar=\(look.statusBar) "
            + "tabs=\(look.tabsOnTop) gaps=\(look.splitGaps) dividers=\(look.columnDividers) reduce_transparency=\(themes.reduceTransparency) increase_contrast=\(themes.increaseContrast)"
    }

    /// What the settings of Settings → Appearance, Fonts, Terminals & sidebar and Git came to, as the windows hold them now:
    /// the control folder's `look-keys`, which a check reads after it flipped a key.
    var settingsEffects: String {
        let tokens = themes.tokens
        let panel = hub.panel.preferences
        let sidebar = look.sidebar
        let secondary = themes.secondary
        return """
            accent primary=\(tokens.rgba(named: "primary")?.hex ?? "-") branch=\(tokens.rgba(named: "git.branch")?.hex ?? "-") \
            border=\(tokens.rgba(named: "border")?.hex ?? "-") card_edge=\(tokens.rgba(named: "card.edge")?.hex ?? "-")
            interface_font family=\(tokens.metrics.uiFontFamily ?? "system") size=\(tokens.metrics.uiFontSize)
            code_font family=\(look.codeFont.family ?? "system-mono") size=\(look.codeFont.size) tab_width=\(panel.tabWidth) binary_sizes=\(panel.binarySizes)
            panel changes_tree=\(panel.changesTree) group_untracked=\(panel.groupUntracked) diff_split=\(panel.diffSplit) diff_wrap=\(panel.diffWrap)
            icon_animations=\(look.iconAnimations)
            sidebar scope=\(sidebar.changesScope.rawValue) status_dot=\(sidebar.showsStatus) cmd_hints=\(sidebar.showsCommandHints) numbers_follow_list=\(hub.state.layout?.numbersFollowList.description ?? "-")
            secondary_windows settings=\(secondary.settings) studio=\(secondary.studio) hosts=\(secondary.hosts) opacity=\(secondary.opacity) blur=\(secondary.blur)
            row_counts \(hub.state.sections.flatMap(\.rows).map { "\($0.terminal.id):number=\($0.number.map(String.init) ?? "-"),pinned=\($0.terminal.isPinned),changes=\($0.terminal.changeCount),repo_files=\($0.terminal.repoFiles.map(String.init) ?? "-")" }.joined(separator: " "))
            """
    }
}
