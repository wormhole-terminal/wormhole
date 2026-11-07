// Export JSON…: the theme as a file the person puts where they want. A save panel as a sheet on the window,
// starting at the theme's name in lower case with dashes; the panel remembers the folder of the last one.

import AppKit
import StudioModel
import UniformTypeIdentifiers

extension StudioSession {
    public func exportJSON() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = document.exportFileName
        panel.allowedContentTypes = [.json]
        panel.isExtensionHidden = false
        let finish: (NSApplication.ModalResponse) -> Void = { [document] response in
            guard response == .OK, let url = panel.url else { return }
            do { try document.exported(to: url) } catch { document.report(error) }
        }
        if let window { panel.beginSheetModal(for: window, completionHandler: finish) } else { finish(panel.runModal()) }
    }
}
