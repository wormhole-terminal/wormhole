// A file's or folder's icon as the icon theme in force draws it, in the lead column of a row. A theme whose icons keep
// their colours draws them as they are; a mono one tints them with the row's muted colour. Where no theme is set, or it
// has no icon, the system's symbol stands in.

import AppKit
import DesignSystem
import PanelModel
import SwiftUI

extension EnvironmentValues {
    @Entry public var fileIcons: (any FileIconProviding)?
}

struct FileIcon: View {
    enum Kind { case file(String), folder(String, open: Bool), link }

    let kind: Kind
    let tint: Color
    @Environment(\.fileIcons) private var icons
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        Group {
            if let url, let image = IconImages.image(at: url) { themed(image) } else { Image(systemName: symbol).font(.system(size: Metric.glyph)).foregroundStyle(tint) }
        }
        .frame(width: Metric.lead, height: Metric.lead)
        .accessibilityHidden(true)
    }

    private var url: URL? {
        let dark = scheme == .dark
        switch kind {
        case let .file(name): return icons?.file(name, dark: dark)
        case let .folder(name, open): return icons?.folder(name, open: open, dark: dark)
        case .link: return nil
        }
    }

    @ViewBuilder private func themed(_ image: NSImage) -> some View {
        if icons?.rendersInColour == false {
            Image(nsImage: image).renderingMode(.template).resizable().scaledToFit().foregroundStyle(tint)
        } else {
            Image(nsImage: image).resizable().scaledToFit()
        }
    }

    private var symbol: String {
        switch kind {
        case .file: "doc"
        case let .folder(_, open): open ? "folder.fill" : "folder"
        case .link: "link"
        }
    }
}

/// SVGs decoded once: a long list asks for the same few hundred icons on every row.
@MainActor
enum IconImages {
    private static var cache: [URL: NSImage] = [:]

    static func image(at url: URL) -> NSImage? {
        if let hit = cache[url] { return hit }
        guard let image = NSImage(contentsOf: url) else { return nil }
        cache[url] = image
        return image
    }
}
