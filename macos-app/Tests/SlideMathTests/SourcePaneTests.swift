import XCTest
@testable import SlideMath

final class SourcePaneTests: XCTestCase {
    @MainActor func testNestedSplitsKeepIndependentTabsAndCloseOnlyChosenPane() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("PaneTests-\(UUID())")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        for name in ["main.typ", "symbols.typ"] { try Data(name.utf8).write(to: root.appendingPathComponent(name)) }
        let files = ProjectFiles(support: root.appendingPathComponent("support"), editorPreferences: manualEditorPreferences())
        files.configure(root: root, main: "main.typ", format: .typst)
        files.open("symbols.typ"); files.select("main.typ")
        files.update("main.typ", text: "unsaved")
        files.splitSourcePane(.right)
        XCTAssertEqual(files.currentPath, "main.typ")
        files.open("symbols.typ"); files.select("main.typ")
        files.cycleSourceTab(1); XCTAssertEqual(files.currentPath, "symbols.typ")
        XCTAssertEqual(files.path(in: 0), "main.typ")
        files.splitSourcePane(.down)
        XCTAssertEqual(files.sourceLayout, .split(.right, .pane(0), .split(.down, .pane(1), .pane(2))))
        files.open("main.typ"); files.select("symbols.typ")
        files.cycleSourceTab(1); XCTAssertEqual(files.currentPath, "main.typ")
        files.cycleSourceTab(-1); XCTAssertEqual(files.currentPath, "symbols.typ")
        XCTAssertFalse(files.controller("main.typ", pane: 0) === files.controller("main.typ", pane: 2))
        files.closeSourcePane(1)
        XCTAssertEqual(files.sourceLayout, .split(.right, .pane(0), .pane(2)))
        XCTAssertEqual(files.activeSourcePane, 2)
        files.closeSourcePane(0); XCTAssertEqual(files.sourceLayout, .pane(2))
        files.closeSourcePane(2); XCTAssertEqual(files.sourceLayout, .pane(2))
        files.select("main.typ"); XCTAssertEqual(files.current?.text, "unsaved")
        files.saveCurrent()
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("main.typ")), "unsaved")
    }

    func testLaTeXOutlineHierarchyCommentsAndCRLFLines() {
        let source = "% \\section{Hidden}\r\n\\section[Short]{Introduction}\r\n\\subsection*{A \\textbf{bold} title}\r\n\\begin{verbatim}\r\n\\section{Ignore}\r\n\\end{verbatim}\r\n\\section{Results}"
        let headings = SourceOutline.headings(source, file: "main.tex", format: .latex)
        XCTAssertEqual(headings.map(\.title), ["Introduction", "A bold title", "Results"])
        XCTAssertEqual(headings.map(\.line), [2, 3, 7])
        XCTAssertEqual(headings.map { $0.path.count }, [1, 2, 1])
    }

    func testTypstOutlineSkipsCommentsAndFencesAndTracksUnsavedSource() {
        let source = "= Intro <intro>\n/*\n= Hidden\n*/\n== Detail\n```typ\n= Code\n```\n= Results // note"
        let headings = SourceOutline.headings(source, file: "main.typ", format: .typst)
        XCTAssertEqual(headings.map(\.title), ["Intro", "Detail", "Results"])
        XCTAssertEqual(headings.map(\.line), [1, 5, 9])
        XCTAssertEqual(headings.map { $0.path.count }, [1, 2, 1])
    }
}

import SwiftUI
import AppKit

