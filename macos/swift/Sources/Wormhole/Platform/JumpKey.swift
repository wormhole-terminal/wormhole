// The key that jumps to the terminal waiting longest is the person's to choose (`jump_waiting_key`, ⌥⌘J unless they recorded
// another): Go to Waiting Terminal takes it in the menu bar and in the sheet, and the old key goes back to whatever else has it.

import PlatformModel

extension AppController {
    func bindJumpKey(_ text: String) {
        let keys = (try? KeySpec(parsing: text)).map { [$0] } ?? []
        Keymap.install(Keymap.shared.replacingKeys(of: .goToWaiting, with: keys))
        menus?.windowsChanged()
    }
}
