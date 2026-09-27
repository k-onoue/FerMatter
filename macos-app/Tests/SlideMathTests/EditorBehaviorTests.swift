import XCTest
import AppKit
@testable import SlideMath

@MainActor func manualEditorPreferences() -> EditorPreferences {
    let name = "EditorTests-\(UUID())"
    let defaults = UserDefaults(suiteName: name)!
    let preferences = EditorPreferences(defaults: defaults)
    preferences.saveMode = .manual
    defaults.removePersistentDomain(forName: name)
    return preferences
}

final class EditorBehaviorTests: XCTestCase {
    @MainActor private func editor(_ text: String, caret: Int = 0) -> AssistedTextView {
        _ = NSApplication.shared
        let view = AssistedTextView(); view.allowsUndo = true
        view.string = text; view.setSelectedRange(NSRange(location: caret, length: 0))
        return view
    }

    @MainActor func testRepeatedCommandDeleteJoinsPreviousLineAndUndoPreservesUnicodeCRLF() {
        let view = editor("😀 alpha\r\n  beta", caret: 16)
        view.setSelectedRange(NSRange(location: (view.string as NSString).length, length: 0))
        view.deleteToBeginningOfLine(nil)
        XCTAssertEqual(view.string, "😀 alpha\r\n")
        RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        view.deleteToBeginningOfLine(nil)
        XCTAssertEqual(view.string, "😀 alpha")
        view.undoManager?.undo(); XCTAssertEqual(view.string, "😀 alpha\r\n")
        view.undoManager?.undo(); XCTAssertEqual(view.string, "😀 alpha\r\n  beta")
    }

    @MainActor func testUndoRedoShortcutsUpdateBindingForTypingCompletionAndLineEdits() throws {
        let view = editor("", caret: 0)
        var boundText = ""
        view.onSourceChange = { boundText = $0; view.highlight() }
        defer { view.onSourceChange = nil }
        func shortcut(redo: Bool = false) throws {
            let event = try XCTUnwrap(NSEvent.keyEvent(with: .keyDown, location: .zero,
                modifierFlags: redo ? [.command, .shift] : .command, timestamp: 0,
                windowNumber: 0, context: nil, characters: redo ? "Z" : "z",
                charactersIgnoringModifiers: redo ? "Z" : "z", isARepeat: false, keyCode: 6))
            XCTAssertTrue(view.handleSourceKey(event))
        }
        view.insertText("\\f", replacementRange: view.selectedRange())
        view.breakUndoCoalescing()
        RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        try shortcut(); XCTAssertEqual(boundText, "")
        try shortcut(redo: true); XCTAssertEqual(boundText, "\\f")
        let completion = try XCTUnwrap(SourceCompletion.catalog(.latex, context: .document).first { $0.trigger == "\\frac" })
        view.applyCompletion(completion, replacing: NSRange(location: 0, length: 2))
        let completed = view.string
        RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        try shortcut(); XCTAssertEqual(boundText, "\\f")
        try shortcut(redo: true); XCTAssertEqual(boundText, completed)
        view.duplicateSourceLines(down: true)
        let duplicated = view.string
        RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        try shortcut(); XCTAssertEqual(boundText, completed)
        try shortcut(redo: true); XCTAssertEqual(boundText, duplicated)
    }

    @MainActor func testUndoRedoMenuActionsUseTheEditorsIndependentHistory() {
        let first = editor(""), second = editor("")
        first.insertText("first", replacementRange: first.selectedRange()); first.breakUndoCoalescing()
        second.insertText("second", replacementRange: second.selectedRange()); second.breakUndoCoalescing()
        RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        let undo = NSMenuItem(title: "Undo", action: NSSelectorFromString("undo:"), keyEquivalent: "z")
        let redo = NSMenuItem(title: "Redo", action: NSSelectorFromString("redo:"), keyEquivalent: "Z")
        XCTAssertTrue(first.validateMenuItem(undo))
        XCTAssertFalse(first.validateMenuItem(redo))
        XCTAssertTrue(NSApp.sendAction(undo.action!, to: first, from: undo))
        XCTAssertEqual(first.string, ""); XCTAssertEqual(second.string, "second")
        XCTAssertFalse(first.validateMenuItem(undo))
        XCTAssertTrue(first.validateMenuItem(redo))
        XCTAssertTrue(NSApp.sendAction(redo.action!, to: first, from: redo))
        XCTAssertEqual(first.string, "first"); XCTAssertEqual(second.string, "second")
        first.isEditable = false
        XCTAssertFalse(first.validateMenuItem(undo))
    }

    @MainActor func testCopyCutAndPasteWholeLineWithoutSelection() {
        let pasteboard = NSPasteboard.general
        let original = pasteboard.pasteboardItems?.map { item in item.types.compactMap { type in item.data(forType: type).map { (type, $0) } } } ?? []
        defer {
            pasteboard.clearContents()
            pasteboard.writeObjects(original.map { data in let item = NSPasteboardItem(); data.forEach { item.setData($0.1, forType: $0.0) }; return item })
        }
        let view = editor("one\ntwo", caret: 5)
        view.copy(nil)
        XCTAssertEqual(pasteboard.string(forType: .string), "two\n")
        view.setSelectedRange(NSRange(location: 1, length: 0)); view.paste(nil)
        XCTAssertEqual(view.string, "two\none\ntwo")
        RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        view.cut(nil); XCTAssertEqual(view.string, "two\ntwo")
        view.undoManager?.undo(); XCTAssertEqual(view.string, "two\none\ntwo")
    }

