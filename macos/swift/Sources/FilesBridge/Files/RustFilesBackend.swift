// `FilesBackend` over `wormhole-core`: capped, link-safe reads, `rename_within`, the pins in `config.json`, and the
// folder watch (FSEvents, local volumes; a slow poll where there is none). All of it decided in Rust: a path
// that leaves the root or goes through a link is refused there.

import Foundation
import PanelModel
import WormholeBindings

public final class RustFilesBackend: FilesBackend, @unchecked Sendable {
    private let service: FilesService

    public init(service: FilesService) { self.service = service }

    public func list(_ folder: String, in root: String) async -> Result<Directory, DirectoryFailure> {
        let service = service
        return await offMain {
            do { return .success(Directory(try service.list(root: root, folder: folder))) } catch { return .failure(DirectoryFailure(error)) }
        }
    }

    public func read(_ path: String, in root: String) async -> FileContent {
        let service = service
        return await offMain { FileContent(service.read(root: root, path: path)) }
    }

    public func rename(_ path: String, to name: String, in root: String) async -> Result<Void, PanelModel.RenameFailure> {
        let service = service
        return await offMain {
            do { _ = try service.rename(root: root, path: path, name: name); return .success(()) } catch { return .failure(PanelModel.RenameFailure(error)) }
        }
    }

    public func pins() async -> [String] {
        let service = service
        return await offMain { service.pins() }
    }

    public func setPinned(_ folder: String, _ pinned: Bool) async {
        let service = service
        await offMain { service.setPinned(folder: folder, pinned: pinned) }
    }

    public func changes(in root: String) -> AsyncStream<Void> {
        let service = service
        return WakeStream.make { relay in
            let watch = service.watch(root: root, listener: relay)
            return WatchHandle(drain: { watch.drain() }, stop: { watch.stop() })
        }
    }
}
