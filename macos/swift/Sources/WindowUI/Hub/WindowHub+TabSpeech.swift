// What VoiceOver says of a panel tab in the strip: core's words with the counts the stores hold ("Changes, 3 changed files",
// "Notes, 2 open next steps, 1 note"), as the GPUI app's tabs say them.

import PanelModel
import PanelUI

extension WindowHub {
    func spokenTab(named name: String) -> String {
        let files = panel.changes.phase == .loaded ? panel.changes.files.count : nil
        let notes = panel.notes.model
        return TabSpeech.tab(name, files, notes.tasks.remaining, notes.filledNotes)
    }
}
