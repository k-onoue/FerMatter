import XCTest
import AppKit
@testable import SlideMath

final class SourceAssistTests: XCTestCase {
    @MainActor func testImportMentionKeepsProjectFunctionArgumentsInDocumentEditor() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: .zero, styleMask: .borderless, backing: .buffered, defer: true)
        let view = AssistedTextView(); view.sourceFormat = .typst
        window.contentView = view; window.makeFirstResponder(view)
        defer { view.dismissSuggestions(); window.contentView = nil }
        view.projectCompletions = ProjectCompletions.definitions(in: "#let norm(x) = x", format: .typst)
        view.string = "#import \"symbols.typ\": norm\n$ no"
        view.setSelectedRange(NSRange(location: (view.string as NSString).length, length: 0))
        view.showSuggestions(); view.acceptSuggestion()
        XCTAssertTrue(view.string.hasSuffix("norm(x)"))
        XCTAssertEqual((view.string as NSString).substring(with: view.selectedRange()), "x")
        // A local zero-argument definition must still override an older disk definition.
        let local = ProjectCompletions.definitions(in: "#let norm() = 1", format: .typst)
        XCTAssertEqual(ProjectCompletions.unique(local + view.projectCompletions).first?.expansion.text, "norm()")
    }

    func testProjectDefinitionsIncludeArgumentFieldsAndIgnoreComments() throws {
        let latex = #"""
        % \newcommand{\ignored}[1]{#1}
        \newcommand{\vect}[1]{\mathbf{#1}}
        \renewcommand\pair[2][x]{#1+#2}
        \DeclareMathOperator*{\argmin}{argmin}
        \def\myfrac#1#2{#1/#2}
        \NewDocumentCommand{\custom}{m m}{#1 #2}
        \newcommand{\optional}[2][\mathbf{x}]{#1 #2}
        """#
        let tex = Dictionary(uniqueKeysWithValues: ProjectCompletions.definitions(in: latex, format: .latex).map { ($0.trigger, $0.expansion) })
        XCTAssertNil(tex[#"\ignored"#])
        XCTAssertEqual(tex[#"\vect"#]?.text, #"\vect{arg1}"#)
        XCTAssertEqual(tex[#"\pair"#]?.text, #"\pair[x]{arg1}"#)
        XCTAssertEqual(tex[#"\myfrac"#]?.fields.count, 2)
        XCTAssertEqual(tex[#"\custom"#]?.fields.count, 2)
        XCTAssertEqual(tex[#"\argmin"#]?.fields.count, 0)
        XCTAssertEqual(tex[#"\optional"#]?.text, #"\optional[\mathbf{x}]{arg1}"#)
        XCTAssertEqual(tex[#"\optional"#]?.fields.count, 2)
        let typst = #"""
        // #let ignored = 1
        #let rr = math.bb(R)
        #let norm(x, y, size: (1, 2)) = x + y
        #import "symbols.typ": norm as magnitude
        """#
        let typ = Dictionary(uniqueKeysWithValues: ProjectCompletions.definitions(in: typst, format: .typst).map { ($0.trigger, $0.expansion) })
        XCTAssertNil(typ["ignored"])
        XCTAssertEqual(typ["rr"]?.text, "rr")
        XCTAssertEqual(typ["norm"]?.text, "norm(x, y)")
        XCTAssertNotNil(typ["magnitude"])
    }

    @MainActor func testSuggestionsDoNotChangeTextUntilAcceptedAndProjectDefinitionsOverrideBuiltins() throws {
        _ = NSApplication.shared
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 400, height: 300), styleMask: .titled, backing: .buffered, defer: true)
        let view = AssistedTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 300))
        window.contentView = view; window.makeFirstResponder(view)
        defer { view.dismissSuggestions(); window.contentView = nil }
        view.string = #"\f"#; view.setSelectedRange(NSRange(location: 2, length: 0))
        view.showSuggestions()
        XCTAssertEqual(view.suggestions.map(\.trigger), [#"\frac"#])
        XCTAssertEqual(view.string, #"\f"#)
        view.acceptSuggestion()
        XCTAssertEqual(view.string, #"\frac{a}{b}"#)
        XCTAssertEqual((view.string as NSString).substring(with: view.selectedRange()), "a")
        view.insertText("x^2", replacementRange: view.selectedRange()); view.insertTab(nil)
        XCTAssertEqual((view.string as NSString).substring(with: view.selectedRange()), "b")
        view.projectCompletions = ProjectCompletions.definitions(in: #"\newcommand{\frac}[1]{#1}"#, format: .latex)
        view.string = #"\fr"#; view.setSelectedRange(NSRange(location: 3, length: 0)); view.showSuggestions(); view.acceptSuggestion()
        XCTAssertEqual(view.string, #"\frac{arg1}"#)
        view.string = #"\sq"#; view.setSelectedRange(NSRange(location: 3, length: 0)); view.showSuggestions(); view.dismissSuggestions()
        XCTAssertEqual(view.string, #"\sq"#)
        XCTAssertTrue(view.suggestions.isEmpty)
    }

    func testProjectIndexFindsCompanionFilesAndExcludesOtherFormatsAndBuildFiles() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory.appendingPathComponent("build"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        try Data(#"\newcommand{\rr}{R}"#.utf8).write(to: directory.appendingPathComponent("macros.sty"))
        try Data(#"\newcommand{\hidden}{R}"#.utf8).write(to: directory.appendingPathComponent("build/compiled.tex"))
        try Data("#let typst = 1".utf8).write(to: directory.appendingPathComponent("main.typ"))
        XCTAssertEqual(ProjectCompletions.load(root: directory, format: .latex).map(\.trigger), [#"\rr"#])
        XCTAssertEqual(ProjectCompletions.load(root: directory, format: .typst).map(\.trigger), ["typst"])
    }

    @MainActor func testCompletionReplacesSelectionAndTabTracksEditedUnicodeFields() throws {
        let view = AssistedTextView()
        view.string = "前置き 😀 xx tail"; view.allowsUndo = true
        let range = (view.string as NSString).range(of: "xx")
        let fraction = try XCTUnwrap(SourceCompletion.catalog(.latex, context: .math).first { $0.title == "Fraction" })
        view.applyCompletion(fraction, replacing: range)
        XCTAssertEqual(view.string, "前置き 😀 \\frac{a}{b} tail")
        view.insertText("日本語", replacementRange: view.selectedRange())
        view.insertTab(nil)
        XCTAssertEqual((view.string as NSString).substring(with: view.selectedRange()), "b")
        view.insertText("denominator", replacementRange: view.selectedRange())
        view.insertBacktab(nil)
        XCTAssertEqual((view.string as NSString).substring(with: view.selectedRange()), "日本語")
        view.insertTab(nil); view.insertTab(nil)
        XCTAssertEqual(view.selectedRange().location, ("前置き 😀 \\frac{日本語}{denominator}" as NSString).length)
        XCTAssertTrue(view.string.hasSuffix(" tail"))
    }

    @MainActor func testCompletionIsExplicitAndCompletionInsertionCanBeUndone() throws {
        let view = AssistedTextView(); view.allowsUndo = true; view.sourceFormat = .typst
        _ = NSApplication.shared
        let window = NSWindow(contentRect: .zero, styleMask: .borderless, backing: .buffered, defer: true)
        window.contentView = view
        defer { window.contentView = nil }
        XCTAssertNotNil(view.undoManager)
        view.string = "😀 sq"; view.setSelectedRange(NSRange(location: (view.string as NSString).length, length: 0))
        let range = view.rangeForUserCompletion
        var index = 0
        XCTAssertEqual(view.completions(forPartialWordRange: range, indexOfSelectedItem: &index), ["sqrt"])
        view.insertCompletion("sqrt", forPartialWordRange: range, movement: NSReturnTextMovement, isFinal: false)
        XCTAssertEqual(view.string, "😀 sq")
        view.insertCompletion("sqrt", forPartialWordRange: range, movement: NSReturnTextMovement, isFinal: true)
        XCTAssertEqual(view.string, "😀 sqrt(x)")
        XCTAssertEqual((view.string as NSString).substring(with: view.selectedRange()), "x")
        view.undoManager?.undo()
        XCTAssertEqual(view.string, "😀 sq")
    }

    func testCompletionBoundariesAndFigureCompletionsMatchLanguage() throws {
        XCTAssertEqual(SourceCompletion.completionRange(in: #"x + \fr"#, caret: 7, format: .latex), NSRange(location: 4, length: 3))
        XCTAssertEqual(SourceCompletion.completionRange(in: "text", caret: 4, format: .latex).length, 0)
        for format in [ProjectFormat.latex, .typst] {
            let math = SourceCompletion.catalog(format, context: .math)
            XCTAssertFalse(math.contains { $0.title.contains("canvas") })
            let drawing = SourceCompletion.catalog(format, context: .figure)
            XCTAssertTrue(drawing.contains { $0.title == (format == .latex ? "TikZ canvas" : "CeTZ canvas") })
            for completion in drawing {
                XCTAssertFalse(completion.expansion.text.contains("${"))
                XCTAssertTrue(completion.expansion.fields.allSatisfy { NSMaxRange($0) <= (completion.expansion.text as NSString).length })
            }
        }
    }
}
