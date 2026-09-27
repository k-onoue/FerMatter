import XCTest
import AppKit
@testable import SlideMath

final class FileOperationsTests: XCTestCase {
    @MainActor private func fixture() throws -> (URL, ProjectFiles) {
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("FileActions-\(UUID())").resolvingSymlinksInPath()
        let root = base.appendingPathComponent("project", isDirectory: true)
        let trash = base.appendingPathComponent("trash", isDirectory: true)
        try FileManager.default.createDirectory(at: root.appendingPathComponent("parts.v1"), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: trash, withIntermediateDirectories: true)
        try Data("main".utf8).write(to: root.appendingPathComponent("main.tex"))
        try Data("part".utf8).write(to: root.appendingPathComponent("parts.v1/body.tex"))
        addTeardownBlock { try? FileManager.default.removeItem(at: base) }
        let files = ProjectFiles(support: base.appendingPathComponent("support"), editorPreferences: manualEditorPreferences())
        files.configure(root: root, main: "main.tex", format: .latex)
        files.operations = FileOperations(root: root) { source in
            let destination = trash.appendingPathComponent(UUID().uuidString + "-" + source.lastPathComponent)
            try FileManager.default.moveItem(at: source, to: destination); return destination
        }
        return (root, files)
    }

    @MainActor func testRenameRemapsDirtyBuffersEveryPaneAndSupportsUndoRedo() throws {
        let (root, files) = try fixture()
        files.open("parts.v1/body.tex"); files.update("parts.v1/body.tex", text: "unsaved")
        for _ in 0..<11 { files.splitSourcePane(.down) }
        let controller = files.controller("parts.v1/body.tex")
        var relocated: [FileRelocation] = []; files.onRelocate = { relocated = $0 }
        try files.rename("parts.v1", to: "chapters")
        XCTAssertEqual(relocated.first?.to.lastPathComponent, "chapters")
        XCTAssertEqual(files.currentPath, "chapters/body.tex")
        XCTAssertEqual(Set(files.paneSelections.values), ["chapters/body.tex"])
        XCTAssertTrue(files.controller("chapters/body.tex") === controller)
        XCTAssertEqual(files.current?.text, "unsaved")
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("chapters/body.tex")), "part")
        files.undoFileOperation(); XCTAssertNil(files.error)
        XCTAssertEqual(files.currentPath, "parts.v1/body.tex")
        files.redoFileOperation(); XCTAssertNil(files.error)
        XCTAssertEqual(files.currentPath, "chapters/body.tex")
        try files.rename("main.tex", to: "MAIN.tex")
        XCTAssertTrue(try FileManager.default.contentsOfDirectory(atPath: root.path).contains("MAIN.tex"))
        files.undoFileOperation(); XCTAssertNil(files.error)
        XCTAssertTrue(try FileManager.default.contentsOfDirectory(atPath: root.path).contains("main.tex"))
    }

    @MainActor func testDuplicateParentSelectionOnlyOnceAndUndoConflictDoesNotOverwrite() throws {
        let (root, files) = try fixture()
        files.treeSelection = ["parts.v1", "parts.v1/body.tex"]
        try files.duplicateSelection()
        XCTAssertEqual(files.treeSelection, ["parts.v1 2"])
        XCTAssertTrue(FileOperations.exists(root.appendingPathComponent("parts.v1 2/body.tex")))
        files.undoFileOperation(); XCTAssertNil(files.error)
        XCTAssertFalse(FileOperations.exists(root.appendingPathComponent("parts.v1 2")))
        try FileManager.default.createDirectory(at: root.appendingPathComponent("parts.v1 2"), withIntermediateDirectories: false)
        files.redoFileOperation(); XCTAssertNotNil(files.error)
        XCTAssertEqual(files.redoFileTitle, "Duplicate")
        try FileManager.default.removeItem(at: root.appendingPathComponent("parts.v1 2"))
        files.redoFileOperation(); XCTAssertNil(files.error)
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("parts.v1 2/body.tex")), "part")
    }

    @MainActor func testFinderClipboardCopyCutAndStaleCutGeneration() throws {
        let (root, files) = try fixture()
        let pasteboard = NSPasteboard.withUniqueName(); defer { pasteboard.releaseGlobally() }
        let external = root.deletingLastPathComponent().appendingPathComponent("external.bin")
        let data = Data([0, 1, 255]); try data.write(to: external)
        pasteboard.writeObjects([external as NSURL]); files.treeSelection = ["parts.v1"]
        try files.pasteTreeSelection(pasteboard: pasteboard)
        XCTAssertEqual(try Data(contentsOf: root.appendingPathComponent("parts.v1/external.bin")), data)
        XCTAssertEqual(try Data(contentsOf: external), data)
        files.treeSelection = ["main.tex"]; files.copyTreeSelection(cut: true, pasteboard: pasteboard)
        files.treeSelection = ["parts.v1"]; try files.pasteTreeSelection(pasteboard: pasteboard)
        XCTAssertFalse(FileOperations.exists(root.appendingPathComponent("main.tex")))
        XCTAssertEqual(files.currentPath, "parts.v1/main.tex")
        files.undoFileOperation(); XCTAssertNil(files.error)
        XCTAssertEqual(files.currentPath, "main.tex")
        files.treeSelection = ["main.tex"]; files.copyTreeSelection(cut: true, pasteboard: pasteboard)
        pasteboard.clearContents(); pasteboard.writeObjects([external as NSURL])
        files.treeSelection = []; try files.pasteTreeSelection(pasteboard: pasteboard)
        XCTAssertTrue(FileOperations.exists(root.appendingPathComponent("main.tex")))
        XCTAssertEqual(try Data(contentsOf: root.appendingPathComponent("external.bin")), data)
    }

    @MainActor func testTrashRetainsUnsavedDraftAndUndoRestoresOriginal() throws {
        let (root, files) = try fixture()
        files.update("main.tex", text: "unsaved draft")
        files.treeSelection = ["main.tex"]; try files.trashSelection()
        XCTAssertEqual(files.current?.text, "unsaved draft")
        XCTAssertTrue(files.conflicts.contains("main.tex"))
        XCTAssertFalse(FileOperations.exists(root.appendingPathComponent("main.tex")))
        XCTAssertThrowsError(try files.save("main.tex"))
        files.undoFileOperation(); XCTAssertNil(files.error)
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("main.tex")), "main")
        XCTAssertEqual(files.current?.text, "unsaved draft")
        XCTAssertFalse(files.conflicts.contains("main.tex"))
    }

    @MainActor func testBatchPreflightAndSymlinkCannotModifyOutsideProject() throws {
        let (root, files) = try fixture()
        let target = root.appendingPathComponent("new.tex")
        XCTAssertThrowsError(try files.operations!.perform([
            .copy(root.appendingPathComponent("main.tex"), target),
            .copy(root.appendingPathComponent("missing.tex"), root.appendingPathComponent("missing-copy.tex"))
        ]))
        XCTAssertFalse(FileOperations.exists(target))
        XCTAssertThrowsError(try files.transfer([root.appendingPathComponent("parts.v1")], to: "parts.v1", move: false))
        let outside = root.deletingLastPathComponent().appendingPathComponent("outside", isDirectory: true)
        try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
        try FileManager.default.createSymbolicLink(at: root.appendingPathComponent("link"), withDestinationURL: outside)
        XCTAssertThrowsError(try files.entryURL("link/test.tex"))
        XCTAssertThrowsError(try files.transfer([root.appendingPathComponent("main.tex")], to: "link", move: false))
        try files.rename("link", to: "renamed-link")
        XCTAssertTrue(FileOperations.exists(outside))
        XCTAssertTrue(FileOperations.exists(root.appendingPathComponent("renamed-link")))
    }

    @MainActor func testFailedBatchRollsBackCompletedCopies() throws {
        let (root, files) = try fixture()
        let existingFolder = root.appendingPathComponent("parts.v1")
        // A non-directory parent passes the existence check, but the copy must fail.
        let invalidTarget = root.appendingPathComponent("main.tex/child")
        XCTAssertThrowsError(try files.operations!.perform([
            .copy(existingFolder, root.appendingPathComponent("copied")),
            .copy(root.appendingPathComponent("main.tex"), invalidTarget)
        ]))
        XCTAssertFalse(FileOperations.exists(root.appendingPathComponent("copied")))
        XCTAssertEqual(try String(contentsOf: root.appendingPathComponent("main.tex")), "main")
        XCTAssertFalse(try FileManager.default.contentsOfDirectory(atPath: root.path).contains { $0.hasPrefix(".slidemath-copy-") })
    }
}
