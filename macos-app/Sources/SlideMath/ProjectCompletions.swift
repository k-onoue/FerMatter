import Foundation

/// A local, read-only symbol index. It does not execute TeX or Typst source.
enum ProjectCompletions {
    static func load(root: URL, format: ProjectFormat) -> [SourceCompletion] {
        let root = root.resolvingSymlinksInPath()
        let keys: [URLResourceKey] = [.isDirectoryKey, .isRegularFileKey, .fileSizeKey]
        guard let files = FileManager.default.enumerator(at: root, includingPropertiesForKeys: keys, options: [.skipsHiddenFiles, .skipsPackageDescendants]) else { return [] }
        var result: [SourceCompletion] = [], bytes = 0
        for case let file as URL in files {
            guard let info = try? file.resourceValues(forKeys: Set(keys)) else { continue }
            if info.isDirectory == true {
                if ["build", "node_modules", "Cache", "Exports", "vendor"].contains(file.lastPathComponent) { files.skipDescendants() }
                continue
            }
            guard info.isRegularFile == true,
                  (format == .latex ? ["tex", "sty", "cls"] : ["typ"]).contains(file.pathExtension.lowercased()),
                  file.resolvingSymlinksInPath().path.hasPrefix(root.path + "/"),
                  let size = info.fileSize, size <= 2_000_000 else { continue }
            bytes += size
            if bytes > 20_000_000 { break }
            if let text = try? String(contentsOf: file, encoding: .utf8) {
                result += definitions(in: text, format: format, origin: file.lastPathComponent)
            }
        }
        // Prefer a function definition over an import-only mention of its name.
        return unique(result).sorted { $0.trigger.localizedStandardCompare($1.trigger) == .orderedAscending }
    }

    static func unique(_ items: [SourceCompletion]) -> [SourceCompletion] {
        var result: [SourceCompletion] = [], positions: [String: Int] = [:]
        for item in items {
            if let index = positions[item.trigger] {
                if result[index].isImportOnly && !item.isImportOnly { result[index] = item }
            } else {
                positions[item.trigger] = result.count; result.append(item)
            }
        }
        return result
    }

