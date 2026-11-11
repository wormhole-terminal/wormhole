// What choosing a row of the status item, the Dock menu or the quick panel does (`menubar::MenuCommand`).

import Foundation

public enum AmbientCommand: Hashable, Sendable {
    case showApp
    case newTerminal
    case newWindow
    /// Pick a folder and open a new terminal there.
    case openFolder
    /// Bring the tab forward, in whichever window it is.
    case openTab(Int64)
    /// Type a reply into the tab and press Return, without bringing anything forward.
    case reply(tab: Int64, text: String)
    case runAction(root: String, id: String)
    case openProject(path: String)
    case jumpToWaiting
    case openSettings
    case checkForUpdates
    case quit
}
