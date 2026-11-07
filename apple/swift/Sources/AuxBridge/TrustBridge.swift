// `TrustBackend` over core's project trust (`TrustDesk`): what the `.wormhole/` folder would run, in core's words,
// the profile's approvals, and the approval itself. The owner is asked first, through the platform's
// `Authenticating` (never raised in tests: they pass a scripted one); only an answer of yes reaches Rust, and it
// trusts exactly what the last `review()` showed.

import AuxModel
import Foundation
import PlatformModel
import WormholeBindings

@MainActor
public final class FFITrustBackend: TrustBackend {
    private let desk: TrustDesk
    private let hub = WakeHub()
    private let authenticator: any Authenticating
    private var approving = false

    /// `root`: the project's folder; `profile`: whose approval it is (the default one when `nil`).
    public init(root: URL, profile: String? = nil, dataDir: URL? = nil, authenticator: any Authenticating) throws {
        desk = try TrustDesk(root: root.path, profile: profile, dataDir: dataDir?.path, listener: hub)
        self.authenticator = authenticator
    }

    public var biometricsAvailable: Bool { authenticator.biometricsAvailable() }

    public func review() -> TrustReview {
        let record = desk.review()
        return TrustReview(
            name: record.name, directory: record.directory, status: record.status.value,
            lines: record.lines.map { TrustLine(what: $0.what, runs: $0.runs, note: $0.note) }, problem: record.problem,
            approving: approving)
    }

    public func changes() -> AsyncStream<Void> {
        let desk = desk
        _ = desk.follow()
        return hub.stream(onFinish: { desk.unfollow() })
    }

    public func approve() async {
        guard !approving else { return }
        approving = true
        defer { approving = false }
        // The window shows what is about to be approved; the approval is for exactly that.
        let shown = review()
        guard shown.canApprove else { return }
        let reason = "trust the project folder \u{201C}\(shown.name)\u{201D} and let it run commands"
        do {
            try await authenticator.authenticate(reason: reason)
            try desk.approveReviewed()
        } catch {
            // Cancelled or refused: nothing is trusted, and the window still shows the project as it was.
        }
    }

    public func revoke() throws { try desk.revoke() }
}

extension ReviewStatusRecord {
    var value: TrustStatus {
        switch self {
        case .trusted: .trusted
        case .changed: .changed
        case .untrusted: .untrusted
        }
    }
}
