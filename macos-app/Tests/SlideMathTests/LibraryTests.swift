import XCTest
import CryptoKit
import AppKit
@testable import SlideMath

final class LibraryTests: XCTestCase {
    @MainActor func testShowInFinderOpensSelectedProjectInsideApplicationSupport() async throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("FinderTests-\(UUID())")
        defer { try? FileManager.default.removeItem(at: base) }
        let support = base.appendingPathComponent("Application Support/Slide Math")
        let project = support.appendingPathComponent("Sources/Quick Typst", isDirectory: true)
        try FileManager.default.createDirectory(at: project, withIntermediateDirectories: true)
        let store = LibraryStore(support: support)
        store.root = base.appendingPathComponent("Another Project").path
        var opened: [URL] = []
        store.showProjectInFinder(project.path) { opened.append($0); return true }
        XCTAssertEqual(opened, [project])
        XCTAssertNil(store.alert)
    }

    @MainActor func testShowInFinderRejectsMissingOrInvalidProjectWithoutOpeningParent() async throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("FinderTests-\(UUID())")
        defer { try? FileManager.default.removeItem(at: base) }
        try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        let file = base.appendingPathComponent("main.typ")
        try Data("$x$".utf8).write(to: file)
        let store = LibraryStore(support: base)
        for path in ["", "relative/project", file.path, base.appendingPathComponent("Missing Project").path] {
            store.alert = nil
            store.showProjectInFinder(path) { _ in XCTFail("Must not open Finder for \(path)"); return true }
            XCTAssertNotNil(store.alert)
        }
    }

    @MainActor func testShowInFinderReportsOpenFailure() async throws {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("FinderTests-\(UUID())")
        defer { try? FileManager.default.removeItem(at: base) }
        try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        let store = LibraryStore(support: base)
        store.showProjectInFinder(base.path) { _ in false }
        XCTAssertEqual(store.alert, "Could not open the project folder in Finder: \(base.path)")
    }

    @MainActor func testPerItemOutputSizeOverridesDefaultAndSurvivesSavingPreview() async throws {
        let (base, root, original) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        store.load(root, rescan: false); store.stop(); store.format = .typst
        store.sourceFontSize = 10; store.exportFontSize = 20
        try Data(#"<svg width="100pt" height="20pt" viewBox="0 0 100 20"/>"#.utf8).write(to: URL(fileURLWithPath: original.svgPath!))
        store.setExportFontSize(30, for: original)
        let card = try XCTUnwrap(store.cards.first)
        XCTAssertEqual(try SVGExporter.pointSize(Data(contentsOf: store.svgForExport(card))).width, 300)
        XCTAssertEqual(try SVGExporter.pointSize(Data(contentsOf: store.svgForExport(original))).width, 200)
        store.beginEdit(card)
        var draft = try XCTUnwrap(store.editor)
        store.preview = draft // The rendered preview can predate the size change.
        draft.exportFontSize = 48
        store.saveDraft(draft)
        XCTAssertEqual(store.selected?.exportFontSize, 48)
        let reopened = LibraryStore(support: base)
        reopened.load(root, rescan: false); reopened.stop()
        XCTAssertEqual(reopened.cards.first?.exportFontSize, 30)
        XCTAssertEqual(reopened.cards.last?.exportFontSize, 48)
        store.setExportFontSize(0, for: card)
        XCTAssertEqual(try store.svgForExport(store.cards[0]).path, original.svgPath)
        store.setExportFontSize(nil, for: card)
        XCTAssertEqual(store.exportSize(for: store.cards[0]), 20)
        var figure = card; figure.kind = "figure"
        XCTAssertEqual(try SVGExporter.pointSize(Data(contentsOf: store.svgForExport(figure))).width, 300)
        figure.exportFontSize = nil
        XCTAssertEqual(try store.svgForExport(figure).path, original.svgPath)
    }

    @MainActor private func fixture() throws -> (URL, URL, Equation) {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("SlideMathTests-\(UUID())")
        let root = base.appendingPathComponent("Manuscript")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let image = base.appendingPathComponent("cached.png")
        let svg = base.appendingPathComponent("cached.svg")
        try Data().write(to: image); try Data().write(to: svg)
        let card = Equation(id: "source-original", name: "Original", tex: "x=1", kind: "block", origin: "source", sourceOrder: 0, status: "ready", svgPath: svg.path, pngPath: image.path)
        try writeLegacy(root: root, support: base, cards: [card])
        return (base, root, card)
    }

    private func writeLegacy(root: URL, support: URL, cards: [Equation]) throws {
        let key = SHA256.hash(data: Data(root.path.utf8)).map { String(format: "%02x", $0) }.joined().prefix(24)
        let directory = support.appendingPathComponent("Projects/\(key)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let legacy = SavedProject(root: root.path, main: "main.tex", engine: "pdflatex", cards: cards, customOrder: cards.map(\.id))
        try JSONEncoder().encode(legacy).write(to: directory.appendingPathComponent("project.json"))
    }

    @MainActor func testLegacyProjectAppearsOnHomeAndOpensWithoutDataLoss() async throws {
        let (base, root, card) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        XCTAssertTrue(store.showingProjects)
        XCTAssertEqual(store.projects.map(\.count), [1])
        store.load(root, rescan: false)
        XCTAssertEqual(store.cards, [card])
        XCTAssertTrue(store.sections.isEmpty)
        XCTAssertFalse(store.busy)
        XCTAssertFalse(store.showingProjects)
        XCTAssertEqual(store.format, .latex)
        XCTAssertEqual(store.exportFormat, .svg)
        XCTAssertEqual(store.projects.first?.format, .latex)
    }

    @MainActor func testTypstLibraryRestoresContextAndScalesUsingCardFontSize() async throws {
        let (base, root, original) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        store.load(root, rescan: false)
        store.stop()
        store.format = .typst; store.main = "main.typ"; store.engine = "project"; store.hasRC = false
        store.cards[0].typstContext = [TypstContextFrame(file: "main.typ", offset: 42)]
        store.cards[0].sourceFontSize = 20
        store.sourceFontSize = 11; store.exportFontSize = 40
        let svg = Data(#"<svg width="100pt" height="20pt" viewBox="0 0 100 20"><path d="M0 0L1 1"/></svg>"#.utf8)
        try svg.write(to: URL(fileURLWithPath: original.svgPath!))
        store.addSection("Typst section")
        store.assign(store.cards[0], to: store.sections[0].id)
        store.saveNow()
        let reopened = LibraryStore(support: base)
        reopened.load(root, rescan: false)
        XCTAssertEqual(reopened.format, .typst)
        XCTAssertEqual(reopened.projects.first?.format, .typst)
        XCTAssertTrue(reopened.canRender)
        let card = try XCTUnwrap(reopened.cards.first)
        XCTAssertEqual(card.typstContext?.first?.offset, 42)
        XCTAssertEqual(card.sourceFontSize, 20)
        XCTAssertEqual(reopened.sectionName(for: card), "Typst section")
        let exported = try reopened.svgForExport(card)
        let document = try XMLDocument(contentsOf: exported)
        XCTAssertEqual(document.rootElement()?.attribute(forName: "width")?.stringValue, "200.000000pt")
        reopened.beginEdit(card)
        XCTAssertEqual(reopened.editor?.typstContext, card.typstContext)
        XCTAssertEqual(reopened.editor?.origin, "custom")
        // Opening a different format in the same saved root must not merge or overwrite it.
        let otherMain = root.appendingPathComponent("main.tex")
        try Data().write(to: otherMain)
        reopened.load(otherMain, rescan: false)
        XCTAssertNotNil(reopened.alert)
        XCTAssertEqual(reopened.format, .typst)
        XCTAssertEqual(reopened.cards, [card])
    }

    @MainActor func testDocumentOutlineFiltersDescendantsAndPreservesManualLabels() async throws {
        let (base, root, original) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        store.load(root, rescan: false); store.stop()
        let parent = DocumentHeading(id: "p", title: "Methods", level: 2, file: "main.tex", line: 10, path: ["p"])
        let child = DocumentHeading(id: "c", title: "Model", level: 3, file: "model.tex", line: 1, path: ["p", "c"])
        let other = DocumentHeading(id: "o", title: "Results", level: 2, file: "main.tex", line: 20, path: ["o"])
        store.outline = [parent, child, other]
        var a = original; a.id = "a"; a.outlinePath = parent.path
        var b = original; b.id = "b"; b.outlinePath = child.path; b.sourceOrder = 1
        var c = original; c.id = "c"; c.outlinePath = other.path; c.sourceOrder = 2
        var unassigned = original; unassigned.id = "u"; unassigned.outlinePath = []; unassigned.sourceOrder = 3
        store.cards = [a, b, c, unassigned]
        store.addSection("Favorites"); store.assign(b, to: store.sections[0].id)
        store.selectSection("outline:p")
        XCTAssertEqual(store.filtered.map(\.id), ["a", "b"])
        XCTAssertEqual(store.outlineCount(parent), 2)
        XCTAssertEqual(store.outlineTitle(for: b), "Methods › Model")
        store.selectSection("outline:c")
        XCTAssertEqual(store.filtered.map(\.id), ["b"])
        XCTAssertEqual(store.sectionTitle, "Model")
        store.toggleOutline(parent)
        XCTAssertEqual(store.visibleOutline.map(\.id), ["p", "o"])
        store.selectSection("outline:none")
        XCTAssertEqual(store.filtered.map(\.id), ["u"])
        store.selectSection(store.sections[0].id)
        XCTAssertEqual(store.filtered.map(\.id), ["b"])
        store.beginEdit(b)
        XCTAssertEqual(store.editor?.outlinePath, child.path)
        store.saveNow()
        let reopened = LibraryStore(support: base)
        reopened.load(root, rescan: false)
        XCTAssertEqual(reopened.outline, [parent, child, other])
        XCTAssertEqual(reopened.collapsedOutline, ["p"])
        XCTAssertEqual(reopened.sectionName(for: b), "Favorites")
    }

    @MainActor func testSectionAssignmentRenameAndReopen() async throws {
        let (base, root, card) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        store.load(root, rescan: false)
        store.addSection("Results")
        let section = try XCTUnwrap(store.sections.first)
        store.assign(card, to: section.id)
        XCTAssertEqual(store.filtered.map(\.id), [card.id])
        store.renameSection(section, to: "Key Results")
        store.exportFontSize = 32; store.sourceFontSize = 12; store.saveNow()
        let reopened = LibraryStore(support: base)
        reopened.load(root, rescan: false)
        XCTAssertEqual(reopened.sections.first?.name, "Key Results")
        XCTAssertEqual(reopened.sectionAssignments[card.id], section.id)
        XCTAssertEqual(reopened.exportFontSize, 32)
        XCTAssertEqual(reopened.sourceFontSize, 12)
        reopened.selectSection("unsectioned")
        XCTAssertTrue(reopened.filtered.isEmpty)
        reopened.selectSection(section.id)
        XCTAssertEqual(reopened.filtered.count, 1)
    }

    @MainActor func testDeletingSectionKeepsEquationsAndClearsMembership() async throws {
        let (base, root, card) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        store.load(root, rescan: false)
        store.addSection("Temporary")
        let section = try XCTUnwrap(store.sections.first)
        store.assign(card, to: section.id)
        store.deleteSection(section)
        XCTAssertEqual(store.cards, [card])
        XCTAssertEqual(store.sectionFilter, "all")
        XCTAssertTrue(store.sectionAssignments.isEmpty)
        let reopened = LibraryStore(support: base)
        reopened.load(root, rescan: false)
        XCTAssertTrue(reopened.sections.isEmpty)
        XCTAssertEqual(reopened.cards, [card])
    }

    @MainActor func testProjectSwitchKeepsClassificationsSeparate() async throws {
        let (base, root, card) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let secondRoot = base.appendingPathComponent("Other Manuscript")
        try FileManager.default.createDirectory(at: secondRoot, withIntermediateDirectories: true)
        try writeLegacy(root: secondRoot, support: base, cards: [card])
        let store = LibraryStore(support: base)
        store.load(root, rescan: false)
        store.addSection("First project")
        store.assign(card, to: store.sections[0].id)
        store.load(secondRoot, rescan: false)
        XCTAssertTrue(store.sections.isEmpty)
        XCTAssertTrue(store.sectionAssignments.isEmpty)
        store.load(root, rescan: false)
        XCTAssertEqual(store.sections.first?.name, "First project")
        XCTAssertEqual(store.sectionAssignments.count, 1)
        store.showProjects()
        XCTAssertEqual(store.projects.count, 2)
    }

    @MainActor func testEditingSourceCreatesClassifiedCustomCopy() async throws {
        let (base, root, card) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        store.load(root, rescan: false)
        store.addSection("Results")
        let sectionID = store.sections[0].id
        store.assign(card, to: sectionID)
        store.selectSection("all")
        store.beginEdit(card)
        let draft = try XCTUnwrap(store.editor)
        store.preview = draft
        store.saveDraft(draft)
        XCTAssertEqual(store.cards.count, 2)
        XCTAssertEqual(store.cards.first, card)
        XCTAssertEqual(store.sectionAssignments[draft.id], sectionID)
        XCTAssertEqual(store.cards.last?.origin, "custom")
    }

    @MainActor func testExportFormatPersistsWithoutChangingLegacyDefault() async throws {
        let (base, root, _) = try fixture()
        defer { try? FileManager.default.removeItem(at: base) }
        let store = LibraryStore(support: base)
        store.load(root, rescan: false); store.stop()
        XCTAssertEqual(store.exportFormat, .svg)
        store.exportFormat = .png; store.saveNow()
        let reopened = LibraryStore(support: base)
        reopened.load(root, rescan: false); reopened.stop()
        XCTAssertEqual(reopened.exportFormat, .png)
    }

    @MainActor func testWindowPreferencesPersistAndClampInvisibleOpacity() async throws {
        let suite = "SlideMathTests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let prefs = WindowPreferences(defaults: defaults)
        prefs.pinned = true; prefs.appearance = "light"; prefs.opacity = 0.7
        let restored = WindowPreferences(defaults: defaults)
        XCTAssertTrue(restored.pinned)
        XCTAssertEqual(restored.appearance, "light")
        XCTAssertEqual(restored.opacity, 0.7)
        defaults.set(0, forKey: "opacity")
        XCTAssertEqual(WindowPreferences(defaults: defaults).opacity, 0.45)
    }

    @MainActor func testWindowPreferencesApplyAndResetNativeProperties() async throws {
        _ = NSApplication.shared
        let suite = "SlideMathTests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let window = NSWindow(contentRect: .zero, styleMask: .borderless, backing: .buffered, defer: true)
        let prefs = WindowPreferences(defaults: defaults)
        prefs.pinned = true; prefs.opacity = 0.75; prefs.appearance = "dark"
        prefs.apply(to: window)
        XCTAssertEqual(window.level, .floating)
        XCTAssertEqual(window.alphaValue, 1)
        XCTAssertFalse(window.isOpaque)
        XCTAssertEqual(window.backgroundColor, .clear)
        XCTAssertTrue(window.titlebarAppearsTransparent)
        XCTAssertEqual(window.appearance?.name, .darkAqua)
        prefs.pinned = false; prefs.opacity = 1; prefs.appearance = "system"
        prefs.apply(to: window)
        XCTAssertEqual(window.level, .normal)
        XCTAssertEqual(window.alphaValue, 1)
        XCTAssertNil(window.appearance)
        XCTAssertFalse(window.isVisible)
    }
}