    @MainActor func testLineDuplicationMovementAndSelectedBlockBoundaries() {
        let view = editor("first\n😀 second\nlast", caret: 8)
        view.duplicateSourceLines(down: true)
        XCTAssertEqual(view.string, "first\n😀 second\n😀 second\nlast")
        view.undoManager?.undo(); XCTAssertEqual(view.string, "first\n😀 second\nlast")
        view.setSelectedRange(NSRange(location: 8, length: 0))
        view.moveSourceLines(down: true)
        XCTAssertEqual(view.string, "first\nlast\n😀 second")
        XCTAssertEqual(view.selectedRange().location, 13)
        view.moveSourceLines(down: false)
        XCTAssertEqual(view.string, "first\n😀 second\nlast")
        view.string = "a\r\nb\r\nc"; view.setSelectedRange(NSRange(location: 0, length: 3))
        view.duplicateSourceLines(down: true)
        XCTAssertEqual(view.string, "a\r\na\r\nb\r\nc")
        view.string = "one\nlast"; view.setSelectedRange(NSRange(location: 8, length: 0))
        view.deleteSourceLines(); XCTAssertEqual(view.string, "one")
    }

    @MainActor func testIndentCommentsAndInsertLineRetainSelectionAndUndo() {
        let view = editor("a\nb\nc")
        view.setSelectedRange(NSRange(location: 0, length: 4))
        view.insertTab(nil); XCTAssertEqual(view.string, "  a\n  b\nc")
        view.insertBacktab(nil); XCTAssertEqual(view.string, "a\nb\nc")
        view.toggleSourceComments(); XCTAssertEqual(view.string, "% a\n% b\nc")
        view.toggleSourceComments(); XCTAssertEqual(view.string, "a\nb\nc")
        view.sourceFormat = .typst; view.setSelectedRange(NSRange(location: 2, length: 0))
        view.toggleSourceComments(); XCTAssertEqual(view.string, "a\n// b\nc")
        view.string = "  last"; view.setSelectedRange(NSRange(location: 3, length: 0))
        view.insertSourceLine(above: false); XCTAssertEqual(view.string, "  last\n  ")
        view.insertSourceLine(above: true); XCTAssertEqual(view.string, "  last\n  \n  ")
    }

    @MainActor func testRulerCountsLogicalLinesAndNativeBackgroundIsTransparent() {
        let container = SourceEditorContainer()
        let view = container.scroll.documentView as! AssistedTextView
        let ruler = container.scroll.verticalRulerView as! SourceLineRuler
        view.string = "😀\r\n日本語\n"; ruler.refresh()
        XCTAssertEqual(ruler.lineStarts, [0, 4, 8])
        XCTAssertFalse(container.scroll.drawsBackground)
        XCTAssertFalse(container.scroll.contentView.drawsBackground)
        XCTAssertFalse(ruler.isOpaque)
    }

    @MainActor func testAutoSaveDebouncesManualModeAndConflicts() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("AutoSave-\(UUID())")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let url = root.appendingPathComponent("main.typ")
        try Data("original".utf8).write(to: url)
        let preferences = manualEditorPreferences(); preferences.saveMode = .afterDelay; preferences.delay = 0.03
        let files = ProjectFiles(support: root.appendingPathComponent("support"), editorPreferences: preferences)
        files.configure(root: root, main: "main.typ", format: .typst)
        var saves = 0; files.onSave = { saves += 1 }
        files.update("main.typ", text: "first"); files.update("main.typ", text: "latest")
        XCTAssertEqual(try String(contentsOf: url), "original")
        RunLoop.main.run(until: Date().addingTimeInterval(0.08))
        XCTAssertEqual(try String(contentsOf: url), "latest"); XCTAssertEqual(saves, 1)
        files.update("main.typ", text: "draft")
        try Data("external".utf8).write(to: url)
        files.flushAutoSaves()
        XCTAssertEqual(try String(contentsOf: url), "external")
        XCTAssertEqual(files.current?.text, "draft"); XCTAssertTrue(files.conflicts.contains("main.typ"))
        preferences.saveMode = .manual
        RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        files.reload("main.typ"); files.update("main.typ", text: "manual")
        files.flushAutoSaves()
        RunLoop.main.run(until: Date().addingTimeInterval(0.08))
        XCTAssertEqual(try String(contentsOf: url), "external")
        files.saveCurrent(); XCTAssertEqual(try String(contentsOf: url), "manual")
    }

    @MainActor func testPendingAutoSaveFollowsRenameAndDoesNotWriteRecoveredDrafts() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("AutoSaveRename-\(UUID())")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        try Data("original".utf8).write(to: root.appendingPathComponent("main.typ"))
        let preferences = manualEditorPreferences()
        let support = root.appendingPathComponent("support")
        let files = ProjectFiles(support: support, editorPreferences: preferences)
        files.configure(root: root, main: "main.typ", format: .typst)
        files.update("main.typ", text: "recovered"); files.persistDrafts()
        let automatic = manualEditorPreferences(); automatic.saveMode = .afterDelay
        let reopened = ProjectFiles(support: support, editorPreferences: automatic)
        reopened.configure(root: root, main: "main.typ", format: .typst); reopened.flushAutoSaves()
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("main.typ")), "original")
        reopened.update("main.typ", text: "new edit")
        try reopened.rename("main.typ", to: "renamed.typ")
        reopened.flushAutoSaves()
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("renamed.typ")), "new edit")
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("main.typ").path))
    }
}
