// The app's own small windows, drawn small (`gallery/windows.rs`): the Session Hosts window's badges and cards, the first
// run's step card and the Licenses window's rows and text, each by the window's own views. About is the system's panel
// in the Swift app, so it is shown by opening it.

import AuxWindows
import DesignSystem
import SwiftUI

struct WindowsSpecimen: View {
    var body: some View {
        VStack(alignment: .leading, spacing: Space._6) {
            AuxWindowsSpecimen()
            FirstRunSpecimen()
            LicensesSpecimen()
            SystemNote(StudioWords.systemAbout) { Button(StudioWords.showAbout) { AboutPanel.show() } }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
