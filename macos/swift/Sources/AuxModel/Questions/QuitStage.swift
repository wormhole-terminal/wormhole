// A quit that is under way, and what a second request to quit gets while it is: the question on the screen, or the
// session hosts ending, answers for both. One question, one End All, however often quitting is asked for (a second ⌘Q,
// the Dock's Quit, the last window's close). The GPUI app's `quit_asking` does the same (`window/windows.rs:438-446`):
// nothing while it is quitting, and a second ⌘Q while the question is up leaves it up.

/// How far a quit has got.
public enum QuitStage: Equatable, Sendable {
    /// Its question is on the screen.
    case asking
    /// Quit and End All: the session hosts end, off the main thread.
    case endingHosts
    /// They ended: the quit goes on.
    case hostsEnded

    /// What a request to quit gets.
    public enum Reply: Equatable, Sendable {
        /// Nothing stands before it: it is decided as a first request is.
        case decide
        /// The hosts ended: it goes.
        case now
        /// The answer that is on its way goes for this request too.
        case later
        /// Dropped: the question on the screen is the one the person answers.
        case absorbed
    }

    /// The reply to a request made during `stage`. `asks` is whether the request would put the question: a quit that
    /// runs unattended (`kill`, the control folder, the idle check, an update's restart) asks nothing, and no question
    /// on the screen holds it up.
    public static func reply(during stage: QuitStage?, asks: Bool) -> Reply {
        switch stage {
        case nil: .decide
        case .asking?: asks ? .absorbed : .decide
        case .endingHosts?: .later
        case .hostsEnded?: .now
        }
    }
}
