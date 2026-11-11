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
}
