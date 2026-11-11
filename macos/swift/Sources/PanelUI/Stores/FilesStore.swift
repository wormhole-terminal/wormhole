// The Files tab's state: the tree of the terminal's folder (or one chosen by hand), the row selected, the file open
// in the viewer, the pins. The disk is read through the backend, off the main actor, and again when its watch says
// something changed; a folder that reads the same changes nothing on screen.

import Foundation
import Observation
import PanelModel

@MainActor @Observable
public final class FilesStore {
    public internal(set) var tree = FileTree()
    public internal(set) var rows: [FileRow] = []
    /// The terminal's folder, and the one chosen by hand that takes its place until it is followed again.
    public private(set) var terminalFolder: String?
    public private(set) var chosen: String?
    public internal(set) var selected: String?
    public internal(set) var pins: [String] = []
    public internal(set) var viewing: String?
    public internal(set) var content = FileContent.reading
    /// The row whose name is a field now, and why the last rename was refused.
    public internal(set) var renaming: String?
    public internal(set) var renameError: String?
    /// The open file's text cut into lines, and the colours of each: made once when it is read.
    public internal(set) var lines: [String] = []
    public internal(set) var spans: [[SyntaxSpan]] = []
    /// The open picture once decoded; nil while it is.
    var picture: FilePicture?
    @ObservationIgnored let backend: any FilesBackend
    @ObservationIgnored let highlighter: any SyntaxHighlighting
    @ObservationIgnored weak var host: (any PanelHost)?
    @ObservationIgnored private var watch: Task<Void, Never>?
    @ObservationIgnored var reading: Task<Void, Never>?
    /// A file is about to open in the viewer: the controller takes down what else covers the terminal.
    @ObservationIgnored var opened: () -> Void = {}

    public init(backend: any FilesBackend, highlighter: any SyntaxHighlighting = LexerHighlighter(), host: (any PanelHost)? = nil) {
        self.backend = backend
        self.highlighter = highlighter
        self.host = host
    }

    public var root: String? { chosen ?? terminalFolder }

    /// The walk's id of the path button.
    static let headID = "files-head"

    /// What belongs to one terminal and waits for it while another is shown: its tree with the folders opened, the
    /// row selected and a folder chosen by hand (GPUI's tree per session).
    private struct Kept {
        var tree: FileTree
        var selected: String?
        var chosen: String?
        var root: String?
    }
    @ObservationIgnored private var kept: [Int: Kept] = [:]
    @ObservationIgnored private var terminal: Int?

    /// The tree follows the terminal's folder; a folder chosen by hand stays until Follow Terminal. Another terminal
    /// brings its own tree and takes the viewer and a rename down.
    public func follow(_ folder: String?, terminal: Int? = nil) {
        if terminal != self.terminal {
            if let old = self.terminal { kept[old] = Kept(tree: tree, selected: selected, chosen: chosen, root: root) }
            let mine = terminal.flatMap { kept.removeValue(forKey: $0) }
            (self.terminal, terminalFolder, chosen) = (terminal, folder, mine?.chosen)
            (renaming, renameError) = (nil, nil)
            retarget()
            // Its folders are open again and read afresh, unless the terminal moved meanwhile.
            if let mine, mine.root == root {
                (tree, selected, rows) = (mine.tree, mine.selected, mine.tree.rows)
                reload()
            }
            return
        }
        let changed = terminalFolder != folder
        terminalFolder = folder
        if chosen == nil, changed { retarget() }
    }

    public func choose(_ folder: String) {
        chosen = folder
        retarget()
    }

    public func followTerminal() {
        chosen = nil
        retarget()
    }

    public func chooseFolder() {
        Task { if let folder = await host?.chooseFolder() { choose(folder) } }
    }

    private func retarget() {
        tree.clear()
        selected = nil
        closeViewer()
        rows = tree.rows
        watch?.cancel()
        Task { pins = await backend.pins() }
        guard let root else { return }
        reload()
        watch = Task { [backend, weak self] in
            for await _ in backend.changes(in: root) { self?.reload() }
        }
    }

    /// Reads the root and every open folder again, the ones above first.
    public func reload() {
        guard let root else { return }
        let backend = backend
        let wanted = tree.wanted
        Task {
            for folder in wanted {
                let answer = await backend.list(folder, in: root)
                guard root == self.root else { return }
                if tree.set(folder, answer) { rows = tree.rows }
            }
            rows = tree.rows
            if viewing != nil { readViewed() }
        }
    }

    /// A click: a folder opens or closes, a file shows in the viewer; every entry is selected.
    public func click(_ row: FileRow) {
        guard row.isEntry else { return }
        selected = row.path
        switch row.kind {
        case .file: view(row.path)
        case .folder: toggle(row.path)
        default: break
        }
    }

    public func toggle(_ path: String) {
        let needsReading = tree.toggle(path)
        rows = tree.rows
        if needsReading { reload() }
    }

    public func isPinned(_ path: String) -> Bool { root.map { pins.contains(join($0, path)) } ?? false }

    public func setPinned(_ path: String, _ on: Bool) {
        guard let root else { return }
        let full = path.isEmpty ? root : join(root, path)
        Task {
            await backend.setPinned(full, on)
            pins = await backend.pins()
        }
    }

    public var pathMenu: PathMenuModel? {
        root.map { PathMenuModel(shown: $0, volume: FileManager.default.displayName(atPath: "/"), pins: pins, chosen: chosen != nil, home: host?.homeFolder) }
    }

    func join(_ root: String, _ path: String) -> String { root == "/" ? "/" + path : root + "/" + path }
}
