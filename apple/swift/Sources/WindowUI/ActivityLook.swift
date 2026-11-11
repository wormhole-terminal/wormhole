// How the chrome says what a terminal is doing: the glyph, the words ("working") and the colour of the words.
// The sidebar has its own mapping for its rows; the title row and the status bar read it here, in the bar's
// own vocabulary.

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

extension Activity {
    var status: StatusKind {
        switch self {
        case .idle, .exited: .idle
        case .working: .working
        case .needsInput: .attention
        case .finished: .done
        case .failed: .failed
        }
    }

    /// Nothing to say of a terminal that is idle.
    var stateWords: String? {
        switch self {
        case .idle: nil
        case .exited: Words.text("window-state-exited", fallback: "exited")
        case .working: Words.text("window-state-working", fallback: "working")
        case .needsInput: Words.text("window-state-needs-you", fallback: "needs you")
        case .finished: Words.text("window-state-done", fallback: "done")
        case .failed: Words.text("window-state-failed", fallback: "failed")
        }
    }

    func stateColour(_ colours: ChromeColours) -> Color {
        switch self {
        case .needsInput: colours.attentionText
        case .failed: colours.dangerText
        case .idle, .working, .finished, .exited: colours.muted
        }
    }
}
