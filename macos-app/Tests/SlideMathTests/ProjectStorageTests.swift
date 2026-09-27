import CryptoKit
import XCTest
@testable import SlideMath

final class ProjectStorageTests: XCTestCase {
    func testExistingStoragePathAndLegacyMetadataRemainCompatible() throws {
        let support = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: support) }
        let storage = ProjectStorage(support: support), root = support.appendingPathComponent("paper").path
        let key = SHA256.hash(data: Data(root.utf8)).map { String(format: "%02x", $0) }.joined().prefix(24)
        let directory = support.appendingPathComponent("Projects/\(key)", isDirectory: true)
        XCTAssertEqual(storage.directory(for: root), directory)
        XCTAssertNil(try storage.load(root: root))
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let legacy: [String: Any] = ["root": root, "main": "main.typ", "engine": "project", "cards": [], "customOrder": []]
        try JSONSerialization.data(withJSONObject: legacy).write(to: directory.appendingPathComponent("project.json"))
        let project = try XCTUnwrap(storage.load(root: root))
        XCTAssertEqual(project.resolvedFormat, .typst)
        try storage.save(project)
        XCTAssertEqual(try storage.load(root: root)?.main, "main.typ")
        XCTAssertEqual(storage.summaries().map(\.root), [root])
    }

    func testFailedEncodingAndUnreadableMetadataDoNotDestroyExistingFile() throws {
        let support = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: support) }
        let storage = ProjectStorage(support: support), root = support.appendingPathComponent("project").path
        var project = SavedProject(root: root, main: "main.tex", engine: "project", cards: [], customOrder: [])
        try storage.save(project)
        let file = storage.directory(for: root).appendingPathComponent("project.json")
        let original = try Data(contentsOf: file)
        project.exportFontSize = .nan
        XCTAssertThrowsError(try storage.save(project))
        XCTAssertEqual(try Data(contentsOf: file), original)
        let corrupt = Data("incomplete json".utf8)
        try corrupt.write(to: file)
        XCTAssertThrowsError(try storage.load(root: root))
        XCTAssertTrue(storage.summaries().isEmpty)
        XCTAssertEqual(try Data(contentsOf: file), corrupt)
    }
}
