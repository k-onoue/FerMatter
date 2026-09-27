import XCTest
@testable import SlideMath

final class ProjectFilesTests: XCTestCase {
    @MainActor private func fixture() throws -> (URL, ProjectFiles) {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("SourceEditorTests-\(UUID())")
        let root = base.appendingPathComponent("project")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("sections"), withIntermediateDirectories: true)
        try Data("\\newcommand{\\vect}[1]{\\mathbf{#1}}\n".utf8).write(to: root.appendingPathComponent("main.tex"))
        try Data("Text $x$\n".utf8).write(to: root.appendingPathComponent("sections/model.tex"))
        addTeardownBlock { try? FileManager.default.removeItem(at: base) }
        let files = ProjectFiles(support: base.appendingPathComponent("support"), editorPreferences: manualEditorPreferences())
        files.configure(root: root, main: "main.tex", format: .latex)
        return (root, files)
    }

    @MainActor func testTreeTabSwitchingAndRecoveryNeverWriteSourceUntilSave() throws {
        let (root, files) = try fixture()
        let original = try Data(contentsOf: root.appendingPathComponent("main.tex"))
        XCTAssertEqual(files.tree.map(\.name), ["sections", "main.tex"])
        let edited = files.current!.text + "\\usepackage{amsmath}\n"
        files.update("main.tex", text: edited)
        files.open("sections/model.tex")
        files.open("main.tex")
        XCTAssertEqual(files.current?.text, edited)
        XCTAssertEqual(try Data(contentsOf: root.appendingPathComponent("main.tex")), original)
        files.persistDrafts()
        let reopened = ProjectFiles(support: root.deletingLastPathComponent().appendingPathComponent("support"), editorPreferences: manualEditorPreferences())
        reopened.configure(root: root, main: "main.tex", format: .latex)
        XCTAssertEqual(reopened.current?.text, edited)
        XCTAssertEqual(reopened.dirtyCount, 1)
        var saved = false; reopened.onSave = { saved = true }
        try reopened.save("main.tex")
        XCTAssertTrue(saved)
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("main.tex")), edited)
        XCTAssertEqual(reopened.dirtyCount, 0)
    }

    @MainActor func testExternalEditConflictPreservesBothVersionsAndReloadsCleanBuffers() throws {
        let (root, files) = try fixture()
        let main = root.appendingPathComponent("main.tex")
        try Data("external version".utf8).write(to: main)
        files.refresh(); XCTAssertEqual(files.current?.text, "external version")
        files.update("main.tex", text: "local edits")
        try Data("new external version".utf8).write(to: main)
        files.refresh()
        XCTAssertTrue(files.conflicts.contains("main.tex"))
        XCTAssertThrowsError(try files.save("main.tex"))
        XCTAssertEqual(try String(contentsOf: main), "new external version")
        XCTAssertEqual(files.current?.text, "local edits")
        files.reload("main.tex")
        XCTAssertEqual(files.current?.text, "new external version")
        XCTAssertFalse(files.conflicts.contains("main.tex"))
    }

    @MainActor func testCreationAndWritesCannotEscapeProjectOrOverwriteFiles() throws {
        let (root, files) = try fixture()
        XCTAssertThrowsError(try files.create(name: "main.tex", parent: "", directory: false))
        XCTAssertThrowsError(try files.create(name: "../escape.tex", parent: "", directory: false))
        let outside = root.deletingLastPathComponent().appendingPathComponent("outside.tex")
        try Data("untouched".utf8).write(to: outside)
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("link.tex"), withDestinationURL: outside)
        files.open("link.tex"); XCTAssertNotNil(files.error)
        XCTAssertThrowsError(try files.resolved("../outside.tex"))
        try files.create(name: "macros", parent: "", directory: true)
        try files.create(name: "symbols.tex", parent: "macros", directory: false)
        files.update("macros/symbols.tex", text: "\\newcommand{\\RR}{\\mathbb{R}}")
        XCTAssertTrue(files.completions.contains { $0.trigger == "\\RR" })
        try files.save("macros/symbols.tex")
        XCTAssertEqual(try String(contentsOf: outside), "untouched")
    }

    @MainActor func testUTF8BOMAndCRLFArePreservedAndBinaryIsRejected() throws {
        let data = Data([0xef, 0xbb, 0xbf]) + Data("α\r\nβ\r\n".utf8)
        var buffer = try ProjectFiles.buffer(path: "main.typ", data: data)
        XCTAssertFalse(buffer.dirty)
        buffer.text += "γ\n"
        XCTAssertEqual(buffer.encoded, data + Data("γ\r\n".utf8))
        XCTAssertThrowsError(try ProjectFiles.buffer(path: "binary", data: Data([0xff, 0x00])))
    }

    @MainActor func testDraftsStayWithTheirProjectAndDeletedFilesAreNotRecreated() throws {
        let (root, files) = try fixture()
        files.update("main.tex", text: "unsaved original project")
        let other = root.deletingLastPathComponent().appendingPathComponent("other")
        try FileManager.default.createDirectory(at: other, withIntermediateDirectories: true)
        try Data("#let f(x) = x".utf8).write(to: other.appendingPathComponent("main.typ"))
        files.configure(root: other, main: "main.typ", format: .typst)
        XCTAssertEqual(files.current?.text, "#let f(x) = x")
        files.configure(root: root, main: "main.tex", format: .latex)
        XCTAssertEqual(files.current?.text, "unsaved original project")
        try FileManager.default.removeItem(at: root.appendingPathComponent("main.tex"))
        XCTAssertThrowsError(try files.save("main.tex"))
        XCTAssertFalse(FileManager.default.fileExists(atPath: root.appendingPathComponent("main.tex").path))
    }
}
