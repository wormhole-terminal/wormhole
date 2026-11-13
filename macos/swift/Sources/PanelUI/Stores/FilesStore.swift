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
    /// A rename is on its way to the disk: the field neither commits again nor closes until it is back.
    @ObservationIgnored var renameBusy = false
    /// The open file's text cut into lines, and the colours of each: made once when it is read.
    public internal(set) var lines: [String] = []
    public internal(set) var spans: [[SyntaxSpan]] = []
    /// The open picture once decoded; nil while it is.
    var picture: FilePicture?
    @ObservationIgnored let backend: any FilesBackend
    @ObservationIgnored let highlighter: any SyntaxHighlighting
    @ObservationIgnored weak var host: (any PanelHost)?
    @ObservationIgnored private var watch: Task<Void, Never>?
    @ObservationIgnored private(set) var isShown = true
    @ObservationIgnored var reading: Task<Void, Never>?
    /// A pass over the folders is running, and something asked for another meanwhile.
    @ObservationIgnored private var reloading = false
    @ObservationIgnored private var reloadAgain = false
    /// What each folder looked like from outside when the tree last listed it, and the open file when it was last
    /// read (`FilesBackend.stamp`); nil where the backend cannot say.
    @ObservationIgnored private var stamps: [String: String] = [:]
    @ObservationIgnored var viewedStamp: String?
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

    /// The window's terminals now: what waited for one that is gone (closed, or moved to another window) goes with it.
    public func keep(terminals: Set<Int>) { kept = kept.filter { terminals.contains($0.key) } }

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
        stamps = [:]
        selected = nil
        closeViewer()
        rows = tree.rows
        stopReading()
        startReading()
    }

    /// Whether the tab is on screen (the window says; true where nobody does: previews, tests). Hidden, or with the
    /// feature off, nothing is listed or watched (`features/files/read.rs`), but for a file left open over the terminal.
    public func setShown(_ on: Bool) {
        guard on != isShown else { return }
        isShown = on
        if on { startReading() } else if viewing == nil { stopReading() }
    }

    private func startReading() {
        guard isShown, watch == nil, let root else { return }
        Task { pins = await backend.pins() }
        reload()
        watch = Task { [backend, weak self] in
            for await _ in backend.changes(in: root) { self?.reload() }
        }
    }

    func stopReading() {
        watch?.cancel()
        watch = nil
    }

    /// Reads the root and every open folder again, the ones above first. One pass at a time whatever asks (the watch,
    /// a folder opened, a rename): what is asked meanwhile is one more pass after it (`read.rs` `in_flight`). The open
    /// file is read again only when it looks different from outside.
    public func reload() {
        guard let root, isShown || viewing != nil else { return }
        guard !reloading else { reloadAgain = true; return }
        reloading = true
        let backend = backend
        let wanted = tree.wanted
        Task {
            for folder in wanted {
                // Before the listing, so a change during it shows as one next time. A folder that looks as it did when
                // the tree last read it is not listed again.
                let stamp = await backend.stamp(folder, in: root)
                guard root == self.root else { break }
                if let stamp, stamps[folder] == stamp, tree.isRead(folder) { continue }
                let answer = await backend.list(folder, in: root)
                guard root == self.root else { break }
                stamps[folder] = stamp
                if tree.set(folder, answer) { rows = tree.rows }
            }
            reloading = false
            let again = reloadAgain
            reloadAgain = false
            guard root == self.root else { return reload() }
            rows = tree.rows
            if viewing != nil { readViewed(onlyIfChanged: true) }
            if again { reload() }
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
