// How long a terminal has been in its state, in the words the core's status bar uses ("<1m", "4m", "1h 5m"), and when
// they next read differently: the one timer's due time (the words move with time, so they are never read off a cache).

public struct Waited: Sendable, Equatable {
    public var words: String
    public var next: Duration

    public init(words: String, next: Duration) { (self.words, self.next) = (words, next) }
}
