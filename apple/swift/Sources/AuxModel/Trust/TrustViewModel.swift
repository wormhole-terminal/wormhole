// The trust window's state: the project as the hub holds it now. It follows the backend's changes, so a `git pull`
// while the window is open shows what would run now, and approving goes through the backend (Touch ID).

import Foundation
import Observation

@MainActor @Observable
public final class TrustViewModel {
    public private(set) var review: TrustReview
    @ObservationIgnored public let backend: any TrustBackend
    @ObservationIgnored private var follow: Task<Void, Never>?

    public init(backend: any TrustBackend) {
        self.backend = backend
        review = backend.review()
    }

    public func start() {
        follow?.cancel()
        let changes = backend.changes()
        follow = Task { [weak self] in
            for await _ in changes { self?.reload() }
        }
    }

    public func stop() { follow?.cancel() }

    public func reload() { review = backend.review() }

    /// The button's words: waiting while the dialog is up, else Touch ID or the password where there is none.
    public var approveLabel: String {
        review.approving ? AuxText.trustWaiting : backend.biometricsAvailable ? AuxText.trustWithTouchID : AuxText.trustWithPassword
    }

    public func approve() {
        Task {
            await backend.approve()
            reload()
        }
    }
}
