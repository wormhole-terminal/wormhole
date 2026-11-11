// The owner's authentication (Touch ID, an Apple Watch, the password) for Rust's `approve`: Rust asks, on the
// background thread that called it, and waits. The window's `Authenticating` is async, so this blocks that
// thread, never the main actor, until it answers. A test instance answers from WORMHOLE_AUTH without a prompt.

import Dispatch
import PlatformModel
import WormholeBindings

public final class OwnerAuthentication: OwnerAuthenticator, @unchecked Sendable {
    private let authenticator: any Authenticating

    public init(_ authenticator: any Authenticating) { self.authenticator = authenticator }

    public func authenticate(reason: String) -> OwnerAnswer {
        let outcome = Outcome()
        let done = DispatchSemaphore(value: 0)
        let authenticator = authenticator
        Task.detached {
            do {
                try await authenticator.authenticate(reason: reason)
                outcome.set(.approved)
            } catch AuthError.cancelled {
                outcome.set(.cancelled)
            } catch {
                outcome.set(.failed(message: "\(error)"))
            }
            done.signal()
        }
        done.wait()
        return outcome.answer
    }

    private final class Outcome: @unchecked Sendable {
        private(set) var answer = OwnerAnswer.cancelled
        func set(_ answer: OwnerAnswer) { self.answer = answer }
    }
}