extension SourcePaneTests {
    @MainActor func testRetainedTextViewDoesNotRetainClosedProject() throws {
        _ = NSApplication.shared
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("EditorLifetime-\(UUID())")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        try Data("$ x $".utf8).write(to: root.appendingPathComponent("main.typ"))
        weak var releasedProject: ProjectFiles?
        var retainedEditor: SourceEditorController?
        autoreleasepool {
            let files = ProjectFiles(support: root.appendingPathComponent("support"), editorPreferences: manualEditorPreferences())
            files.configure(root: root, main: "main.typ", format: .typst)
            let hosting = NSHostingView(rootView: ProjectSourcePane(files: files, active: true)
                .environmentObject(LibraryStore(support: root.appendingPathComponent("support"))))
            let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 800, height: 600), styleMask: .borderless, backing: .buffered, defer: true)
            window.contentView = hosting
            hosting.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(0.1))
            retainedEditor = files.controller("main.typ")
            releasedProject = files
            window.contentView = nil
        }
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        XCTAssertNotNil(retainedEditor?.view)
        XCTAssertNil(releasedProject, "Text change and focus callbacks must not keep a closed project alive.")
    }

    @MainActor func testRenderedNestedPanesPreserveEditorScrollViewAndSelection() throws {
        _ = NSApplication.shared
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("PaneViewTests-\(UUID())")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        try Data("= Intro\n$ x^2 $\n".utf8).write(to: root.appendingPathComponent("main.typ"))
        let files = ProjectFiles(support: root.appendingPathComponent("support"), editorPreferences: manualEditorPreferences())
        files.configure(root: root, main: "main.typ", format: .typst)
        let hosting = NSHostingView(rootView: ProjectSourcePane(files: files, active: true)
            .environmentObject(LibraryStore(support: root.appendingPathComponent("support"))))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1200, height: 850), styleMask: .borderless, backing: .buffered, defer: true)
        window.contentView = hosting
        defer { window.contentView = nil }
        func settle() {
            hosting.layoutSubtreeIfNeeded()
            RunLoop.main.run(until: Date().addingTimeInterval(0.1))
            hosting.layoutSubtreeIfNeeded()
        }
        settle()
        let editor = try XCTUnwrap(files.controller("main.typ", pane: 0).view)
        let scroll = try XCTUnwrap(editor.enclosingScrollView)
        editor.setSelectedRange(NSRange(location: 2, length: 5))
        editor.insertText("Heading", replacementRange: editor.selectedRange())
        editor.breakUndoCoalescing()
        editor.setSelectedRange(NSRange(location: 2, length: 7))
        settle()
        files.splitSourcePane(.right); settle()
        files.splitSourcePane(.down); settle()
        XCTAssertTrue(files.controller("main.typ", pane: 0).view === editor)
        XCTAssertTrue(editor.enclosingScrollView === scroll)
        XCTAssertEqual(editor.selectedRange(), NSRange(location: 2, length: 7))
        let rightTop = try XCTUnwrap(files.controller("main.typ", pane: 1).view?.enclosingScrollView)
        let rightBottom = try XCTUnwrap(files.controller("main.typ", pane: 2).view?.enclosingScrollView)
        for view in [scroll, rightTop, rightBottom] {
            XCTAssertFalse(view.isHiddenOrHasHiddenAncestor)
            XCTAssertGreaterThan(view.bounds.width, 100)
            XCTAssertGreaterThan(view.bounds.height, 60)
        }
        let leftFrame = scroll.convert(scroll.bounds, to: hosting)
        let topFrame = rightTop.convert(rightTop.bounds, to: hosting)
        let bottomFrame = rightBottom.convert(rightBottom.bounds, to: hosting)
        XCTAssertGreaterThan(topFrame.minX, leftFrame.maxX)
        XCTAssertEqual(topFrame.minX, bottomFrame.minX, accuracy: 1)
        XCTAssertGreaterThan(abs(topFrame.midY - bottomFrame.midY), 100)
        files.closeSourcePane(1); settle()
        XCTAssertTrue(files.controller("main.typ", pane: 0).view === editor)
        XCTAssertFalse(scroll.isHiddenOrHasHiddenAncestor)
        XCTAssertTrue(editor.undoManager?.canUndo == true)
        editor.undoManager?.undo()
        settle()
        XCTAssertEqual(files.buffers.first?.text, "= Intro\n$ x^2 $\n")
    }

    @MainActor func testSourceTabAndBothSplitShortcutsAreCustomizable() {
        let suite = "SourceShortcuts-\(UUID())", defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let shortcuts = ShortcutPreferences(defaults: defaults)
        XCTAssertEqual(shortcuts.binding(.nextSourceTab), ShortcutBinding("\t", command: false, control: true))
        XCTAssertEqual(shortcuts.binding(.previousSourceTab), ShortcutBinding("\t", command: false, shift: true, control: true))
        XCTAssertNil(shortcuts.validation(shortcuts.binding(.nextSourceTab), for: .nextSourceTab))
        XCTAssertNil(shortcuts.validation(shortcuts.binding(.previousSourceTab), for: .previousSourceTab))
        XCTAssertNil(shortcuts.validation(shortcuts.binding(.splitSourceDown), for: .splitSourceDown))
        XCTAssertNil(shortcuts.set(ShortcutBinding("j", option: true), for: .splitSourceDown))
        XCTAssertEqual(ShortcutPreferences(defaults: defaults).binding(.splitSourceDown), ShortcutBinding("j", option: true))
    }
}

