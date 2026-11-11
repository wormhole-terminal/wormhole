// The folder right-clicked: a native menu like the one a Finder window's title opens. The path as shown heads
// it; Open in Finder, Copy path and Insert into terminal follow; then the path's folders, the innermost
// first, down to the volume, each with the icon Finder gives it. Choosing a folder opens it in Finder.

import AppKit
import PanesModel
import SwiftUI

struct PathMenuItems: View {
    let facts: TitleFacts
    let actions: TitleActions

    var body: some View {
        // A heading is a line of text: a SwiftUI `Section` would add separators of its own above and below.
        Text(facts.shownFolder ?? "")
        Divider()
        Button(PanesText.openInFinder, action: actions.revealFolder)
        Button(PanesText.copyPath, action: actions.copyPath)
        Button(PanesText.insertIntoTerminal, action: actions.insertPath).disabled(!actions.canInsert)
        if let folder = facts.folder {
            Divider()
            ForEach(PathChain.of(folder, bootVolume: bootVolumeName), id: \.path) { item in
                Button {
                    NSWorkspace.shared.open(URL(fileURLWithPath: item.path))
                } label: {
                    Label { Text(item.name) } icon: { Image(nsImage: icon(of: item.path)) }
                }
            }
        }
    }

    /// The icon Finder gives the folder, at menu size (the house for the home folder, a disk for the volume).
    private func icon(of path: String) -> NSImage {
        let icon = NSWorkspace.shared.icon(forFile: path)
        icon.size = NSSize(width: 16, height: 16)
        return icon
    }

    /// The name of the volume the system starts from ("Macintosh HD").
    private var bootVolumeName: String {
        (try? URL(fileURLWithPath: "/").resourceValues(forKeys: [.volumeNameKey]).volumeName) ?? "Macintosh HD" // text: macOS's default name of the startup volume
    }
}
