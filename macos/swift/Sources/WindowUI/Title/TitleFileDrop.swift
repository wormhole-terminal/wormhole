// Files dropped on the title row go to the terminal under it, as a drop on the grid does (`title/mod.rs`
// `drop_files_on_titlebar`: the row covers the top of the terminal, so the terminal would never receive them). The row
// wears a tint while files hover over it.

import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct TitleFileDrop: ViewModifier {
    let drop: @MainActor ([URL]) -> Void
    @State private var targeted = false

    func body(content: Content) -> some View {
        content
            .background(targeted ? Color(nsColor: .controlAccentColor).opacity(0.2) : .clear)
            .onDrop(of: [.fileURL], isTargeted: $targeted) { providers in
                let files = providers.filter { $0.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) }
                guard !files.isEmpty else { return false }
                Task { @MainActor in
                    var urls: [URL] = []
                    for provider in files {
                        let url: URL? = await withCheckedContinuation { continuation in
                            _ = provider.loadObject(ofClass: URL.self) { url, _ in continuation.resume(returning: url) }
                        }
                        if let url { urls.append(url) }
                    }
                    drop(urls)
                }
                return true
            }
    }
}