extension SourcePaneTests {
    @MainActor func testDragEdgesMoveBetweenPanesAndClosingDirtyTabs() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("TabTests-\(UUID())").resolvingSymlinksInPath()
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        try Data("original".utf8).write(to: root.appendingPathComponent("main.typ"))
        try Data("other".utf8).write(to: root.appendingPathComponent("other.typ"))
        let files = ProjectFiles(support: root.appendingPathComponent("support"), editorPreferences: manualEditorPreferences())
        files.configure(root: root, main: "main.typ", format: .typst)
        files.open("other.typ"); files.select("main.typ")
        files.update("main.typ", text: "draft")
        let payload = SourceTabDrag(project: root.path, pane: 0, path: "main.typ")
        XCTAssertTrue(files.dropSourceTab(payload, onto: 0, edge: .top))
        XCTAssertEqual(files.sourceLayout, .split(.down, .pane(1), .pane(0)))
        XCTAssertEqual(files.paneTabs[1], ["main.typ"])
        files.requestCloseTab("main.typ", pane: 1)
        XCTAssertNil(files.pendingTabClose)
        XCTAssertEqual(files.sourceLayout, .pane(0))
        XCTAssertEqual(files.current?.text, "draft")
        XCTAssertTrue(files.dropSourceTab(payload, onto: 0, edge: .left))
        XCTAssertEqual(files.sourceLayout, .split(.right, .pane(2), .pane(0)))
        let other = SourceTabDrag(project: root.path, pane: 0, path: "other.typ")
        XCTAssertTrue(files.dropSourceTab(other, onto: 2, edge: .center))
        XCTAssertEqual(files.paneTabs[0], ["main.typ"])
        XCTAssertEqual(files.paneTabs[2], ["main.typ", "other.typ"])
        XCTAssertFalse(files.dropSourceTab(SourceTabDrag(project: "/another-project", pane: 0, path: "main.typ"), onto: 2, edge: .bottom))
        files.requestCloseTab("main.typ", pane: 0)
        XCTAssertNil(files.pendingTabClose)
        files.requestCloseTab("main.typ", pane: 2)
        XCTAssertEqual(files.pendingTabClose?.path, "main.typ")
        files.pendingTabClose = nil // Cancel leaves the draft and tab intact.
        XCTAssertEqual(files.tabs(in: 2).first?.text, "draft")
        files.requestCloseTab("main.typ", pane: 2); files.confirmCloseTab(saveChanges: true)
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("main.typ")), "draft")
        XCTAssertEqual(files.paneTabs[2], ["other.typ"])
        files.update("other.typ", text: "discard me")
        files.requestCloseTab("other.typ", pane: 2); files.confirmCloseTab(saveChanges: false)
        XCTAssertTrue(files.buffers.isEmpty)
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("other.typ")), "other")
        XCTAssertNil(files.currentPath)
    }

    func testDragZoneUsesNearestEdgeAndCenterHasNoSplit() {
        let size = CGSize(width: 600, height: 400)
        XCTAssertEqual(SourceDropZone.at(CGPoint(x: 300, y: 20), size: size), .top)
        XCTAssertEqual(SourceDropZone.at(CGPoint(x: 300, y: 380), size: size), .bottom)
        XCTAssertEqual(SourceDropZone.at(CGPoint(x: 10, y: 200), size: size), .left)
        XCTAssertEqual(SourceDropZone.at(CGPoint(x: 590, y: 200), size: size), .right)
        XCTAssertEqual(SourceDropZone.at(CGPoint(x: 300, y: 200), size: size), .center)
    }
}
