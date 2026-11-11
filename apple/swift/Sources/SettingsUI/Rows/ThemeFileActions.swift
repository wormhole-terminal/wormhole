// What the theme cards do with files (`theme_picker`): Import theme… copies a theme file into the profile's themes folder and picks
// it, Move to Trash… (on a theme the person imported) asks first and says which picks change. The open panel has a seam for a
// driver (`WORMHOLE_THEME_IMPORT_FILE`, a path in a scratch folder), so a test instance never shows one.

import AppKit
import AuxModel
import AuxWindows
import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ImportThemeButton: View {
    let model: SettingsModel
    @State private var failure: String?
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._3) {
            ActionButton(title: Words.text("settings-appearance-import-theme", fallback: "Import theme…"), symbol: "arrow.down", frameId: "import-theme") { importFile() }
            if let open = model.openStudio {
                ActionButton(title: Words.text("settings-appearance-theme-studio", fallback: "Theme Studio"), symbol: "paintpalette", frameId: "open-studio", action: open)
            }
            if let failure { Text(failure).textRole(.caption).foregroundStyle(tokens.statusDangerText) }
        }
    }

    private func importFile() {
        guard let path = ThemeFiles.chosenFile() else { return }
        failure = model.host.importTheme(path: path)
        ThemeStore.shared.reloadThemes()
    }
}

enum ThemeFiles {
    @MainActor static func chosenFile() -> String? {
        if let seam = ProcessInfo.processInfo.environment["WORMHOLE_THEME_IMPORT_FILE"], !seam.isEmpty { return seam } // text: environment variable
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.prompt = Words.text("settings-window-import-theme", fallback: "Import theme")
        return PanelPick.run(panel)?.path
    }

    /// Move to Trash…: the question (a sheet on the Settings window, held for a driver in a test instance), then the move.
    @MainActor static func trash(_ value: String, model: SettingsModel) {
        guard let plan = model.host.themeTrashPlan(value) else { return }
        Task {
            let confirm = Words.text("settings-appearance-move-to-trash", fallback: "Move to Trash")
            guard await SettingsQuestion.confirm(title: plan.message, detail: plan.detail, confirm: confirm) else { return }
            _ = model.host.trashTheme(value)
            ThemeStore.shared.reloadThemes()
        }
    }
}
