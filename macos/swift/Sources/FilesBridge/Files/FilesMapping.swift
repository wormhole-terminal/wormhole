// The Files tab's records from Rust as `PanelModel`'s values.

import PanelModel
import WormholeBindings

extension Directory {
    init(_ record: DirectoryRecord) {
        self.init(
            entries: record.entries.map { DiskEntry(name: $0.name, label: $0.label, kind: DiskEntry.Kind($0.kind)) },
            more: Int(record.more), atLeast: record.atLeast)
    }
}

extension DiskEntry.Kind {
    init(_ kind: FileEntryKind) {
        switch kind {
        case .folder: self = .folder
        case .file: self = .file
        case .link: self = .link
        }
    }
}

extension DirectoryFailure {
    init(_ error: Error) {
        switch error as? FilesFailure {
        case .Gone: self = .gone
        case .Denied: self = .denied
        default: self = .other
        }
    }
}

extension PanelModel.RenameFailure {
    init(_ error: Error) {
        switch error as? WormholeBindings.RenameFailure {
        case .Name: self = .name
        case .Exists: self = .exists
        case .Denied: self = .denied
        case .Gone: self = .gone
        default: self = .other
        }
    }
}

extension FileContent {
    init(_ record: FileContentRecord) {
        switch record {
        case let .text(text, size, truncated): self = .text(text, size: Int(size), truncated: truncated)
        case let .binary(size): self = .binary(size: Int(size))
        case let .image(bytes, size): self = .image(bytes, size: Int(size))
        case let .unavailable(problem): self = .unavailable(Problem(problem))
        }
    }
}

extension FileContent.Problem {
    init(_ problem: FileProblem) {
        switch problem {
        case .gone: self = .gone
        case .outside: self = .outside
        case .notAFile: self = .notAFile
        case .denied: self = .denied
        case .other: self = .other
        }
    }
}
