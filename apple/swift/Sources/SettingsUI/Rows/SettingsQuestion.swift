// The question before something that cannot be undone in Settings (a profile deleted, settings put back, a theme moved to the
// Trash): the same sheet on the Settings window as every other question of the app, so a test instance holds it for the control
// folder (`question confirm|cancel`) instead of showing a native alert nobody can answer.

import AppKit
import AuxModel
import AuxWindows
import SettingsModel

@MainActor
enum SettingsQuestion {
    /// True when the person confirmed.
    static func confirm(title: String, detail: String, confirm: String) async -> Bool {
        let window = NSApp.windows.first { $0.title == SettingsText.settings }
        if case .confirm = await QuestionSheet(Question(title: title, detail: detail, confirm: confirm)).ask(in: window) { return true }
        return false
    }
}
