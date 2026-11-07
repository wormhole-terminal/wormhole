// What the last save, export or copy did, under the title row (`studio/save.rs` `Notice`).

import Foundation

public enum StudioNotice: Sendable, Equatable {
    case saved(URL, setAsTheme: Bool)
    case copySaved(name: String, URL)
    case exported(URL)
    case copiedJSON
    case failed(String)

    public var text: String {
        switch self {
        case .saved(let url, let set): set ? StudioText.savedAndSet(url.path) : StudioText.saved(url.path)
        case .copySaved(let name, let url): StudioText.copySaved(name, url.path)
        case .exported(let url): StudioText.exported(url.path)
        case .copiedJSON: StudioText.copiedJSON
        case .failed(let reason): reason
        }
    }

    public var isFailure: Bool { if case .failed = self { true } else { false } }
    /// Where Reveal in Finder goes, for a notice that wrote a file.
    public var file: URL? {
        switch self {
        case .saved(let url, _), .copySaved(_, let url), .exported(let url): url
        default: nil
        }
    }
}
