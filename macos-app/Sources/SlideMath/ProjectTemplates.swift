import Foundation
import SwiftUI

struct ProjectTemplate: Codable, Identifiable, Equatable {
    var id: String
    var name: String
    var format: ProjectFormat
    var source: String
    var compiler = "pdflatex"
    var isBuiltIn: Bool { Self.builtIns.contains { $0.id == id } }
    static let placeholder = "{{content}}"

    static let builtIns: [ProjectTemplate] = {
        let latex = #"""
        \documentclass[12pt]{article}
        \usepackage{amsmath,amssymb,mathtools,bm}
        \pagestyle{empty}
        \begin{document}
        {{content}}
        \end{document}
        """#
        let typst = """
        #set text(size: 12pt)
        #set math.equation(numbering: none)

        {{content}}
        """
        return [
            .init(id: "builtin-latex", name: "LaTeX", format: .latex, source: latex),
            .init(id: "builtin-typst", name: "Typst", format: .typst, source: typst),
            .init(id: "builtin-tikz", name: "LaTeX + TikZ", format: .latex,
                  source: latex.replacingOccurrences(of: #"\pagestyle"#, with: "\\usepackage{tikz}\n\\usetikzlibrary{arrows.meta,calc,positioning}\n\\pagestyle")),
            .init(id: "builtin-cetz", name: "Typst + CeTZ", format: .typst,
                  source: "#import \"@preview/cetz:0.5.2\"\n" + typst)
        ]
    }()

    func validate() throws {
        guard !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { throw ProjectCreationError("Enter a template name.") }
        guard source.components(separatedBy: Self.placeholder).count == 2 else {
            throw ProjectCreationError("Place {{content}} exactly once in the template.")
        }
        if format == .latex && !["pdflatex", "xelatex", "lualatex"].contains(compiler) {
            throw ProjectCreationError("Choose a LaTeX compiler.")
        }
    }

    func customized() -> ProjectTemplate {
        var copy = self
        copy.id = UUID().uuidString; copy.name += " copy"
        return copy
    }
}

struct ProjectCreationError: LocalizedError {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

@MainActor
final class TemplateStore: ObservableObject {
    private struct Configuration: Codable {
        var custom: [ProjectTemplate] = []
        var defaultID = ProjectTemplate.builtIns[0].id
    }
    @Published private var configuration: Configuration
    @Published var error: String?
    private let file: URL
    var templates: [ProjectTemplate] { ProjectTemplate.builtIns + configuration.custom }
    var defaultTemplate: ProjectTemplate { templates.first { $0.id == configuration.defaultID } ?? ProjectTemplate.builtIns[0] }

    init(support: URL? = nil) {
        let support = support ?? AppIdentity.applicationSupport
        file = support.appendingPathComponent("templates.json")
        do {
            if FileManager.default.fileExists(atPath: file.path) {
                configuration = try JSONDecoder().decode(Configuration.self, from: Data(contentsOf: file))
            } else { configuration = Configuration() }
        } catch {
            configuration = Configuration()
            self.error = "Could not load templates: \(error.localizedDescription)"
        }
    }

    func save(_ template: ProjectTemplate) throws {
        try template.validate()
        guard !template.isBuiltIn else { throw ProjectCreationError("Customize a built-in template to save your own copy.") }
        var next = configuration
        if let index = next.custom.firstIndex(where: { $0.id == template.id }) { next.custom[index] = template }
        else { next.custom.append(template) }
        try persist(next)
    }

    func setDefault(_ template: ProjectTemplate) throws {
        var next = configuration; next.defaultID = template.id
        try persist(next)
    }

    func delete(_ template: ProjectTemplate) throws {
        var next = configuration
        next.custom.removeAll { $0.id == template.id }
        if next.defaultID == template.id { next.defaultID = ProjectTemplate.builtIns[0].id }
        try persist(next)
    }

    private func persist(_ next: Configuration) throws {
        // Leave an unreadable configuration untouched, so it can be recovered.
        guard error == nil else { throw ProjectCreationError(error!) }
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try encoder.encode(next).write(to: file, options: .atomic)
        configuration = next
    }
}

enum ProjectInputMode: String, CaseIterable, Identifiable {
    case automatic, equations, source
    var id: Self { self }
    var title: String {
        switch self { case .automatic: "Auto"; case .equations: "One per line"; case .source: "Source" }
    }
}

struct PreparedProject {
    var source: String
    var interpretation: String
}

enum ProjectComposer {
    static func prepare(_ text: String, template: ProjectTemplate, mode: ProjectInputMode) throws -> PreparedProject {
        try template.validate()
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { throw ProjectCreationError("Paste some equations or source text.") }
        let normalized = text.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
        let code = template.format == .latex ? withoutLaTeXComments(normalized)
            : normalized.replacingOccurrences(of: #"(?s)/\*.*?\*/|(?m)//[^\n]*"#, with: "", options: .regularExpression)
        if mode != .equations, template.format == .latex,
           code.range(of: #"\\begin\s*\{document\}"#, options: .regularExpression) != nil {
            return PreparedProject(source: text, interpretation: "Full document · Uses pasted preamble")
        }
        let isSource = mode == .source || (mode == .automatic && looksLikeSource(code, format: template.format))
        let content: String
        if isSource { content = text }
        else {
            let lines = normalized.components(separatedBy: "\n")
            content = lines.map { line in
                let trimmed = line.trimmingCharacters(in: .whitespaces)
                if trimmed.isEmpty || trimmed.hasPrefix(template.format == .latex ? "%" : "//") { return line }
                return template.format == .latex ? "\\[\n\(line)\n\\]" : "$ \(line)\n $"
            }.joined(separator: "\n\n")
        }
        return PreparedProject(source: template.source.replacingOccurrences(of: ProjectTemplate.placeholder, with: content),
                               interpretation: isSource ? "Source · Preserves markup" : "One equation per line")
    }

    private static func looksLikeSource(_ text: String, format: ProjectFormat) -> Bool {
        let pattern = format == .latex
            ? #"(?<!\\)(?:\\\\)*\$|\\[\[(]|\\(?:begin|end|section|subsection|subsubsection|chapter|part|paragraph|subparagraph|input|include|documentclass|usepackage|newcommand|renewcommand|providecommand|def)\b"#
            : #"(?m)(?<!\\)(?:\\\\)*\$|^\s*(?:#|={1,6}\s|```)|#(?:import|include|let|set|show|\w+\.)"#
        return text.range(of: pattern, options: .regularExpression) != nil
    }

    private static func withoutLaTeXComments(_ text: String) -> String {
        text.components(separatedBy: "\n").map { line in
            var escaped = false
            for index in line.indices {
                let char = line[index]
                if char == "%" && !escaped { return String(line[..<index]) }
                escaped = char == "\\" && !escaped
            }
            return line
        }.joined(separator: "\n")
    }

    static func create(name: String, text: String, template: ProjectTemplate, mode: ProjectInputMode, directory: URL) throws -> URL {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, !name.hasPrefix("."), name.utf8.count <= 180,
              name.rangeOfCharacter(from: CharacterSet(charactersIn: "/\\:").union(.controlCharacters)) == nil else {
            throw ProjectCreationError("Use a project name without slashes, colons, or a leading dot (up to 180 bytes).")
        }
        let prepared = try prepare(text, template: template, mode: mode)
        let manager = FileManager.default
        try manager.createDirectory(at: directory, withIntermediateDirectories: true)
        var folder = directory.appendingPathComponent(name, isDirectory: true)
        var suffix = 2
        while manager.fileExists(atPath: folder.path) {
            folder = directory.appendingPathComponent("\(name) \(suffix)", isDirectory: true); suffix += 1
        }
        // Reserve the folder without overwriting any existing project.
        try manager.createDirectory(at: folder, withIntermediateDirectories: false)
        do {
            let main = folder.appendingPathComponent(template.format == .latex ? "main.tex" : "main.typ")
            try Data(prepared.source.utf8).write(to: main, options: .atomic)
            if template.format == .latex {
                let pdfMode = ["pdflatex": 1, "lualatex": 4, "xelatex": 5][template.compiler]!
                try Data("$pdf_mode = \(pdfMode);\n".utf8).write(to: folder.appendingPathComponent("latexmkrc"), options: .atomic)
            }
            return main
        } catch {
            try? manager.removeItem(at: folder)
            throw error
        }
    }
}
