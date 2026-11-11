// The opened commit's block as the core made it (`GitCommitDetails`, which mirrors the GPUI app's `Shown`): the words,
// the labelled facts with their tinted stretches, the capped lists and what a copy puts on the clipboard. The view
// lays out what is here and decides nothing about it.

public struct CommitDetails: Hashable, Sendable {
    public enum Tone: Sendable { case plain, muted, added, removed }

    public struct Span: Hashable, Sendable {
        public var text: String
        public var tone: Tone
        public init(_ text: String, _ tone: Tone = .plain) { (self.text, self.tone) = (text, tone) }
    }

    /// A labelled line of the block ("Author", "Changes"): its text as stretches in the colours they are drawn in.
    public struct Fact: Hashable, Sendable {
        public var label: String
        public var spans: [Span]
        public init(_ label: String, _ spans: [Span]) { (self.label, self.spans) = (label, spans) }
    }

    /// A parent: its full hash, the short one it is drawn as and what VoiceOver calls the link.
    public struct Parent: Hashable, Sendable {
        public var sha: String
        public var short: String
        public var spoken: String
        public init(sha: String, short: String, spoken: String) { (self.sha, self.short, self.spoken) = (sha, short, spoken) }
    }

    public var sha: String
    public var subject: String
    public var spoken: String
    /// The whole message and `Name <email>`: what the copies put on the clipboard.
    public var message: String
    public var authorLine: String
    public var people: [Fact]
    public var parentsLabel: String
    public var parents: [Parent]
    public var parentsMore: String?
    public var refs: [(kind: RefKind, name: String)]
    public var refsMore: String?
    /// The body's prose and its trailers, cleaned for the screen: what the whole view draws.
    public var whole: String
    public var trailers: String
    /// The lines that say the body goes on, or that the message was longer than is read.
    public var continues: String?
    public var tooLong: String?
    public var changes: Fact
    public var files: [ChangedFile]

    public init(
        sha: String, subject: String, spoken: String, message: String, authorLine: String, people: [Fact], parentsLabel: String,
        parents: [Parent], parentsMore: String? = nil, refs: [(kind: RefKind, name: String)] = [], refsMore: String? = nil,
        whole: String = "", trailers: String = "", continues: String? = nil, tooLong: String? = nil, changes: Fact, files: [ChangedFile] = []
    ) {
        (self.sha, self.subject, self.spoken, self.message, self.authorLine) = (sha, subject, spoken, message, authorLine)
        (self.people, self.parentsLabel, self.parents, self.parentsMore) = (people, parentsLabel, parents, parentsMore)
        (self.refs, self.refsMore, self.whole, self.trailers) = (refs, refsMore, whole, trailers)
        (self.continues, self.tooLong, self.changes, self.files) = (continues, tooLong, changes, files)
    }

    public static func == (lhs: CommitDetails, rhs: CommitDetails) -> Bool { lhs.sha == rhs.sha && lhs.files == rhs.files && lhs.message == rhs.message }
    public func hash(into hasher: inout Hasher) { hasher.combine(sha) }

    /// The message as far as it is shown compact, and whether something is left out: the core's estimate for lines of
    /// about this many characters.
    public struct Compact: Sendable, Equatable {
        public var text: String
        public var more: Bool
        public init(text: String, more: Bool) { (self.text, self.more) = (text, more) }
    }
}
