// Tips closed for good ("Don't show again" on the sidebar's note): kept in the data folder's database by the core
// (`onboarding::DismissedTips`), brought back by Settings → Reset tips.

#if WITH_UNIFFI
    import WormholeBindings
#endif

extension AppController {
    func dismissAgentsTip() {
        #if WITH_UNIFFI
            try? dismissNoAgentsTip()
        #endif
    }

    /// A first run that ended on the primer puts up the sidebar's "no agents" note, unless it was closed for good
    /// (`onboarding/mod.rs` `primer`; the terminal welcome's ending does the same in its own window).
    func showNoAgentsTip() {
        #if WITH_UNIFFI
            guard !((try? noAgentsTipDismissed()) ?? false) else { return }
        #endif
        hub.agentsTip = .note
    }
}
