import XCTest
import AppKit
import SwiftUI
@testable import SlideMath

final class NavigationTests: XCTestCase {
    @MainActor func testSourceMatchingUsesLineRangesAndColumnsAndIgnoresCustomCopies() {
        let store = LibraryStore(support: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
        var first = Equation.draft(); first.id = "a"; first.origin = "source"; first.file = "main.tex"; first.line = 4; first.endLine = 4; first.column = 5; first.endColumn = 10; first.sourceOrder = 1
        var second = first; second.id = "b"; second.column = 30; second.endColumn = 40; second.sourceOrder = 2
        var block = first; block.id = "c"; block.line = 10; block.endLine = 15; block.sourceOrder = 3
        var custom = first; custom.id = "custom"; custom.origin = "custom"; custom.sourceOrder = 0
        store.cards = [custom, first, second, block]
        XCTAssertEqual(store.nearestSourceEquation(file: "main.tex", line: 4, column: 7)?.id, "a")
        XCTAssertEqual(store.nearestSourceEquation(file: "main.tex", line: 4, column: 35)?.id, "b")
        XCTAssertEqual(store.nearestSourceEquation(file: "main.tex", line: 13, column: -1)?.id, "c")
        XCTAssertNil(store.nearestSourceEquation(file: "elsewhere.tex", line: 4, column: -1))
        XCTAssertNil(store.nearestSourceEquation(file: "main.tex", line: 100, column: -1))
        store.query = "hidden"; store.kind = "inline"; store.origin = "custom"; store.sectionFilter = "outline:other"
        store.selectPDFTarget("c")
        XCTAssertEqual(store.selection, "c")
        XCTAssertTrue(store.filtered.contains { $0.id == "c" })
        XCTAssertEqual(store.cards.count, 4)
    }

    @MainActor func testPDFCoordinatesRespectPageOriginAndResetDisablesNavigation() {
        XCTAssertEqual(PDFSession.pagePoint(x: 70, y: 30, bounds: NSRect(x: 10, y: 20, width: 600, height: 800)), NSPoint(x: 80, y: 790))
        let pdf = PDFSession()
        pdf.document = DocumentPDF(pdfPath: "/tmp/test.pdf", sourceRoot: "/tmp/source", format: .latex, synchronized: true, anchors: [], warnings: [])
        XCTAssertTrue(pdf.canNavigate)
        pdf.building = true; XCTAssertFalse(pdf.canNavigate)
        pdf.building = false; pdf.error = "failure"; XCTAssertFalse(pdf.canNavigate)
        pdf.reset(); XCTAssertFalse(pdf.canNavigate); XCTAssertNil(pdf.document)
    }

    @MainActor func testDefaultShortcutsAreUniqueAndCanBeReassignedClearedAndRestored() throws {
        let suite = "ShortcutTests-\(UUID())", defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(suite)
        defer { defaults.removePersistentDomain(forName: suite); try? FileManager.default.removeItem(at: directory) }
        let prefs = ShortcutPreferences(defaults: defaults, directory: directory)
        XCTAssertEqual(Set(AppCommand.allCases.map { prefs.binding($0) }).count, AppCommand.allCases.count)
        for command in AppCommand.allCases { XCTAssertNil(prefs.validation(prefs.binding(command), for: command), command.title) }
        XCTAssertNotNil(prefs.set(ShortcutBinding("o"), for: .edit))
        XCTAssertEqual(prefs.binding(.edit), AppCommand.edit.defaultShortcut)
        XCTAssertNotNil(prefs.set(ShortcutBinding("q"), for: .edit))
        XCTAssertNotNil(prefs.set(ShortcutBinding("c"), for: .edit))
        XCTAssertNotNil(prefs.set(ShortcutBinding("x", command: false), for: .edit))
        let newBinding = ShortcutBinding("k", shift: true)
        XCTAssertNil(prefs.set(newBinding, for: .togglePDF))
        XCTAssertEqual(ShortcutPreferences(defaults: defaults, directory: directory).binding(.togglePDF), newBinding)
        XCTAssertNil(prefs.set(.unassigned, for: .togglePDF))
        XCTAssertNil(ShortcutPreferences(defaults: defaults, directory: directory).binding(.togglePDF).shortcut)
        prefs.reset()
        XCTAssertEqual(prefs.binding(.togglePDF), AppCommand.togglePDF.defaultShortcut)
        XCTAssertEqual(ShortcutPreferences(defaults: defaults, directory: directory).binding(.togglePDF), AppCommand.togglePDF.defaultShortcut)
    }

    @MainActor func testFiguresKeepOriginalSVGSizeAndHaveTheirOwnCreationMode() throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        store.format = .typst; store.newFigure()
        XCTAssertEqual(store.editor?.kind, "figure")
        XCTAssertTrue(store.editor?.tex.contains("cetz.canvas") == true)
        var figure = try XCTUnwrap(store.editor)
        figure.status = "ready"; figure.sourceFontSize = nil
        let svg = base.appendingPathComponent("drawing.svg")
        try Data(##"<svg width="100pt" height="80pt" viewBox="0 0 100 80"><path fill="#3478d4" d="M0 0L1 1"/></svg>"##.utf8).write(to: svg)
        figure.svgPath = svg.path
        store.cards = [figure]; store.exportFontSize = 72
        XCTAssertTrue(store.canExport(figure))
        XCTAssertEqual(try store.svgForExport(figure), svg)
        XCTAssertEqual(store.allItemsTitle, "All items")
        store.kind = "figure"; XCTAssertEqual(store.filtered.count, 1)
        store.format = .latex; store.newFigure()
        XCTAssertTrue(store.editor?.tex.contains("tikzpicture") == true)
    }

    func testRecorderNormalizesShiftedLettersAndSymbols() throws {
        func event(_ text: String, _ flags: NSEvent.ModifierFlags) throws -> NSEvent {
            try XCTUnwrap(NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0, windowNumber: 0, context: nil, characters: text, charactersIgnoringModifiers: text, isARepeat: false, keyCode: 0))
        }
        XCTAssertEqual(ShortcutBinding.from(try event("K", [.command, .shift])), ShortcutBinding("k", shift: true))
        XCTAssertEqual(ShortcutBinding.from(try event("+", [.command, .shift])), AppCommand.zoomIn.defaultShortcut)
        XCTAssertEqual(AppCommand.zoomIn.defaultShortcut.shortcut?.key, KeyEquivalent("+"))
        XCTAssertEqual(AppCommand.zoomIn.defaultShortcut.shortcut?.modifiers, .command)
        XCTAssertEqual(ShortcutBinding.from(try event("{", [.command, .shift])), ShortcutBinding("[", shift: true))
    }
}
