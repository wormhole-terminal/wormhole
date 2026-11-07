// The FFI's records as the panel's values. Words that core owns (a submodule's label, a commit's cleaned message)
// arrive finished; this only moves fields.

import PanelModel
import WormholeBindings

extension ChangedFile {
    init(_ change: GitChange) {
        self.init(
            path: change.path, kind: ChangeKind(change.kind), added: change.added.map(Int.init),
            removed: change.removed.map(Int.init), originalPath: change.originalPath, lfs: change.lfs,
            submodule: change.submodule)
    }
}

extension ChangeKind {
    init(_ kind: GitChangeKind) {
        switch kind {
        case .modified: self = .modified
        case .added: self = .added
        case .deleted: self = .deleted
        case .renamed: self = .renamed
        case .untracked: self = .untracked
        case .conflicted: self = .conflicted
        }
    }
}

extension Rev {
    init(_ rev: GitRev) {
        let kind: Kind =
            switch rev.kind {
            case .branch: .branch
            case .tag: .tag
            case .commit: .commit
            case .stashes: .stashes
            }
        self.init(name: rev.name, kind: kind, detail: rev.detail)
    }
}

extension GitRev {
    init(_ rev: Rev) {
        let kind: GitRevKind =
            switch rev.kind {
            case .branch: .branch
            case .tag: .tag
            case .commit: .commit
            case .stashes: .stashes
            }
        self.init(name: rev.name, kind: kind, detail: rev.detail)
    }
}

extension Signature {
    init(_ signature: GitSignature) {
        self.init(name: signature.name, email: signature.email, time: Int(signature.time), zone: Int(signature.zone))
    }
}

/// Core names remote-tracking branches apart; the panel draws them as branches.
private func panelRefs(_ refs: [GitRef]) -> [(kind: RefKind, name: String)] {
    refs.map { found in
        switch found.kind {
        case .head: (.head, found.name)
        case .branch, .remote: (.branch, found.name)
        case .tag: (.tag, found.name)
        }
    }
}

extension Commit {
    /// A row of the log: the subject and the first parent; the rest comes with `details`.
    init(_ row: GitLogCommit) {
        self.init(
            sha: row.sha, subject: row.subject, author: Signature(name: row.author, time: Int(row.time)),
            parents: row.parent.map { [$0] } ?? [], refs: panelRefs(row.refs), merge: row.merge)
    }

    init(_ details: GitCommitDetails) {
        self.init(
            sha: details.sha, subject: details.subject, body: details.body, author: Signature(details.author),
            parents: details.parents, refs: panelRefs(details.refs), files: details.files.map(ChangedFile.init))
    }
}

/// What one status says besides the files: the title row's branch and how far it is from its upstream.
public struct GitRepoState: Sendable, Equatable {
    public var files: [ChangedFile]
    public var branch: String?
    /// The tree has uncommitted changes, whatever the list is compared with.
    public var dirty: Bool
    public var upstream: String?
    public var ahead: Int
    public var behind: Int
    /// The compared branch no longer exists: the files are against HEAD and the panel says so.
    public var compareMissing: String?

    init(_ status: GitStatus) {
        files = status.files.map(ChangedFile.init)
        (branch, dirty, upstream) = (status.branch, status.dirty, status.upstream)
        (ahead, behind) = (Int(status.ahead), Int(status.behind))
        compareMissing = status.compareMissing
    }
}
