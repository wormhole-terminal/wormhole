// The fonts the app makes available to CoreText for this run (`fonts.rs` `init` and `poll_custom`): the bundled family
// at launch, and the files of the profile's fonts folder when the app starts and each time core says one was added,
// replaced or removed. Core decides what a font file is and what changed (`font_folder_changes`); this only calls
// CoreText with the answer. A custom family named like the bundled one replaces it, as it does there.

import CoreGraphics
import CoreText
import FontPickerModel
import Foundation
import WormholeBindings

@MainActor
final class FontFolder {
    /// How a file was registered. CoreText can drop a font it was given as bytes at any time, but a font it was given as
    /// a path only while the file is still there (the error is "not registered"), and a removed file is just that.
    private enum Registration {
        case bytes(CGFont)
        case path(URL)
    }

    private(set) var state = FontFolderState()
    private var registered: [String: Registration] = [:]
    private var families: [String: [String]] = [:]
    private var bundled: [URL]?

    /// `bundled` are the family's files, registered with the first look at the folder unless a custom font replaces it.
    init(bundled: [URL]) { self.bundled = bundled }

    /// Registers and unregisters what changed since the last call; whether anything did.
    func take(from handle: SettingsHandle) -> Bool {
        let changes = handle.fontFolderChanges()
        defer { bundled = nil }
        // A changed file is in both lists: dropped first, so the new bytes are the ones CoreText reads.
        for path in changes.removed {
            unregister(path)
            families[path] = nil
        }
        for file in changes.added {
            register(file.path)
            families[file.path] = file.families.map(\.name)
        }
        let names = Set(families.values.joined())
        if let bundled, !names.contains(fontBundledFamily()) {
            bundled.forEach { _ = CTFontManagerRegisterFontsForURL($0 as CFURL, .process, nil) }
        }
        guard !changes.removed.isEmpty || !changes.added.isEmpty else { return false }
        state = FontFolderState(revision: state.revision + 1, families: names)
        return true
    }

    private func register(_ path: String) {
        let url = URL(fileURLWithPath: path)
        // A collection is several fonts to CoreText but one to a `CGFont`, so those stay registered by path.
        let isCollection = ["ttc", "otc"].contains(url.pathExtension.lowercased())
        if !isCollection, let data = try? Data(contentsOf: url), let provider = CGDataProvider(data: data as CFData),
            let font = CGFont(provider), CTFontManagerRegisterGraphicsFont(font, nil)
        {
            registered[path] = .bytes(font)
        } else if CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil) {
            registered[path] = .path(url)
        }
    }

    private func unregister(_ path: String) {
        switch registered.removeValue(forKey: path) {
        case .bytes(let font)?: CTFontManagerUnregisterGraphicsFont(font, nil)
        case .path(let url)?: CTFontManagerUnregisterFontsForURL(url as CFURL, .process, nil)
        case nil: break
        }
    }
}
