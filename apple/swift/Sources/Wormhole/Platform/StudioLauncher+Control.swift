// The control folder's `studio-out`: Export JSON, Copy JSON, Save theme and Duplicate on the open Studio, with the
// save panel and the pasteboard replaced by files in `<data dir>/studio-out`, so a check never opens a system panel,
// writes where the person keeps files or touches their clipboard.

import Foundation
import StudioUI

extension AppController {
    /// `studio-out export|copy-json|save-theme|duplicate`; the files land in `<data dir>/studio-out` (`pasteboard.txt` is the copy).
    func runStudioOutputs(_ argument: String) {
        guard studio.isOpen else { return reply("error: the Studio is not open (`studio`)") }
        let folder = dataDirectory.appendingPathComponent("studio-out")
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        studio.setOutputs(
            StudioOutputs(
                exportDestination: { name, _ in folder.appendingPathComponent(name) },
                copy: { try? $0.write(to: folder.appendingPathComponent("pasteboard.txt"), atomically: true, encoding: .utf8) }))
        Task { @MainActor in
            switch argument {
            case "export": await studio.exportJSON()
            case "copy-json": studio.copyJSON()
            case "save-theme": await studio.saveTheme()
            case "duplicate": studio.duplicateTheme()
            default: break
            }
            let files = (try? FileManager.default.contentsOfDirectory(atPath: folder.path))?.sorted() ?? []
            reply("studio-out \(argument): notice=\(studio.document?.notice.map { "\($0)" } ?? "none") files=\(files) in \(folder.path)")
        }
    }
}
