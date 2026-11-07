// Export Text As… (⌘S, reserved by the keymap "as in Terminal.app"): an `NSSavePanel` for a plain-text file named
// after the terminal, over the window as a sheet. The write is atomic and the file is UTF-8 with the last line ended,
// so `cat` and editors treat it as a text file. Errors are said in the system's own alert.

import AppKit
import UniformTypeIdentifiers

@MainActor
public enum TextExporter {
    /// The file name a terminal's export suggests: its title without path separators, `.txt`.
    public static func suggestedName(title: String) -> String {
        let clean = title.replacingOccurrences(of: "/", with: "-").replacingOccurrences(of: ":", with: "-")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return (clean.isEmpty ? L10n.exportDefaultName : clean) + ".txt"
    }

    /// The bytes of the file: the text, ended by a newline.
    public static func data(for text: String) -> Data {
        Data((text.hasSuffix("\n") ? text : text + "\n").utf8)
    }

    /// Shows the panel as a sheet on `window` (or modal without one); `completion` gets the file written, if any.
    public static func export(
        _ text: String, title: String, over window: NSWindow?, completion: @escaping (URL?) -> Void = { _ in }
    ) {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.plainText]
        panel.nameFieldStringValue = suggestedName(title: title)
        panel.canCreateDirectories = true
        panel.isExtensionHidden = false
        let finish: (NSApplication.ModalResponse) -> Void = { response in
            guard response == .OK, let url = panel.url else { return completion(nil) }
            do {
                try data(for: text).write(to: url, options: .atomic)
                completion(url)
            } catch {
                NSAlert(error: error).runModal()
                completion(nil)
            }
        }
        if let window { panel.beginSheetModal(for: window, completionHandler: finish) } else { finish(panel.runModal()) }
    }
}
