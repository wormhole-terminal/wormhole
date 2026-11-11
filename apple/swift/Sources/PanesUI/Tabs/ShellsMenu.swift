// "New terminal with" under a right-click of a + (the sidebar's title row, the tab bar's): every shell by name, the one a
// plain click opens checked, as Terminal.app's "New Tab with Profile" (`sidebar/shells.rs`). The shells and their keys are
// the core's; what choosing one does is the window's.

import SwiftUI

public struct ShellOffer: Identifiable, Equatable, Sendable {
    public let key: String
    public let name: String
    /// What a plain New terminal opens.
    public let isDefault: Bool
    public var id: String { key }

    public init(key: String, name: String, isDefault: Bool) { (self.key, self.name, self.isDefault) = (key, name, isDefault) }
}

public struct ShellsMenu: View {
    let shells: [ShellOffer]
    let choose: (String) -> Void

    public init(shells: [ShellOffer], choose: @escaping (String) -> Void) { (self.shells, self.choose) = (shells, choose) }

    public var body: some View {
        Section(PanesText.newTerminalWith) {
            ForEach(shells) { shell in
                Toggle(shell.name, isOn: Binding(get: { shell.isDefault }, set: { _ in choose(shell.key) }))
            }
        }
    }
}