    static func definitions(in text: String, format: ProjectFormat, origin: String = "Local") -> [SourceCompletion] {
        let text = uncomment(text, format: format), source = text as NSString
        var result: [SourceCompletion] = []
        func matches(_ pattern: String, _ consume: (NSTextCheckingResult) -> Void) {
            guard let regex = try? NSRegularExpression(pattern: pattern) else { return }
            regex.matches(in: text, range: NSRange(location: 0, length: source.length)).forEach(consume)
        }
        func capture(_ match: NSTextCheckingResult, _ index: Int) -> String {
            match.range(at: index).location == NSNotFound ? "" : source.substring(with: match.range(at: index))
        }
        func field(_ index: Int, _ label: String) -> String { "${\(index):\(label)}" }
        func add(_ name: String, _ suffix: String = "", importOnly: Bool = false) {
            result.append(SourceCompletion(trigger: name, title: origin, template: name + suffix, isImportOnly: importOnly))
        }
        if format == .latex {
            matches(#"\\(?:newcommand|renewcommand|providecommand|DeclareRobustCommand)\*?\s*\{?\s*(\\[A-Za-z@]+)\s*\}?\s*(?:\[\s*([0-9])\s*\])?\s*(?:\[([^\]\n]*)\])?"#) { match in
                let count = Int(capture(match, 2)) ?? 0
                let optional = match.range(at: 3).location != NSNotFound && count > 0
                var suffix = optional ? "[" + field(1, capture(match, 3)) + "]" : ""
                for index in 0..<max(0, count - (optional ? 1 : 0)) { suffix += "{" + field(index + (optional ? 2 : 1), "arg\(index + 1)") + "}" }
                add(capture(match, 1), suffix)
            }
            matches(#"\\(?:DeclareMathOperator)\*?\s*\{\s*(\\[A-Za-z@]+)\s*\}"#) { add(capture($0, 1)) }
            matches(#"\\(?:gdef|edef|xdef|def)\s*(\\[A-Za-z@]+)((?:\s*#[1-9])*)\s*\{"#) { match in
                let count = capture(match, 2).filter { $0 == "#" }.count
                add(capture(match, 1), (0..<count).map { "{" + field($0 + 1, "arg\($0 + 1)") + "}" }.joined())
            }
            matches(#"\\(?:New|Renew|Provide|Declare)DocumentCommand\s*\{\s*(\\[A-Za-z@]+)\s*\}\s*\{([mo\s]+)\}"#) { match in
                let args = capture(match, 2).filter { !$0.isWhitespace }
                add(capture(match, 1), args.enumerated().map { index, kind in
                    (kind == "o" ? "[" : "{") + field(index + 1, "arg\(index + 1)") + (kind == "o" ? "]" : "}")
                }.joined())
            }
        } else {
            matches(#"(?:#|\b)let\s+([\p{L}_][\p{L}\p{N}_-]*)\s*"#) { match in
                let name = capture(match, 1), end = NSMaxRange(match.range)
                guard end < source.length else { return }
                if source.character(at: end) == 40, let close = closingParenthesis(source, from: end) {
                    let arguments = source.substring(with: NSRange(location: end + 1, length: close - end - 1))
                    // Named parameters have defaults and can safely be omitted.
                    let required = splitArguments(arguments).filter { !$0.contains(":") && !$0.hasPrefix("..") && !$0.isEmpty }
                    let suffix = "(" + required.enumerated().map { field($0.offset + 1, $0.element) }.joined(separator: ", ") + ")"
                    add(name, suffix)
                } else if source.character(at: end) == 61 { add(name) }
            }
            matches(#"(?m)#import\s+[^\n]+:\s*([^\n]+)"#) { match in
                for symbol in capture(match, 1).split(separator: ",") {
                    let name = symbol.components(separatedBy: " as ").last!.trimmingCharacters(in: .whitespaces)
                    if name.range(of: #"^[\p{L}_][\p{L}\p{N}_-]*$"#, options: .regularExpression) != nil { add(name, importOnly: true) }
                }
            }
        }
        return unique(result)
    }

    private static func closingParenthesis(_ text: NSString, from start: Int) -> Int? {
        var depth = 0, quoted = false
        for i in start..<text.length {
            let c = text.character(at: i)
            if c == 34 && (i == 0 || text.character(at: i - 1) != 92) { quoted.toggle() }
            if quoted { continue }
            if c == 40 { depth += 1 }
            if c == 41 { depth -= 1; if depth == 0 { return i } }
        }
        return nil
    }
    private static func splitArguments(_ text: String) -> [String] {
        var depth = 0, quoted = false, previous: Character = " ", value = "", result: [String] = []
        for c in text {
            if c == "\"" && previous != "\\" { quoted.toggle() }
            if !quoted {
                if "([{<".contains(c) { depth += 1 }
                if ")]}>".contains(c) { depth -= 1 }
                if c == ",", depth == 0 { result.append(value.trimmingCharacters(in: .whitespacesAndNewlines)); value = ""; continue }
            }
            value.append(c); previous = c
        }
        result.append(value.trimmingCharacters(in: .whitespacesAndNewlines)); return result
    }
    private static func uncomment(_ text: String, format: ProjectFormat) -> String {
        let pattern = format == .latex ? #"(?<!\\)%[^\n]*|\\begin\{(?:verbatim|lstlisting)\}[\s\S]*?\\end\{(?:verbatim|lstlisting)\}"# : #"/\*[\s\S]*?\*/|(?<!:)//[^\n]*"#
        return text.replacingOccurrences(of: pattern, with: " ", options: .regularExpression)
    }
}
