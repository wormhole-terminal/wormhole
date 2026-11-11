// The palette the terminals are drawn with, told to the engine (`set_terminal_colors`): it answers programs' colour
// queries (OSC 4, 10, 11, 12) and `CSI ? 996 n` with it, and tells the programs that asked (mode 2031) when the theme
// turns light or dark. Every live terminal hears at once, and the ones started later get it at spawn.

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
