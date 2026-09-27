import XCTest
import AppKit
import SwiftUI
@testable import SlideMath

private final class PreviewWorker: WorkerRunning {
    var cancelled = false
    var request: [String: Any] = [:]
    var receive: ((WorkerEvent) -> Void)?
    var finished: (() -> Void)?
    func run(_ request: [String: Any], receive: @escaping (WorkerEvent) -> Void, finished: @escaping () -> Void) {
        self.request = request; self.receive = receive; self.finished = finished
    }
    func cancel() { cancelled = true }
}

final class SourcePreviewTests: XCTestCase {
    private var input: SourcePreviewInput {
        SourcePreviewInput(root: "/tmp/preview-project", main: "main.typ", engine: "project", format: .typst,
                           file: "part.typ", cache: "/tmp/preview-cache", sources: ["part.typ": "$x$"], diskRevision: 1)
    }

    @MainActor func testCaretWithinEquationReusesRenderAndChangedBufferInvalidatesIt() {
        let first = PreviewWorker(), second = PreviewWorker()
        var workers = [first, second]
        let model = SourcePreviewModel(session: WorkerSession { workers.removeFirst() })
        model.update(input, offset: 10, delay: 0)
        var card = Equation.draft(); card.id = "source-test"; card.tex = "x"
        first.receive?(WorkerEvent(event: "source_preview", cards: [card], previewStart: 9, previewEnd: 14))
        first.receive?(WorkerEvent(event: "card", id: card.id, status: "ready", svgPath: "/tmp/x.svg", pngPath: "/tmp/x.png"))
        first.finished?()
        model.update(input, offset: 12, delay: 0)
        XCTAssertEqual(workers.count, 1)
        XCTAssertEqual(model.card?.status, "ready")
        var updated = input; updated.sources["part.typ"] = "$y$"
        model.update(updated, offset: 12, delay: 0)
        XCTAssertNil(model.card)
        XCTAssertEqual(second.request["sources"] as? [String: String], updated.sources)
        XCTAssertEqual(second.request["offset"] as? Int, 12)
    }

    @MainActor func testMovingOutsideEquationAndClosingRejectsLateResult() {
        let first = PreviewWorker(), second = PreviewWorker()
        var workers = [first, second]
        let model = SourcePreviewModel(session: WorkerSession { workers.removeFirst() })
        model.update(input, offset: 10, delay: 0)
        var card = Equation.draft(); card.id = "first"
        first.receive?(WorkerEvent(event: "source_preview", cards: [card], previewStart: 9, previewEnd: 14))
        model.update(input, offset: 20, delay: 0)
        XCTAssertTrue(first.cancelled)
        first.receive?(WorkerEvent(event: "card", id: card.id, status: "ready"))
        first.finished?()
        XCTAssertNil(model.card); XCTAssertTrue(model.busy)
        second.receive?(WorkerEvent(event: "source_preview", cards: []))
        second.finished?()
        XCTAssertNil(model.card); XCTAssertFalse(model.busy)
        model.cancel()
        second.receive?(WorkerEvent(event: "error", message: "late"))
        XCTAssertNil(model.error)
    }

    @MainActor func testIndependentPanesDoNotCancelEachOtherAndContextChangeRerenders() {
        let first = PreviewWorker(), next = PreviewWorker(), other = PreviewWorker()
        var workers = [first, next]
        let primary = SourcePreviewModel(session: WorkerSession { workers.removeFirst() })
        let secondary = SourcePreviewModel(session: WorkerSession { other })
        primary.update(input, offset: 1, delay: 0)
        secondary.update(input, offset: 1, delay: 0)
        var changed = input; changed.sources["symbols.typ"] = "#let value = 7"
        primary.update(changed, offset: 1, delay: 0)
        XCTAssertTrue(first.cancelled); XCTAssertFalse(other.cancelled)
        primary.cancel()
        XCTAssertTrue(next.cancelled); XCTAssertFalse(other.cancelled)
        XCTAssertTrue(secondary.busy)
    }
}


