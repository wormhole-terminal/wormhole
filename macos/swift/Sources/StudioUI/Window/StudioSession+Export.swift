// Export JSON… and Copy JSON: the theme as a file the person puts where they want, and as text on the pasteboard.
// Both go through `outputs`, the system's panel and pasteboard unless a check replaced them.

import StudioModel

extension StudioSession {
    public func exportJSON() async {
        guard let url = await outputs.exportDestination(document.exportFileName, environment.exportFolder?.recall(), window) else { return }
        do {
            try document.exported(to: url)
            environment.exportFolder?.remember(url.deletingLastPathComponent())
        } catch {
            document.report(error)
        }
    }

    public func copyJSON() {
        outputs.copy(document.exportJSON)
        document.copiedJSON()
    }
}
