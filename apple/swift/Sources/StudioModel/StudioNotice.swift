// What the last save, export or copy did, under the title row (`studio/save.rs` `Notice`).

import Foundation

public enum StudioNotice: Sendable, Equatable {
    case saved(URL, text: String)
    case copySaved(name: String, URL)
    /// Written where the person chose; `note` says only the shown appearance of a pair went out.
    case exported(URL, note: String?)
    case copiedJSON(note: String?)
    /// Done, with nothing to reveal (a pair that now follows the system).
    case done(String)
    case failed(String)

    public var text: String {
        switch self {
        case .saved(_, let text): text
        case .copySaved(let name, let url): StudioText.copySaved(name, url.path)
        case .exported(let url, let note): [StudioText.exported(url.path), note].compactMap { $0 }.joined(separator: " ")
        case .copiedJSON(let note): [StudioText.copiedJSON, note].compactMap { $0 }.joined(separator: " ")
        case .done(let text), .failed(let text): text
        }
    }

    public var isFailure: Bool { if case .failed = self { true } else { false } }
    /// Where Reveal in Finder goes, for a notice that wrote a file.
    public var file: URL? {
        switch self {
        case .saved(let url, _), .copySaved(_, let url), .exported(let url, _): url
        default: nil
        }
    }
}
