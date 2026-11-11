// The icons the Files tab and the Changes rows draw for files and folders: the icon theme in force
// (`wormhole_core::theme::icon_theme`). Where there is none, the views draw the system's symbols.

import Foundation

public protocol FileIconProviding: Sendable {
    /// The SVG for a file (a bare name or a repo-relative path), for the light or dark variant.
    func file(_ path: String, dark: Bool) -> URL?
    func folder(_ name: String, open: Bool, dark: Bool) -> URL?
    /// Whether the icons keep their own colours, or are masks tinted with the text colour.
    var rendersInColour: Bool { get }
}
