// text-file: a developer tool's note (inspect mode), English like the GPUI inspector's; never interface text
// The note inspect mode copies, in the GPUI inspector's format (`inspector/edit.rs`): where the element is built and
// what it is called, the line of code, then its size and style in the design system's terms, then the tokens it is
// drawn with (the Studio gallery's table, core's `TOKEN_USES`):
//
//   SidebarUI/RowView.swift:42  sidebar.row
//     Row(isSelected: selected, selection: space) {
//     184×44 · padding x _2 · radius row
//     tokens: sidebar.row.selected, state.hover, text.primary
//     change: padding x _2 → _3
//
// The line of code is read from the checkout a debug build was made from; a release build has no such path in it.

import DesignSystem
import Foundation
import StudioUI

@MainActor
enum InspectNote {
    static func text(for element: InspectedElement) -> String {
        var note = "\(element.site.location)  \(element.site.id)"
        if let code = sourceLine(of: element.site) { note += "\n  \(code)" }
        note += "\n  \(summary(of: element))"
        let tokens = tokens(of: element.site.id)
        if !tokens.isEmpty { note += "\n  tokens: \(tokens.joined(separator: ", "))" }
        let changes = InspectNudge.changes(of: element.site.style)
        if !changes.isEmpty { note += "\n  change: \(changes.joined(separator: "; "))" }
        return note
    }

    /// `184×44 · padding x _2 · radius row`
    static func summary(of element: InspectedElement) -> String {
        let size = String(format: "%.0f×%.0f", element.frame.width, element.frame.height)
        return element.site.style.isEmpty ? size : "\(size) · \(element.site.style)"
    }

    /// The tokens the component `id` is drawn with; none for an element the gallery's table does not have.
    static func tokens(of id: String) -> [String] { uses[id] ?? [] }

    private static let uses = Dictionary(GalleryKnowledge().drawn.map { ($0.id, $0.tokens) }, uniquingKeysWith: { first, _ in first })

    /// Source files by `#fileID`, split into lines.
    private static var sources: [String: [String]?] = [:]

    /// The trimmed source line the element was built on.
    static func sourceLine(of site: InspectSite) -> String? {
        let file = "\(site.file)"
        let lines: [String]?
        if let known = sources[file] {
            lines = known
        } else {
            lines = read(file)
            sources[file] = lines
        }
        guard let lines, site.line >= 1, lines.indices.contains(Int(site.line) - 1) else { return nil }
        let text = lines[Int(site.line) - 1].trimmingCharacters(in: .whitespaces)
        return text.isEmpty ? nil : text
    }

    /// `Module/File.swift` under the checkout's `Sources` (a `#fileID` has no folders below the module, so it is looked for).
    private static func read(_ fileID: String) -> [String]? {
        #if DEBUG
            let parts = fileID.split(separator: "/", maxSplits: 1).map(String.init)
            guard parts.count == 2 else { return nil }
            // This file is `Sources/Wormhole/Inspect/InspectNote.swift`.
            let sourcesFolder = URL(fileURLWithPath: "\(#filePath)").deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            let module = sourcesFolder.appendingPathComponent(parts[0])
            guard let found = FileManager.default.enumerator(at: module, includingPropertiesForKeys: nil) else { return nil }
            for case let url as URL in found where url.lastPathComponent == parts[1] {
                return (try? String(contentsOf: url, encoding: .utf8))?.components(separatedBy: "\n")
            }
            return nil
        #else
            return nil
        #endif
    }
}
