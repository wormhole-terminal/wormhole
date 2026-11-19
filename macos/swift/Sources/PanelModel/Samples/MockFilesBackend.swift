// A `FilesBackend` over a small virtual project, so the tree, the viewer, rename and pins work in previews and tests
// with no disk. A rename really moves the entry.

import Foundation

public actor MockFilesBackend: FilesBackend {
    /// Folder path (relative, "" the root) to its entries.
    var folders: [String: [DiskEntry]]
    var pinned: [String]
    var texts: [String: String]

    public init(pinned: [String] = []) {
        folders = Self.sampleFolders
        self.pinned = pinned
        texts = Self.sampleTexts
    }

    public func list(_ folder: String, in root: String) async -> Result<Directory, DirectoryFailure> {
        guard let entries = folders[folder] else { return .failure(.gone) }
        let order = { (entry: DiskEntry) in (entry.isFolder ? 0 : 1, entry.name.lowercased()) }
        return .success(Directory(entries: entries.sorted { order($0) < order($1) }))
    }

    public func read(_ path: String, in root: String) async -> FileContent {
        if path.hasSuffix(".png") { return .binary(size: 48_213) }
        let name = path.split(separator: "/").last.map(String.init) ?? path
        guard let text = texts[name] else { return .unavailable(.gone) }
        return .text(text, size: text.utf8.count, truncated: false)
    }

    public func rename(_ path: String, to name: String, in root: String) async -> Result<Void, RenameFailure> {
        guard !name.isEmpty, !name.contains("/") else { return .failure(.name) }
        let parent = path.lastIndex(of: "/").map { String(path[..<$0]) } ?? ""
        let old = path.split(separator: "/").last.map(String.init) ?? path
        guard var entries = folders[parent], let at = entries.firstIndex(where: { $0.name == old }) else { return .failure(.gone) }
        guard !entries.contains(where: { $0.name == name }) else { return .failure(.exists) }
        entries[at] = DiskEntry(name: name, kind: entries[at].kind)
        folders[parent] = entries
        return .success(())
    }

    public func resolve(_ path: String, in root: String, folder: Bool) async -> String? { root + "/" + path }

    public func pins() async -> [String] { pinned }

    public func setPinned(_ folder: String, _ on: Bool) async {
        pinned.removeAll { $0 == folder }
        if on { pinned.append(folder) }
    }

    public nonisolated func changes(in root: String) -> AsyncStream<Void> { AsyncStream { $0.finish() } }

    private static let sampleFolders: [String: [DiskEntry]] = [
        "": [
            DiskEntry(name: "crates", kind: .folder), DiskEntry(name: "docs", kind: .folder), DiskEntry(name: "scripts", kind: .folder),
            DiskEntry(name: "Cargo.toml", kind: .file), DiskEntry(name: "README.md", kind: .file), DiskEntry(name: ".gitignore", kind: .file),
            DiskEntry(name: "icon.png", kind: .file), DiskEntry(name: "latest", kind: .link),
        ],
        "crates": [DiskEntry(name: "wormhole", kind: .folder), DiskEntry(name: "wormhole-core", kind: .folder)],
        "crates/wormhole": [DiskEntry(name: "src", kind: .folder), DiskEntry(name: "Cargo.toml", kind: .file)],
        "crates/wormhole/src": [DiskEntry(name: "main.rs", kind: .file), DiskEntry(name: "sidebar", kind: .folder), DiskEntry(name: "lib.rs", kind: .file)],
        "crates/wormhole/src/sidebar": [],
        "crates/wormhole-core": [DiskEntry(name: "src", kind: .folder)],
        "crates/wormhole-core/src": [DiskEntry(name: "lib.rs", kind: .file), DiskEntry(name: "view", kind: .folder)],
        "crates/wormhole-core/src/view": [DiskEntry(name: "mod.rs", kind: .file)],
        "docs": [DiskEntry(name: "ARCHITECTURE.md", kind: .file), DiskEntry(name: "notes-panel.md", kind: .file)],
        "scripts": [DiskEntry(name: "check.sh", kind: .file), DiskEntry(name: "dev.sh", kind: .file)],
    ]

    private static let sampleTexts: [String: String] = [
        "main.rs": """
            //! The app's entry point.

            use std::process::ExitCode;

            /// Starts the app; the exit code is what the shell sees.
            fn main() -> ExitCode {
                let args: Vec<String> = std::env::args().skip(1).collect();
                /* A multi-line comment,
                   still a comment here. */
                match wormhole::run(&args) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(err) => {
                        eprintln!("wormhole: {err:#}");
                        ExitCode::from(2)
                    }
                }
            }
            """,
        "Cargo.toml": "[package]\nname = \"wormhole\"\nversion = \"1.6.3\"\nedition = \"2024\"\n\n[dependencies]\nanyhow = \"1\"\n",
        "README.md": "# Wormhole\n\nA native macOS terminal.\n",
    ]
}
