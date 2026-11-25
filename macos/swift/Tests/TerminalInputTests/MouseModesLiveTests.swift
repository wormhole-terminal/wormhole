#if WITH_UNIFFI
import AppKit
import Testing
import TerminalCore
import UniFFISource

@testable import TerminalMetal

/// The bytes a program receives for the pointer, in every reporting mode the engine has (T05); `ByteBench` says how they are read.
@MainActor
@Suite(.serialized) struct MouseModesLiveTests {
    private static let esc = "\u{1b}"

    @Test func x10ModeReportsThePressOfTheThreeButtonsAndNothingElse() async throws {
        let bench = try ByteBench(#"\033[?9h"#)
        #expect(await bench.start())
        // Cell row 3, column 2 is 3 and 4 as numbers (1-based), each written as the number plus 32.
        bench.click(row: 3, column: 2)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x20, 0x23, 0x24]), "the left press only: no release in mode 9")
        bench.click(.middle, row: 3, column: 2)
        bench.click(.right, row: 3, column: 2)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x21, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x22, 0x23, 0x24]), "middle and right")
        // Mode 9 has no modifiers, no wheel, no motion, no drag.
        bench.press(.left, row: 3, column: 2, flags: .option)
        bench.drag(to: 4, 5)
        bench.release(.left, row: 4, column: 5)
        bench.move(to: 5, 6)
        try bench.wheel(3, row: 3, column: 2)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x20, 0x23, 0x24]), "an Option press is a press without its modifier; the rest nothing")
    }

    @Test func normalModeReportsPressAndReleaseModifiersAndTheWheelInTheOldForm() async throws {
        let bench = try ByteBench(#"\033[?1000h"#)
        #expect(await bench.start())
        bench.click(row: 3, column: 2)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x20, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x23, 0x23, 0x24]), "press, then release as button 3")
        bench.click(.right, row: 0, column: 0)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x22, 0x21, 0x21, 0x1b, 0x5b, 0x4d, 0x23, 0x21, 0x21]), "the right button at the first cell")
        // Alt adds 8, Control 16 (control-click is the menu's on the Mac, so Control goes with the right button).
        bench.click(row: 3, column: 2, flags: .option)
        bench.click(.right, row: 3, column: 2, flags: .control)
        await bench.expect(
            ByteBench.hex([0x1b, 0x5b, 0x4d, 0x28, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x2b, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x32, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x33, 0x23, 0x24]),
            "Option: 32+8, released 3+8 then; Control: 2+16")
        // The wheel is button 64 (up) and 65 (down), pressed once a line, never released.
        try bench.wheel(2, row: 3, column: 2)
        try bench.wheel(-1, row: 3, column: 2)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x60, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x60, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x61, 0x23, 0x24]), "two lines up, one down")
        // No drag report, no hover report at this level.
        bench.press(.left, row: 3, column: 2)
        bench.drag(to: 4, 5)
        bench.move(to: 5, 6)
        bench.release(.left, row: 4, column: 5)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x20, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x23, 0x26, 0x25]), "press where it began, release where it ended, nothing between")
    }

    @Test func sgrReportsBeyondTheOldLimitsWithTheButtonOfTheRelease() async throws {
        let bench = try ByteBench(#"\033[?1000h\033[?1006h"#)
        #expect(await bench.start())
        bench.click(row: 3, column: 2)
        await bench.expect(ByteBench.hex("\u{1b}[<0;3;4M\u{1b}[<0;3;4m"))
        bench.click(.middle, row: 0, column: 0)
        bench.click(.right, row: 19, column: 119)
        await bench.expect(ByteBench.hex("\u{1b}[<1;1;1M\u{1b}[<1;1;1m\u{1b}[<2;120;20M\u{1b}[<2;120;20m"), "the middle at the first cell, the right at the last, released by their own buttons")
        bench.click(row: 3, column: 2, flags: [.option])
        bench.click(.right, row: 3, column: 2, flags: [.control])
        await bench.expect(ByteBench.hex("\u{1b}[<8;3;4M\u{1b}[<8;3;4m\u{1b}[<18;3;4M\u{1b}[<18;3;4m"), "Option 8, Control 16")
        try bench.wheel(1, row: 3, column: 2)
        try bench.wheel(-1, row: 3, column: 2)
        await bench.expect(ByteBench.hex("\u{1b}[<64;3;4M\u{1b}[<65;3;4M"))
    }

    @Test func anyEventModeReportsTheHoverToo() async throws {
        let bench = try ByteBench(#"\033[?1003h\033[?1006h"#)
        #expect(await bench.start())
        bench.move(to: 3, 2)
        bench.move(to: 3, 2)
        bench.move(to: 3, 3)
        // 35 is motion with no button: 3 + 32.
        await bench.expect(ByteBench.hex("\u{1b}[<35;3;4M\u{1b}[<35;4;4M"), "each new cell once")
        bench.press(.left, row: 3, column: 3)
        bench.drag(to: 3, 4)
        bench.release(.left, row: 3, column: 4)
        await bench.expect(ByteBench.hex("\u{1b}[<0;4;4M\u{1b}[<32;5;4M\u{1b}[<0;5;4m"))
    }

    @Test func utf8EncodingWritesAColumnPastTheOldLimitAsTwoBytes() async throws {
        let bench = try ByteBench(#"\033[?1000h\033[?1005h"#, cols: 200)
        #expect(await bench.start())
        bench.click(row: 3, column: 2)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x20, 0x23, 0x24, 0x1b, 0x5b, 0x4d, 0x23, 0x23, 0x24]), "small positions are the old bytes")
        // Column 150 (1-based 151) + 32 = 183 = U+00B7: C2 B7.
        bench.click(row: 3, column: 150)
        await bench.expect(ByteBench.hex([0x1b, 0x5b, 0x4d, 0x20, 0xc2, 0xb7, 0x24, 0x1b, 0x5b, 0x4d, 0x23, 0xc2, 0xb7, 0x24]))
    }

    @Test func urxvtEncodingWritesDecimalNumbersWithTheButtonPlus32() async throws {
        let bench = try ByteBench(#"\033[?1000h\033[?1015h"#)
        #expect(await bench.start())
        bench.click(row: 3, column: 2)
        try bench.wheel(1, row: 3, column: 2)
        // Press 0+32, release 3+32, the wheel up 64+32; the release cannot name its button.
        await bench.expect(ByteBench.hex("\u{1b}[32;3;4M\u{1b}[35;3;4M\u{1b}[96;3;4M"))
    }

    @Test func pixelEncodingWritesPointsFromTheTextAreasCornerStartingAtOne() async throws {
        let bench = try ByteBench(#"\033[?1000h\033[?1016h"#)
        #expect(await bench.start())
        let live = bench.live
        let point = live.point(row: 3, column: 2, half: 0.25)
        bench.live.view.mouseDown(with: live.mouse(.leftMouseDown, point))
        bench.live.view.mouseUp(with: live.mouse(.leftMouseUp, point))
        let x = Int(point.x - MetalRenderer.padding) + 1, y = Int(point.y - MetalRenderer.padding) + 1
        await bench.expect(ByteBench.hex("\u{1b}[<0;\(x);\(y)M\u{1b}[<0;\(x);\(y)m"), "x \(x), y \(y)")
    }

    @Test func shiftTakesThePointerBackForASelection() async throws {
        let bench = try ByteBench(#"\033[?1000h\033[?1006h"#)
        #expect(await bench.start())
        bench.live.view.mouseDown(with: bench.down(.leftMouseDown, row: 0, column: 0, half: 0.25, flags: .shift))
        bench.live.view.mouseDragged(with: bench.down(.leftMouseDragged, row: 0, column: 4, half: 0.75, flags: .shift))
        bench.live.view.mouseUp(with: bench.down(.leftMouseUp, row: 0, column: 4, half: 0.75, flags: .shift))
        bench.live.view.drawFrame(force: true)
        #expect(bench.live.view.canCopy, "Shift-drag selected the text, as in a terminal that does not report")
        await bench.expect("", "and the program heard nothing")
        // The wheel under Shift goes to the history, not to the program.
        try bench.wheel(1, row: 3, column: 2, flags: .shift)
        await bench.expect("")
        // Without the shift it is the program's again.
        bench.click(row: 3, column: 2)
        await bench.expect(ByteBench.hex("\u{1b}[<0;3;4M\u{1b}[<0;3;4m"))
    }

    @Test func nothingIsReportedWhileNoModeIsOnAndTheModeEndsWhenTheProgramSaysSo() async throws {
        let bench = try ByteBench("")
        #expect(await bench.start())
        bench.click(row: 3, column: 2)
        try bench.wheel(1, row: 3, column: 2)
        await bench.expect("", "a program that did not ask hears nothing")
    }

    @Test func theWheelOnTheAlternateScreenIsTheCursorKeysUnlessTheProgramHasTheMouseOrTurnedItOff() async throws {
        // Alternate screen with no mouse mode: one arrow a line, in the form the cursor keys are in (DECCKM).
        let plain = try ByteBench(#"\033[?1049h"#)
        #expect(await plain.start())
        try plain.wheel(2, row: 3, column: 2)
        try plain.wheel(-1, row: 3, column: 2)
        await plain.expect(ByteBench.hex("\u{1b}[A\u{1b}[A\u{1b}[B"), "two lines up, one down")
        let application = try ByteBench(#"\033[?1049h\033[?1h"#)
        #expect(await application.start())
        try application.wheel(1, row: 3, column: 2)
        await application.expect(ByteBench.hex("\u{1b}OA"), "application cursor keys")
        // Under Shift, with alternate scroll switched off (1007), or on the main screen, the wheel is the history's: no bytes.
        try application.wheel(1, row: 3, column: 2, flags: .shift)
        await application.expect("")
        let off = try ByteBench(#"\033[?1049h\033[?1007l"#)
        #expect(await off.start())
        try off.wheel(1, row: 3, column: 2)
        await off.expect("")
        let main = try ByteBench("")
        #expect(await main.start())
        try main.wheel(1, row: 3, column: 2)
        await main.expect("")
        // A program that has the mouse is reported to, on the alternate screen too.
        let mouse = try ByteBench(#"\033[?1049h\033[?1000h\033[?1006h"#)
        #expect(await mouse.start())
        try mouse.wheel(1, row: 3, column: 2)
        await mouse.expect(ByteBench.hex("\u{1b}[<64;3;4M"))
    }

    /// The view tells the seam when it gains or loses the keyboard (`focusChanged`); a window can only be made key for real by a
    /// person (MANUAL.md), so this is the seam's half: the engine's report once mode 1004 is on, nothing before.
    @Test func focusReportsFollowMode1004() async throws {
        let on = try ByteBench(#"\033[?1004h"#)
        #expect(await on.start())
        let interaction = try #require(on.live.view.interaction)
        interaction.focusChanged(true)
        interaction.focusChanged(false)
        await on.expect(ByteBench.hex("\u{1b}[I\u{1b}[O"))
        let off = try ByteBench("")
        #expect(await off.start())
        try #require(off.live.view.interaction).focusChanged(true)
        await off.expect("", "a program that did not ask is not told")
    }
}
#endif
