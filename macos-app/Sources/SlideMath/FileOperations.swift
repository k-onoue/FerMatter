import AppKit

enum FileMutation {
    case copy(URL, URL), move(URL, URL), trash(URL), create(URL, directory: Bool)
}

struct FileRelocation {
    let from: URL
    let to: URL
}

struct FileReceipt {
    var inverse: [FileMutation] = []
    var relocations: [FileRelocation] = []
}

enum FileActionError: LocalizedError {
    case recursive, destinationMissing, noSelection, historyConflict
    var errorDescription: String? {
        switch self {
        case .recursive: "A folder cannot be placed inside itself."
        case .destinationMissing: "Select an existing project folder."
        case .noSelection: "Select a file or folder first."
        case .historyConflict: "The destination already exists. Move or rename it before trying again."
        }
    }
}

/// Records inverse operations using moves and the macOS Trash, never permanent deletion.
/// Each batch is checked before it starts; a failed batch rolls back completed steps.
final class FileOperations {
    let root: URL
    let trash: (URL) throws -> URL
    init(root: URL, trash: @escaping (URL) throws -> URL = FileOperations.systemTrash) {
        self.root = root.resolvingSymlinksInPath(); self.trash = trash
    }
    static func systemTrash(_ url: URL) throws -> URL {
        var result: NSURL?
        try FileManager.default.trashItem(at: url, resultingItemURL: &result)
        guard let result else { throw CocoaError(.fileNoSuchFile) }
        return result as URL
    }
    static func exists(_ url: URL) -> Bool {
        FileManager.default.fileExists(atPath: url.path) || (try? FileManager.default.destinationOfSymbolicLink(atPath: url.path)) != nil
    }
    private func validateParent(_ url: URL) throws {
        if url.path == root.path { throw ProjectFileError.outsideProject }
        if url.path.hasPrefix(root.path + "/") {
            let parent = url.deletingLastPathComponent().resolvingSymlinksInPath().path
            guard parent == root.path || parent.hasPrefix(root.path + "/") else { throw ProjectFileError.outsideProject }
        }
    }
    private func sameEntry(_ a: URL, _ b: URL) -> Bool {
        guard a.deletingLastPathComponent() == b.deletingLastPathComponent(), a.path.lowercased() == b.path.lowercased(),
              let x = try? a.resourceValues(forKeys: [.fileResourceIdentifierKey]).fileResourceIdentifier as? NSObject,
              let y = try? b.resourceValues(forKeys: [.fileResourceIdentifierKey]).fileResourceIdentifier as? NSObject else { return false }
        return x == y
    }
    func perform(_ operations: [FileMutation]) throws -> FileReceipt {
        for operation in operations {
            switch operation {
            case .copy(let from, let to), .move(let from, let to):
                try validateParent(from); try validateParent(to)
                guard Self.exists(from) else { throw CocoaError(.fileNoSuchFile) }
                if Self.exists(to) {
                    if case .move = operation, from.path != to.path, sameEntry(from, to) { /* case-only rename */ }
                    else { throw FileActionError.historyConflict }
                }
                guard FileManager.default.fileExists(atPath: to.deletingLastPathComponent().path) else { throw FileActionError.destinationMissing }
                if to.path.hasPrefix(from.path + "/") { throw FileActionError.recursive }
            case .trash(let url):
                try validateParent(url)
                guard Self.exists(url) else { throw CocoaError(.fileNoSuchFile) }
            case .create(let url, _):
                try validateParent(url)
                guard !Self.exists(url) else { throw ProjectFileError.exists }
            }
        }
        var receipt = FileReceipt()
        do {
            for operation in operations {
                let fm = FileManager.default
                switch operation {
                case .copy(let from, let to):
                    let temporary = to.deletingLastPathComponent().appendingPathComponent(".slidemath-copy-" + UUID().uuidString)
                    do {
                        try fm.copyItem(at: from, to: temporary)
                        try fm.moveItem(at: temporary, to: to)
                    } catch {
                        try? fm.removeItem(at: temporary)
                        throw error
                    }
                    receipt.inverse.insert(.trash(to), at: 0)
                case .move(let from, let to):
                    if Self.exists(to), sameEntry(from, to) {
                        let intermediate = from.deletingLastPathComponent().appendingPathComponent(".slidemath-rename-" + UUID().uuidString)
                        try fm.moveItem(at: from, to: intermediate)
                        do { try fm.moveItem(at: intermediate, to: to) }
                        catch { try? fm.moveItem(at: intermediate, to: from); throw error }
                    } else { try fm.moveItem(at: from, to: to) }
                    receipt.inverse.insert(.move(to, from), at: 0)
                    receipt.relocations.append(FileRelocation(from: from, to: to))
                case .trash(let url):
                    let destination = try trash(url)
                    receipt.inverse.insert(.move(destination, url), at: 0)
                    receipt.relocations.append(FileRelocation(from: url, to: destination))
                case .create(let url, let directory):
                    if directory { try fm.createDirectory(at: url, withIntermediateDirectories: false) }
                    else { try Data().write(to: url, options: .withoutOverwriting) }
                    receipt.inverse.insert(.trash(url), at: 0)
                }
            }
        } catch {
            if !receipt.inverse.isEmpty {
                do { _ = try perform(receipt.inverse) }
                catch let rollbackError { throw NSError(domain: "SlideMath.FileOperation", code: 1, userInfo: [NSLocalizedDescriptionKey: "The operation stopped: \(error.localizedDescription) Recovery also needs attention: \(rollbackError.localizedDescription)"]) }
            }
            throw error
        }
        return receipt
    }
}
