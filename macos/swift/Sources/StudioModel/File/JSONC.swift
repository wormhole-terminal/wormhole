// Theme files are JSON with comments and trailing commas (themes.md). Foundation reads plain JSON only, so
// both are removed first, outside strings, keeping every other byte (and so every error's position) in place.

import Foundation

enum JSONC {
    enum Failure: Error, Equatable { case notAnObject, syntax(String) }

    /// The file's root object; throws with the reason a hand-edited file would be refused for.
    static func parse(_ text: String) throws -> JSONValue {
        let data = Data(strip(text).utf8)
        do {
            let root = try JSONSerialization.jsonObject(with: data)
            guard root is [String: Any] else { throw Failure.notAnObject }
            return JSONValue(foundation: root)
        } catch let failure as Failure {
            throw failure
        } catch {
            throw Failure.syntax(error.localizedDescription)
        }
    }

    static func strip(_ text: String) -> String {
        var out = String.UnicodeScalarView()
        let scalars = Array(text.unicodeScalars)
        var index = 0
        var inString = false
        while index < scalars.count {
            let c = scalars[index]
            if inString {
                out.append(c)
                if c == "\\", index + 1 < scalars.count { out.append(scalars[index + 1]); index += 1 } else if c == "\"" { inString = false }
            } else if c == "\"" {
                inString = true
                out.append(c)
            } else if c == "/", index + 1 < scalars.count, scalars[index + 1] == "/" {
                while index < scalars.count, scalars[index] != "\n" { index += 1 }
                continue
            } else if c == "/", index + 1 < scalars.count, scalars[index + 1] == "*" {
                index += 2
                while index + 1 < scalars.count, !(scalars[index] == "*" && scalars[index + 1] == "/") { index += 1 }
                index += 2
                continue
            } else {
                out.append(c)
            }
            index += 1
        }
        return removeTrailingCommas(String(out))
    }

    private static func removeTrailingCommas(_ text: String) -> String {
        var out = String.UnicodeScalarView()
        let scalars = Array(text.unicodeScalars)
        var inString = false
        for (index, c) in scalars.enumerated() {
            if inString {
                out.append(c)
                if c == "\"", scalars[index - 1] != "\\" { inString = false }
                continue
            }
            if c == "\"" { inString = true } else if c == "," {
                let next = scalars[(index + 1)...].first { !CharacterSet.whitespacesAndNewlines.contains($0) }
                if next == "}" || next == "]" { continue }
            }
            out.append(c)
        }
        return String(out)
    }
}
