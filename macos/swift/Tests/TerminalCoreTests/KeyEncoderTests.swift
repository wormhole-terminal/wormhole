import Foundation
import Testing

@testable import TerminalCore

@Suite struct KeyEncoderTests {
    private func bytes(_ input: KeyInput) -> [UInt8]? { KeyEncoder.encode(input).map(Array.init) }

    @Test func plainKeys() {
        #expect(bytes(KeyInput(key: "a", text: "a")) == [0x61])
        #expect(bytes(KeyInput(key: "enter")) == [0x0D])
        #expect(bytes(KeyInput(key: "backspace")) == [0x7F])
        #expect(bytes(KeyInput(key: "escape")) == [0x1B])
        #expect(bytes(KeyInput(key: "tab", shift: true)) == Array("\u{1B}[Z".utf8))
    }

    @Test func controlAndOption() {
        #expect(bytes(KeyInput(key: "c", text: "c", control: true)) == [0x03])
        #expect(bytes(KeyInput(key: "space", text: " ", control: true)) == [0x00])
        // Option as meta: ESC and the base character, not the composed one the layout typed.
        #expect(bytes(KeyInput(key: "b", text: "∫", alt: true)) == [0x1B, 0x62])
        #expect(bytes(KeyInput(key: "b", text: "∫", alt: true, optionAsMeta: false)) == Array("∫".utf8))
    }

    @Test func cursorAndFunctionKeysCarryModifiers() {
        #expect(bytes(KeyInput(key: "up")) == Array("\u{1B}[A".utf8))
        #expect(bytes(KeyInput(key: "left", control: true)) == Array("\u{1B}[1;5D".utf8))
        #expect(bytes(KeyInput(key: "f1")) == Array("\u{1B}OP".utf8))
        #expect(bytes(KeyInput(key: "f5", shift: true)) == Array("\u{1B}[15;2~".utf8))
        #expect(bytes(KeyInput(key: "delete")) == Array("\u{1B}[3~".utf8))
    }

    @Test func commandIsNotTheProgramsKey() {
        #expect(bytes(KeyInput(key: "v", text: "v", platform: true)) == nil)
    }
}
