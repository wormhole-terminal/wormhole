// Tells the core which app this is, once at launch and before any window reads the settings: the Settings rows that only
// the Swift app paints (`one_background`) are hidden from the GPUI app by this fact, and shown here (`front_end.rs` of the
// bridge, `platform::front_end` of the core).

#if WITH_UNIFFI
    import WormholeBindings
#endif

enum LaunchFrontEnd {
    static func start() {
        #if WITH_UNIFFI
            frontEndStart()
        #endif
    }
}
