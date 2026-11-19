// Reply, from a banner (`NoticeFeed`) or the quick panel (`run(ambient:)`): the one place both go through. The answer
// goes to the terminal's workspace in the core, which trims it, sends nothing when nothing is left, pastes it the way the
// program takes a paste and presses Return 60 ms later, "the program reads the paste first"
// (`notifications/respond.rs:82-85`, `window/control/terminals.rs:231-239`, `window/feature_terminals.rs:19-33`). The core
// holds every terminal that runs, so one no view was made for yet (restored and not shown since, in a window out of
// sight) is answered too, in whichever window it is, and nothing comes forward.

#if WITH_UNIFFI
    import WorkspaceBridge
    import WormholeBindings
#endif

extension AppController {
    func typeReply(_ text: String, into tab: Int64) {
        #if WITH_UNIFFI
            guard let core = (session(holding: Int(tab))?.hub.store.hooks as? RustWorkspaceBackend)?.core else { return }
            _ = core.typeReply(tab: tab, text: text)
        #endif
    }
}
