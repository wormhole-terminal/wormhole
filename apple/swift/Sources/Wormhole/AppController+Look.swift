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
        case "reduce-motion":
            // Stands in for the system's Reduce Motion (SwiftUI's own flag cannot be set): the chrome's animations hold still.
            themes.reduceMotion = argument == "system" ? NSWorkspace.shared.accessibilityDisplayShouldReduceMotion : argument != "off"
        case "keyboard-navigation":
            // Stands in for macOS's Keyboard navigation: ⇥ then reaches the title row's buttons too (`system` follows macOS again).
            WindowFocus.keyboardNavigationOverride = argument == "system" ? nil : argument != "off"
        case "window-front":
            // Says this window is in front (or not) as far as the focus rings go, without moving it.
            front.hub.focus.setFront(argument == "system" ? nil : argument != "off")
        case "backdrop":
            // What is behind the window's surfaces: the effect views of its content, as AppKit holds them.
            // `backdrop [title]`: the window with that title (Settings, Theme Studio, Session Hosts), else the front workspace's.
            let shown = argument.isEmpty ? front.window : NSApp.windows.first { $0.title == argument }
            reply(shown.map { Self.backdropReport(of: $0) + (argument.isEmpty ? "\n" + look.backdropSummary : "") } ?? "no window")
            return true
        default: return false
        }
        reply("\(command) \(argument)")
        return true
    }

    /// The window's effect views (`NSVisualEffectView`, `NSGlassEffectView`) with the material, blending, state and
    /// appearance the system draws them with, and whether the window itself is see-through.
    static func backdropReport(of window: NSWindow) -> String {
        var lines = ["window opaque=\(window.isOpaque) background_clear=\(window.backgroundColor == .clear) has_shadow=\(window.hasShadow) appearance=\(window.effectiveAppearance.name.rawValue)"]
        func walk(_ view: NSView, _ depth: Int) {
            if let effect = view as? NSVisualEffectView {
                lines.append("effect material=\(effect.material.rawValue) blending=\(effect.blendingMode.rawValue) state=\(effect.state.rawValue) hidden=\(effect.isHiddenOrHasHiddenAncestor) appearance=\(effect.appearance?.name.rawValue ?? "inherited") frame=\(Int(effect.frame.width))x\(Int(effect.frame.height))")
            } else if let glass = view as? NSGlassEffectView {
                lines.append("glass style=\(glass.style.rawValue) hidden=\(glass.isHiddenOrHasHiddenAncestor) appearance=\(glass.appearance?.name.rawValue ?? "inherited") frame=\(Int(glass.frame.width))x\(Int(glass.frame.height))")
            }
            view.subviews.forEach { walk($0, depth + 1) }
        }
        window.contentView.map { walk($0, 0) }
        return lines.joined(separator: "\n")
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
