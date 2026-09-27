import XCTest
import PDFKit
@testable import SlideMath

final class TemplateTests: XCTestCase {
    private let latex = ProjectTemplate.builtIns[0]
    private let typst = ProjectTemplate.builtIns[1]
    private func temporaryDirectory() throws -> URL {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("SlideMathTemplateTests-\(UUID())")
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: url) }
        return url
    }

    func testBareEquationsHandleCommentsBlankLinesAndCRLF() throws {
        let text = "% Ignore $ and \\begin{document} in comments\r\nx^2 + y^2 = 1\r\n\r\n\\text{cost} = \\$5 % price\r\n"
        let prepared = try ProjectComposer.prepare(text, template: latex, mode: .automatic)
        XCTAssertEqual(prepared.interpretation, "One equation per line")
        XCTAssertEqual(prepared.source.components(separatedBy: #"\["#).count, 3)
        XCTAssertTrue(prepared.source.contains("\\text{cost} = \\$5 % price\n\\]"))
        let typ = try ProjectComposer.prepare("x^2=1\r\n\r\ny=2 // a note", template: typst, mode: .equations)
        XCTAssertEqual(typ.source.components(separatedBy: "$ ").count, 3)
        XCTAssertTrue(typ.source.contains("y=2 // a note\n $"))
        XCTAssertEqual(try ProjectComposer.prepare("// $ is ignored\nx=1 // another $", template: typst, mode: .automatic).interpretation, "One equation per line")
    }

    func testSourceKeepsHeadingsMathFiguresAndLiteralPlaceholderText() throws {
        let cases: [(ProjectTemplate, String)] = [
            (latex, #"\section{Model}"# + "\n" + #"\[x^2\]"#),
            (latex, #"\begin{align}x &= 1 \\ y &= 2\end{align}"#),
            (latex, #"\begin{tikzpicture}\draw (0,0)--(1,1);\end{tikzpicture}"#),
            (typst, "= Model\n$ x^2 $"),
            (typst, "#cetz.canvas({ circle((0,0), radius: 1) })")
        ]
        for (template, text) in cases {
            let prepared = try ProjectComposer.prepare(text, template: template, mode: .automatic)
            XCTAssertEqual(prepared.interpretation, "Source · Preserves markup")
            XCTAssertTrue(prepared.source.contains(text))
        }
        let raw = "A literal {{content}} and Unicode 日本語"
        XCTAssertTrue(try ProjectComposer.prepare(raw, template: typst, mode: .source).source.contains(raw))
        XCTAssertEqual(try ProjectComposer.prepare("$x$", template: latex, mode: .equations).interpretation, "One equation per line")
    }

    func testFullLatexDocumentRetainsItsPreambleExactly() throws {
        let document = "\\documentclass{article}\r\n\\newcommand{\\R}{\\mathbb{R}}\r\n\\begin{document}\r\n$\\R$\r\n\\end{document}"
        let prepared = try ProjectComposer.prepare(document, template: latex, mode: .automatic)
        XCTAssertEqual(prepared.source, document)
        XCTAssertEqual(prepared.interpretation, "Full document · Uses pasted preamble")
    }

    func testInvalidTemplatesAndInputFailBeforeCreatingProjectFiles() throws {
        let base = try temporaryDirectory()
        var template = latex.customized()
        for source in ["no placeholder", "{{content}} {{content}}"] {
            template.source = source
            XCTAssertThrowsError(try ProjectComposer.create(name: "test", text: "x=1", template: template, mode: .automatic, directory: base))
        }
        for name in ["", ".", "..", "../outside", "a/b", "a\\b", "a:b", "bad\nname"] {
            XCTAssertThrowsError(try ProjectComposer.create(name: name, text: "x=1", template: latex, mode: .automatic, directory: base))
        }
        XCTAssertThrowsError(try ProjectComposer.create(name: "test", text: " \n\t ", template: latex, mode: .automatic, directory: base))
        XCTAssertTrue(try FileManager.default.contentsOfDirectory(atPath: base.path).isEmpty)
    }

    func testProjectCreationNeverOverwritesAndWritesCompilerConfiguration() throws {
        let base = try temporaryDirectory()
        var template = latex.customized(); template.compiler = "xelatex"
        let first = try ProjectComposer.create(name: "メモ", text: "x=1", template: template, mode: .automatic, directory: base)
        let original = try Data(contentsOf: first)
        let second = try ProjectComposer.create(name: "メモ", text: "y=2", template: typst, mode: .automatic, directory: base)
        XCTAssertEqual(first.deletingLastPathComponent().lastPathComponent, "メモ")
        XCTAssertEqual(second.deletingLastPathComponent().lastPathComponent, "メモ 2")
        XCTAssertEqual(try Data(contentsOf: first), original)
        XCTAssertEqual(try String(contentsOf: first.deletingLastPathComponent().appendingPathComponent("latexmkrc")), "$pdf_mode = 5;\n")
        XCTAssertEqual(second.lastPathComponent, "main.typ")
        XCTAssertFalse(FileManager.default.fileExists(atPath: second.deletingLastPathComponent().appendingPathComponent("latexmkrc").path))
    }

    @MainActor func testCustomTemplatePersistenceDefaultDeletionAndProjectIndependence() throws {
        let base = try temporaryDirectory()
        let store = TemplateStore(support: base)
        var custom = latex.customized(); custom.name = "My macros"
        custom.source = custom.source.replacingOccurrences(of: #"\pagestyle"#, with: "\\newcommand{\\RR}{\\mathbb{R}}\n\\pagestyle")
        try store.save(custom); try store.setDefault(custom)
        let reloaded = TemplateStore(support: base)
        XCTAssertEqual(reloaded.defaultTemplate, custom)
        let main = try ProjectComposer.create(name: "Saved", text: #"x\in\RR"#, template: custom, mode: .automatic, directory: base)
        let original = try Data(contentsOf: main)
        custom.source = custom.source.replacingOccurrences(of: "mathbb", with: "mathbf")
        try reloaded.save(custom)
        XCTAssertEqual(try Data(contentsOf: main), original)
        try reloaded.delete(custom)
        XCTAssertEqual(TemplateStore(support: base).templates, ProjectTemplate.builtIns)
        XCTAssertEqual(TemplateStore(support: base).defaultTemplate, latex)
        XCTAssertEqual(try Data(contentsOf: main), original)
        XCTAssertThrowsError(try store.save(latex))
    }

    @MainActor func testUnreadableTemplateConfigurationIsNotOverwritten() throws {
        let base = try temporaryDirectory(), data = Data("broken json".utf8)
        let file = base.appendingPathComponent("templates.json")
        try data.write(to: file)
        let store = TemplateStore(support: base)
        XCTAssertNotNil(store.error)
        XCTAssertThrowsError(try store.save(latex.customized()))
        XCTAssertEqual(try Data(contentsOf: file), data)
    }

    func testBuiltInProjectsCompileToSVGWithActualWorkers() throws {
        guard ProcessInfo.processInfo.environment["SLIDEMATH_COMPILER_TESTS"] == "1" else {
            throw XCTSkip("Set SLIDEMATH_COMPILER_TESTS=1 to run real LaTeX/Typst integration tests.")
        }
        let base = try temporaryDirectory()
        let samples = [
            "x^2+y^2=r^2\n\\int_0^1 x^2\\,dx=\\frac{1}{3}",
            "x^2+y^2=r^2\nintegral_0^1 x^2 dif x=1/3",
            "\\section{Shapes}\n\\begin{tikzpicture}\n\\draw[blue] (0,0)--(2,1);\n\\end{tikzpicture}\n$E=mc^2$",
            "= Shapes\n#cetz.canvas({\n import cetz.draw: *\n circle((0,0), radius: 1, stroke: blue)\n})\n$ E=m c^2 $"
        ]
        for (template, source) in zip(ProjectTemplate.builtIns, samples) {
            let main = try ProjectComposer.create(name: template.name, text: source, template: template, mode: .automatic, directory: base)
            try assertRendered(main: main, base: base, count: 2)
        }
        var custom = latex.customized()
        custom.source = custom.source.replacingOccurrences(of: #"\pagestyle"#, with: "\\newcommand{\\RR}{\\mathbb{R}}\n\\pagestyle")
        let main = try ProjectComposer.create(name: "Custom", text: #"x\in\RR"#, template: custom, mode: .automatic, directory: base)
        try assertRendered(main: main, base: base, count: 1)
        var customTypst = typst.customized()
        customTypst.source = "#let rr = $bold(R)$\n" + customTypst.source
        let typ = try ProjectComposer.create(name: "Custom Typst", text: "x in rr", template: customTypst, mode: .automatic, directory: base)
        try assertRendered(main: typ, base: base, count: 1)
    }

    func testMathSnippetsCompileWithBothRealCompilers() throws {
        guard ProcessInfo.processInfo.environment["SLIDEMATH_COMPILER_TESTS"] == "1" else { throw XCTSkip("Compiler integration disabled") }
        let base = try temporaryDirectory()
        for template in [latex, typst] {
            let snippets = SourceCompletion.catalog(template.format, context: .math)
            let source = snippets.map {
                template.format == .latex ? "\\[\n" + $0.expansion.text + "\n\\]" : "$ " + $0.expansion.text + " $"
            }.joined(separator: "\n\n")
            let main = try ProjectComposer.create(name: template.name, text: source, template: template, mode: .source, directory: base)
            try assertRendered(main: main, base: base, count: snippets.count)
        }
    }

    private func assertRendered(main: URL, base: URL, count: Int) throws {
        let root = main.deletingLastPathComponent()
        let source = try Data(contentsOf: main)
        let scan = try worker(["action": "scan", "root": root.path, "main": main.lastPathComponent], base: base)
        let cards = try XCTUnwrap(scan.first { $0["event"] as? String == "scanned" }?["cards"] as? [[String: Any]])
        XCTAssertEqual(cards.count, count, main.path)
        let events = try worker(["action": "render", "root": root.path, "main": main.lastPathComponent, "engine": "project", "cards": cards, "cache": base.appendingPathComponent("Cache-\(UUID())").path], base: base)
        let rendered = events.filter { $0["event"] as? String == "card" }
        XCTAssertEqual(rendered.count, count)
        for event in rendered {
            XCTAssertEqual(event["status"] as? String, "ready", "\(event)")
            let path = try XCTUnwrap(event["svgPath"] as? String)
            XCTAssertTrue(try String(contentsOfFile: path).contains("<svg"))
            let pdf = URL(fileURLWithPath: try XCTUnwrap(event["pdfPath"] as? String))
            XCTAssertEqual(PDFDocument(url: pdf)?.pageCount, 1)
            for format in [ImageExportFormat.png, .jpeg, .pdf] {
                let exported = try ImageExporter.file(svg: URL(fileURLWithPath: path), pdf: pdf, png: nil, format: format, directory: base.appendingPathComponent("Exports"))
                XCTAssertGreaterThan(try Data(contentsOf: exported).count, 50)
            }
        }
        XCTAssertEqual(try Data(contentsOf: main), source)
    }

    private func worker(_ request: [String: Any], base: URL) throws -> [[String: Any]] {
        let input = base.appendingPathComponent("request.json")
        try JSONSerialization.data(withJSONObject: request).write(to: input)
        let directory = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let process = Process(), output = Pipe()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["python3", "-B", directory.appendingPathComponent("Resources/tex_worker.py").path, input.path]
        process.standardOutput = output
        try process.run()
        let data = output.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        XCTAssertEqual(process.terminationStatus, 0, String(decoding: data, as: UTF8.self))
        return try String(decoding: data, as: UTF8.self).split(separator: "\n").map { try JSONSerialization.jsonObject(with: Data($0.utf8)) as! [String: Any] }
    }
}
