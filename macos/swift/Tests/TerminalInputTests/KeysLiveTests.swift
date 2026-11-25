#if WITH_UNIFFI
import AppKit
import Testing
import TerminalCore
import UniFFISource

@testable import TerminalMetal

/// The bytes a program receives for keys (T07): Option as Meta by side, and the kitty keyboard protocol as far as the engine
/// has it and the view feeds it. `ByteBench` says how they are read.
@MainActor
@Suite(.serialized) struct KeysLiveTests {
    /// The device-dependent bits AppKit puts in a key's flags for the left and right Option keys.
    private static let left = NSEvent.ModifierFlags(rawValue: NSEvent.ModifierFlags.option.rawValue | 0x20)
    private static let right = NSEvent.ModifierFlags(rawValue: NSEvent.ModifierFlags.option.rawValue | 0x40)

    /// US layout: Option-B types ∫ (U+222B), which is what the system tells the view the key makes.
    private static let integral = [0xe2, 0x88, 0xab].map(UInt8.init)

    @Test(arguments: [
        (OptionAsMeta.off, true, false), (.off, false, false), (.left, true, true), (.left, false, false),
        (.right, true, false), (.right, false, true), (.both, true, true), (.both, false, true),
    ])
    func optionAsMetaFollowsTheSideOfTheKey(setting: OptionAsMeta, isLeft: Bool, meta: Bool) async throws {
        let bench = try ByteBench("")
        #expect(await bench.start())
        bench.live.view.optionAsMeta = setting
        bench.key("∫", base: "b", code: 11, flags: isLeft ? Self.left : Self.right)
        await bench.expect(ByteBench.hex(meta ? [0x1b, 0x62] : Self.integral), "\(setting) with the \(isLeft ? "left" : "right") Option key")
    }

    @Test func kittyDisambiguateKeepsTheLegacyFormOfTheKeysThatHaveOne() async throws {
        let bench = try ByteBench(#"\033[>1u\033[?u"#)
        #expect(await bench.start())
        await bench.expect(ByteBench.hex("\u{1b}[?1u"), "the engine took the flag and says so")
        bench.key("a", code: 0)
        bench.key("\r", code: 36)
        bench.key("\t", code: 48)
        bench.key("\u{7f}", code: 51)
        await bench.expect(ByteBench.hex([0x61, 0x0d, 0x09, 0x7f]), "text, Enter, Tab and Backspace stay as they were")
        bench.key("\u{1b}", code: 53)
        bench.key("\u{01}", base: "a", code: 0, flags: .control)
        bench.live.view.optionAsMeta = .both
        bench.key("å", base: "a", code: 0, flags: .option)
        bench.key("\u{F700}", code: 126, flags: .shift)
        bench.key("\u{F708}", code: 96)
        bench.key("\u{F710}", code: 105)
        await bench.expect(
            ByteBench.hex("\u{1b}[27u\u{1b}[97;5u\u{1b}[97;3u\u{1b}[1;2A\u{1b}[15~\u{1b}[57376u"),
            "Escape, Control-A, Option-A (Option as Meta) as codes with their modifiers; Shift-Up and F5 in their old forms; F13 in the private area")
    }

    @Test func kittyReportAllKeysSendsEveryKeyAsACodeAndAlternatesAndTextWhenAsked() async throws {
        let bench = try ByteBench(#"\033[>9u"#)
        #expect(await bench.start())
        bench.key("a", code: 0)
        bench.key("\r", code: 36)
        bench.key("\t", code: 48)
        bench.key("\u{7f}", code: 51)
        bench.key("A", base: "a", code: 0, flags: .shift)
        await bench.expect(ByteBench.hex("\u{1b}[97u\u{1b}[13u\u{1b}[9u\u{1b}[127u\u{1b}[97;2u"), "with report-all, text is a code too, and Enter, Tab, Backspace")
        // Flags 4 and 16 on top: the shifted key after the code, the text at the end.
        let more = try ByteBench(#"\033[>29u\033[?u"#)
        #expect(await more.start())
        await more.expect(ByteBench.hex("\u{1b}[?29u"))
        more.key("a", code: 0)
        more.key("A", base: "a", code: 0, flags: .shift)
        // The text is the third field; the second, the modifiers, is written as 1 (none) where kitty's own terminal leaves it empty:
        // both are the same report to a program that follows the protocol (the modifiers are 1 plus a bit field).
        await more.expect(ByteBench.hex("\u{1b}[97;1;97u\u{1b}[97:65;2;65u"))
    }

    @Test func kittyEventTypesAreAskedForButThePressIsAllThatIsSent() async throws {
        let bench = try ByteBench(#"\033[>11u\033[?u"#)
        #expect(await bench.start())
        await bench.expect(ByteBench.hex("\u{1b}[?11u"))
        bench.key("a", code: 0)
        let press = await bench.read()
        let up = NSEvent.keyEvent(
            with: .keyUp, location: .zero, modifierFlags: [], timestamp: 0, windowNumber: 0, context: nil, characters: "a",
            charactersIgnoringModifiers: "a", isARepeat: false, keyCode: 0)!
        bench.live.view.keyUp(with: up)
        let release = await bench.read()
        // Written down as it is: a press is `CSI 97 u` (a press is the type kitty leaves out) and the release goes nowhere.
        #expect(press == ByteBench.hex("\u{1b}[97u"), "the press")
        #expect(release == "", "no release report is sent: the view does not forward key-ups")
    }

    @Test func popAndQueryReadTheStackBack() async throws {
        let bench = try ByteBench(#"\033[>1u\033[>8u\033[?u\033[<1u\033[?u\033[<5u\033[?u"#)
        #expect(await bench.start())
        // Pushed 1, then 8 on top (the stack holds both, 8 now in force); popped one: 1 again; popped five more than there were: none.
        await bench.expect(ByteBench.hex("\u{1b}[?8u\u{1b}[?1u\u{1b}[?0u"))
    }
}
#endif
