import CryptoKit
import Foundation

/// Library metadata only. Editable manuscript files and source draft recovery
/// are managed by ProjectFiles. Keep this path and Codable format compatible.
struct ProjectStorage {
    let support: URL

    func directory(for root: String) -> URL {
        let key = SHA256.hash(data: Data(root.utf8)).map { String(format: "%02x", $0) }.joined().prefix(24)
        return support.appendingPathComponent("Projects/\(key)", isDirectory: true)
    }

    func load(root: String) throws -> SavedProject? {
        let file = directory(for: root).appendingPathComponent("project.json")
        guard FileManager.default.fileExists(atPath: file.path) else { return nil }
        return try JSONDecoder().decode(SavedProject.self, from: Data(contentsOf: file))
    }

    func save(_ project: SavedProject) throws {
        let folder = directory(for: project.root)
        let data = try JSONEncoder().encode(project)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        try data.write(to: folder.appendingPathComponent("project.json"), options: .atomic)
    }

    func summaries() -> [ProjectSummary] {
        let folders = (try? FileManager.default.contentsOfDirectory(at: support.appendingPathComponent("Projects"), includingPropertiesForKeys: nil)) ?? []
        return folders.compactMap { folder in
            let file = folder.appendingPathComponent("project.json")
            guard let data = try? Data(contentsOf: file), let saved = try? JSONDecoder().decode(SavedProject.self, from: data) else { return nil }
            let updated = (try? file.resourceValues(forKeys: [.contentModificationDateKey]))?.contentModificationDate ?? .distantPast
            return ProjectSummary(root: saved.root, count: saved.cards.count,
                                  figureCount: saved.cards.filter { $0.kind == "figure" }.count,
                                  updated: updated,
                                  preview: saved.cards.first { $0.kind == "block" && $0.status == "ready" } ?? saved.cards.first,
                                  format: saved.resolvedFormat)
        }.sorted { $0.updated > $1.updated }
    }
}
