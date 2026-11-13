// The palette of the profile Wormhole is in, told to the engine (`set_terminal_colors`): what a terminal starts with and
// answers programs' colour queries with (OSC 4, 10, 11, 12, `CSI ? 996 n`, mode 2031) until its view tells it the colours
// it is drawn in (`MetalTerminalView.setPalette`, `Terminal.set_colors`), which a window of another profile needs.

#if WITH_UNIFFI
    import TerminalCore
    import WormholeBindings

    enum EngineColors {
        static func share(_ palette: TerminalPalette) {
            let slots = palette.slots.prefix(Int(PackedColor.cursor) + 1)
            setTerminalColors(slots: slots.map { UInt32($0.x) << 16 | UInt32($0.y) << 8 | UInt32($0.z) })
        }
    }
#endif