extension SourcePreviewTests {
    @MainActor func testCollapsedCaretInHostedEditorDisplaysPreviewAndHidesOutsideViewport() throws {
        _ = NSApplication.shared
        let source = "= Heading\n\n$ alpha + beta $\n" + String(repeating: "text\n", count: 100)
        let editor = SourceEditorController()
        let hosting = NSHostingView(rootView: SourceTextEditor(text: .constant(source), label: "Source preview test", format: .typst, controller: editor))
        let window = NSWindow(contentRect: NSRect(x: 100, y: 100, width: 800, height: 500), styleMask: .borderless, backing: .buffered, defer: true)
        window.contentView = hosting
        hosting.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        hosting.layoutSubtreeIfNeeded()
        let view = try XCTUnwrap(editor.view)
        let worker = PreviewWorker()
        let model = SourcePreviewModel(session: WorkerSession { worker })
        let preview = SourceMathPreviewController(model: model)
        defer { preview.hide(); window.contentView = nil }
        let position = (source as NSString).range(of: "alpha").location + 1
        view.setSelectedRange(NSRange(location: position, length: 0))
        XCTAssertTrue(window.makeFirstResponder(view))
        XCTAssertTrue(window.firstResponder === view)
        var context = input
        context.sources = [context.file: source]
        let configuration = SourcePreviewConfiguration(input: context, format: .svg, changeFormat: { _ in }, drag: { _ in NSItemProvider() }, copy: { _ in })
        preview.configure(view: view, configuration: configuration)
        RunLoop.main.run(until: Date().addingTimeInterval(0.45))
        XCTAssertEqual(worker.request["offset"] as? Int, position)
        var card = Equation.draft(); card.id = "hosted-preview"; card.tex = "alpha + beta"
        worker.receive?(WorkerEvent(event: "source_preview", cards: [card], previewStart: position - 1, previewEnd: position + 11))
        let caret = view.firstRect(forCharacterRange: view.selectedRange(), actualRange: nil)
        let visible = window.convertToScreen(view.convert(view.visibleRect, to: nil))
        XCTAssertGreaterThan(visible.width, 100)
        XCTAssertGreaterThan(visible.height, 100)
        XCTAssertTrue(window.childWindows?.isEmpty == false, "An insertion caret must be visible to the preview controller. caret=\(caret), viewport=\(visible)")
        view.enclosingScrollView?.contentView.scroll(to: NSPoint(x: 0, y: view.bounds.height - 100))
        view.enclosingScrollView?.reflectScrolledClipView(view.enclosingScrollView!.contentView)
        RunLoop.main.run(until: Date().addingTimeInterval(0.05))
        XCTAssertTrue(window.childWindows?.isEmpty != false)
    }
}

extension SourcePreviewTests {
    @MainActor func testProjectSourcePaneEnablesPreviewForFocusedTypstFile() throws {
        _ = NSApplication.shared
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("SourcePreviewWiring-\(UUID())")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let source = "= Heading\n\n$ alpha + beta $\n"
        try Data(source.utf8).write(to: root.appendingPathComponent("main.typ"))
        let store = LibraryStore(support: root.appendingPathComponent("support"))
        store.root = root.path; store.main = "main.typ"; store.format = .typst
        store.files.configure(root: root, main: "main.typ", format: .typst)
        let hosting = NSHostingView(rootView: ProjectSourcePane(files: store.files, active: true).environmentObject(store))
        let window = NSWindow(contentRect: NSRect(x: 100, y: 100, width: 800, height: 500), styleMask: .borderless, backing: .buffered, defer: true)
        window.contentView = hosting
        defer { store.files.controller("main.typ").view?.sourcePreview.hide(); window.contentView = nil }
        hosting.layoutSubtreeIfNeeded()
        RunLoop.main.run(until: Date().addingTimeInterval(0.1))
        hosting.layoutSubtreeIfNeeded()
        let view = try XCTUnwrap(store.files.controller("main.typ").view)
        XCTAssertTrue(view.sourcePreview.isEnabled)
        XCTAssertTrue(window.makeFirstResponder(view))
        let position = (source as NSString).range(of: "alpha").location + 1
        view.setSelectedRange(NSRange(location: position, length: 0))
        RunLoop.main.run(until: Date().addingTimeInterval(0.7))
        // SwiftPM's test bundle intentionally has no worker resources. The worker error still
        // travels through the complete ProjectFileEditor configuration and opens the preview.
        XCTAssertNotNil(view.sourcePreview.model.error)
        XCTAssertTrue(window.childWindows?.isEmpty == false)
        XCTAssertTrue(window.makeFirstResponder(nil))
        XCTAssertNil(view.sourcePreview.model.error)
        XCTAssertTrue(window.makeFirstResponder(view))
        RunLoop.main.run(until: Date().addingTimeInterval(0.7))
        XCTAssertNotNil(view.sourcePreview.model.error, "Refocusing the same caret must restart its preview.")
        XCTAssertTrue(window.childWindows?.isEmpty == false)
    }
}
