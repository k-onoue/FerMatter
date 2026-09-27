import Foundation

enum SourceContext { case math, figure, document }

struct SourceCompletion: Identifiable {
    var trigger: String
    var title: String
    var template: String
    var isImportOnly = false
    var id: String { trigger }

    var expansion: (text: String, fields: [NSRange]) {
        let regex = try! NSRegularExpression(pattern: #"\$\{\d+:"#)
        let source = template as NSString
        var text = "", fields: [NSRange] = [], cursor = 0
        for match in regex.matches(in: template, range: NSRange(location: 0, length: source.length)) {
            guard match.range.location >= cursor else { continue }
            let start = NSMaxRange(match.range)
            var end = start, depth = 1
            while end < source.length {
                let char = source.character(at: end)
                if char == 123 { depth += 1 }
                if char == 125 { depth -= 1; if depth == 0 { break } }
                end += 1
            }
            guard depth == 0 else { continue }
            text += source.substring(with: NSRange(location: cursor, length: match.range.location - cursor))
            let value = source.substring(with: NSRange(location: start, length: end - start))
            fields.append(NSRange(location: (text as NSString).length, length: (value as NSString).length))
            text += value; cursor = end + 1
        }
        text += source.substring(from: cursor)
        return (text, fields)
    }

    static func catalog(_ format: ProjectFormat, context: SourceContext) -> [SourceCompletion] {
        var entries: [(String, String, String)]
        if format == .latex {
            entries = [
                (#"\frac"#, "Fraction", #"\frac{${1:a}}{${2:b}}"#),
                (#"\sqrt"#, "Square root", #"\sqrt{${1:x}}"#),
                (#"\sum"#, "Sum", #"\sum_{${1:i=1}}^{${2:n}} ${3:x_i}"#),
                (#"\int"#, "Integral", #"\int_{${1:0}}^{${2:1}} ${3:f(x)}\,d${4:x}"#),
                (#"\matrix"#, "Matrix", "\\begin{pmatrix}\n  ${1:a} & ${2:b} \\\\\n  ${3:c} & ${4:d}\n\\end{pmatrix}"),
                (#"\cases"#, "Cases", "\\begin{cases}\n  ${1:x} & ${2:x > 0} \\\\\n  ${3:0} & \\text{otherwise}\n\\end{cases}"),
                (#"\aligned"#, "Aligned equations", "\\begin{aligned}\n  ${1:a} &= ${2:b} \\\\\n  ${3:c} &= ${4:d}\n\\end{aligned}"),
                (#"\mathbb"#, "Blackboard bold", #"\mathbb{${1:R}}"#),
                (#"\mathbf"#, "Bold", #"\mathbf{${1:x}}"#),
                (#"\text"#, "Text", #"\text{${1:text}}"#),
                (#"\left"#, "Parentheses", #"\left( ${1:x} \right)"#)
            ]
            if context != .math {
                entries += [
                    (#"\tikzpicture"#, "TikZ canvas", "\\begin{tikzpicture}\n  \\draw[blue, thick] (0,0) -- (${1:2},${2:1});\n\\end{tikzpicture}"),
                    (#"\draw"#, "TikZ line", #"\draw[${1:blue}, thick] (0,0) -- (${2:2},${3:1});"#),
                    (#"\circle"#, "TikZ circle", #"\draw[blue, thick] (${1:0},${2:0}) circle (${3:1cm});"#),
                    (#"\node"#, "TikZ label", #"\node at (${1:0},${2:0}) {${3:Label}};"#)
                ]
            }
            if context == .document {
                entries += [(#"\section"#, "Section", #"\section{${1:Title}}"#),
                            (#"\subsection"#, "Subsection", #"\subsection{${1:Title}}"#),
                            (#"\usepackage"#, "Package", #"\usepackage{${1:package}}"#),
                            (#"\newcommand"#, "Command", #"\newcommand{\${1:name}}[${2:1}]{${3:#1}}"#),
                            (#"\DeclareMathOperator"#, "Math operator", #"\DeclareMathOperator{\${1:name}}{${2:name}}"#),
                            (#"\input"#, "Input file", #"\input{${1:file}}"#),
                            (#"\include"#, "Include file", #"\include{${1:file}}"#),
                            (#"\equation"#, "Display equation", "\\[\n  ${1:x=1}\n\\]")]
            }
        } else {
            entries = [
                ("frac", "Fraction", "frac(${1:a}, ${2:b})"),
                ("sqrt", "Square root", "sqrt(${1:x})"),
                ("sum", "Sum", "sum_(${1:i=1})^${2:n} ${3:x_i}"),
                ("integral", "Integral", "integral_${1:0}^${2:1} ${3:f(x)} dif ${4:x}"),
                ("mat", "Matrix", "mat(${1:a}, ${2:b}; ${3:c}, ${4:d})"),
                ("cases", "Cases", "cases(${1:x} & \"if\" ${2:x > 0}, ${3:0} & \"otherwise\")"),
                ("aligned", "Aligned equations", "${1:a} &= ${2:b} \\\n${3:c} &= ${4:d}"),
                ("bb", "Blackboard bold", "bb(${1:R})"),
                ("bold", "Bold", "bold(${1:x})"),
                ("lr", "Parentheses", "lr((${1:x}))")
            ]
            if context != .math {
                entries += [
                    ("#cetz.canvas", "CeTZ canvas", "#import \"@preview/cetz:0.5.2\"\n#cetz.canvas({\n  import cetz.draw: *\n  line((0, 0), (${1:2}, ${2:1}), stroke: blue + 1pt)\n})"),
                    ("line", "CeTZ line", "line((0, 0), (${1:2}, ${2:1}), stroke: blue + 1pt)"),
                    ("circle", "CeTZ circle", "circle((${1:0}, ${2:0}), radius: ${3:1}, stroke: blue + 1pt)"),
                    ("content", "CeTZ label", "content((${1:0}, ${2:0}), [${3:Label}])")
                ]
            }
            if context == .document {
                entries += [("heading", "Section", "= ${1:Title}\n"),
                            ("#import", "Import", "#import \"${1:package-or-file}\": ${2:symbol}"),
                            ("#include", "Include file", "#include \"${1:file.typ}\""),
                            ("#let", "Function", "#let ${1:name}(${2:x}) = ${3:x}"),
                            ("#set", "Set rule", "#set ${1:text}(${2:size: 12pt})"),
                            ("#show", "Show rule", "#show ${1:heading}: ${2:it => it}"),
                            ("equation", "Display equation", "$ ${1:x=1} $")]
            }
        }
        entries += ["alpha", "beta", "gamma", "delta", "theta", "lambda", "mu", "pi", "sigma", "phi", "omega", "infty"].map {
            let word = format == .latex ? "\\" + $0 : ($0 == "infty" ? "infinity" : $0)
            return (word, $0.capitalized, word)
        }
        return entries.map { SourceCompletion(trigger: $0.0, title: $0.1, template: $0.2) }
    }

    static func completionRange(in text: String, caret: Int, format: ProjectFormat) -> NSRange {
        let source = text as NSString
        guard caret > 0, caret <= source.length else { return NSRange(location: caret, length: 0) }
        var start = caret
        let letters = CharacterSet.letters.union(CharacterSet(charactersIn: format == .latex ? "@" : "._-0123456789"))
        while start > 0, let scalar = UnicodeScalar(source.character(at: start - 1)), letters.contains(scalar) { start -= 1 }
        if start > 0, source.character(at: start - 1) == (format == .latex ? 92 : 35) { start -= 1 }
        if format == .latex && (start == caret || source.character(at: start) != 92) { return NSRange(location: caret, length: 0) }
        return NSRange(location: start, length: caret - start)
    }
}
