// One commit of the History tab, with the files it changed once they are read.

import Foundation

public struct Commit: Identifiable, Hashable, Sendable {
    public var hash: String
    public var subject: String
    public var author: String
    public var date: Date
    public var files: [ChangedFile]

    public var id: String { hash }
    public var shortHash: String { String(hash.prefix(7)) }

    public init(hash: String, subject: String, author: String, date: Date, files: [ChangedFile] = []) {
        self.hash = hash
        self.subject = subject
        self.author = author
        self.date = date
        self.files = files
    }
}

public struct HistoryModel: Equatable, Sendable {
    public var commits: [Commit]
    public init(commits: [Commit] = []) { self.commits = commits }
}
