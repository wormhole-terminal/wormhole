import Foundation

extension L10n {
    static var minimap: String { tr("terminal-minimap", "Minimap") }
    /// The slider's value for VoiceOver: where the view's first line is among all the lines.
    static func minimapValue(_ top: Int, _ total: Int) -> String { tr("terminal-minimap-value", ["line": .whole(top + 1), "total": .whole(total)], "line \(top + 1) of \(total)") }
}
