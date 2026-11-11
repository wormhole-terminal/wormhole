// The profile chip's menu: "Switch Wormhole to" every profile (the current one checked, the rest noting that
// every window switches), then Open a window in… and Move this window to… (a window already in a profile
// can't move to it), New profile…, Manage profiles… and Settings….

import SwiftUI

struct ProfileMenuItems: View {
    let choices: [ProfileChoice]
    let actions: ProfileActions

    var body: some View {
        Section(PanesText.switchTo) {
            ForEach(choices) { choice in
                Toggle(isOn: Binding(get: { choice.current }, set: { _ in actions.switchTo(choice.id) })) {
                    Label { Text(choice.name) } icon: { ProfileAvatar(choice: choice, size: 16) }
                }
            }
        }
        if let openWindow = actions.openWindow, let moveWindow = actions.moveWindow {
            Divider()
            Menu(PanesText.openWindowIn) {
                ForEach(choices) { choice in Button(choice.name) { openWindow(choice.id) } }
            }
            Menu(PanesText.moveWindowTo) {
                ForEach(choices.filter { !$0.here }) { choice in Button(choice.name) { moveWindow(choice.id) } }
            }
        }
        Divider()
        Button(PanesText.newProfile, action: actions.newProfile)
        Button(PanesText.manageProfiles, action: actions.manage)
        Divider()
        Button(PanesText.settings, action: actions.settings).keyboardShortcut(",", modifiers: .command)
    }
}
