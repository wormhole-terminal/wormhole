// What VoiceOver says of a tab in the panel's strip ("Changes, 3 changed files", "Notes, 2 open next steps, 1 note"): the core's
// words (`view::spoken::panel_tab`). The app installs core's at launch (`FilesBridge.installRules()`); until then, in previews
// and tests, the bare name answers.

public enum TabSpeech {
    // why: written once on the main thread at launch, before any window reads it; every later access is a read.
    nonisolated(unsafe) public static var tab: @Sendable (_ name: String, _ files: Int?, _ openSteps: Int, _ notes: Int) -> String = { name, _, _, _ in
        name.capitalized
    }
}
