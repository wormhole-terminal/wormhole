// Where a theme leaves the Studio: the save panel Export JSON asks, and the pasteboard Copy JSON fills. Both are
// seams so a check points them at a scratch folder and never touches the person's files or clipboard.

import AppKit
import UniformTypeIdentifiers

/// The Studio's two ways out for a theme as text, replaceable by a check.
public struct StudioOutputs {
    /// Where the exported file goes, or `nil` when the person cancelled; `name` is the suggested file name.
    public var exportDestination: @MainActor (_ name: String, _ window: NSWindow?) async -> URL?
    public var copy: @MainActor (_ json: String) -> Void

    public init(
        exportDestination: @escaping @MainActor (String, NSWindow?) async -> URL?, copy: @escaping @MainActor (String) -> Void
    ) {
        (self.exportDestination, self.copy) = (exportDestination, copy)
    }

    /// The system's: a save panel (a sheet on the window) and the general pasteboard.
    @MainActor public static let system = StudioOutputs(
        exportDestination: { name, window in
            let panel = NSSavePanel()
            panel.nameFieldStringValue = name
            panel.allowedContentTypes = [.json]
            panel.isExtensionHidden = false
            guard let window else { return panel.runModal() == .OK ? panel.url : nil }
            return await withCheckedContinuation { continuation in
                panel.beginSheetModal(for: window) { continuation.resume(returning: $0 == .OK ? panel.url : nil) }
            }
        },
        copy: { json in
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(json, forType: .string)
        })
}
