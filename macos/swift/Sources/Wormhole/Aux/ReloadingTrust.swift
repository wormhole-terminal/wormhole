// The project commands window approves through a desk of its own, so the Project tab, which follows the project's files and not
// the trust store, would go on saying "not trusted": this tells it to read again once an approval is done.

import AuxModel

@MainActor
final class ReloadingTrust: TrustBackend {
    private let base: any TrustBackend
    private let reload: @MainActor () -> Void

    init(_ base: any TrustBackend, reload: @escaping @MainActor () -> Void) { (self.base, self.reload) = (base, reload) }

    var biometricsAvailable: Bool { base.biometricsAvailable }
    func review() -> TrustReview { base.review() }
    func changes() -> AsyncStream<Void> { base.changes() }

    func approve() async {
        await base.approve()
        reload()
    }
}
