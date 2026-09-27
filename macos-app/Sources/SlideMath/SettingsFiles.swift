import AppKit
import SwiftUI
import UniformTypeIdentifiers

/// Documents are declarative, local data. Extensions round-trip without being executed.
protocol SettingsDocument: Codable {
    func validate() throws
}

enum SettingsFileError: LocalizedError {
    case invalid(String)
    var errorDescription: String? { if case .invalid(let message) = self { return message }; return nil }
}

enum SettingsValue: Codable, Equatable {
    case string(String), number(Decimal), bool(Bool), array([SettingsValue]), object([String: SettingsValue]), null
    init(from decoder: Decoder) throws {
        let value = try decoder.singleValueContainer()
        if value.decodeNil() { self = .null }
        else if let decoded = try? value.decode(Bool.self) { self = .bool(decoded) }
        else if let decoded = try? value.decode(String.self) { self = .string(decoded) }
        else if let decoded = try? value.decode(Decimal.self) { self = .number(decoded) }
        else if let decoded = try? value.decode([SettingsValue].self) { self = .array(decoded) }
        else { self = .object(try value.decode([String: SettingsValue].self)) }
    }
    func encode(to encoder: Encoder) throws {
        var value = encoder.singleValueContainer()
        switch self {
        case .string(let decoded): try value.encode(decoded)
        case .number(let decoded): try value.encode(decoded)
        case .bool(let decoded): try value.encode(decoded)
        case .array(let decoded): try value.encode(decoded)
        case .object(let decoded): try value.encode(decoded)
        case .null: try value.encodeNil()
        }
    }
}

/// Validates before publishing or writing, with an independent recovery copy.
final class SettingsFile<Document: SettingsDocument> {
    let url: URL
    var recoveryURL: URL { url.deletingLastPathComponent().appendingPathComponent(".last-valid-" + url.lastPathComponent) }
    private var loadedData: Data?
    static var maxBytes: Int { 1_048_576 }

    init(directory: URL, name: String) { url = directory.appendingPathComponent(name) }

    func decode(_ data: Data) throws -> Document {
        guard data.count <= Self.maxBytes else { throw SettingsFileError.invalid("Settings files must be smaller than 1 MB.") }
        let document = try JSONDecoder().decode(Document.self, from: data)
        try document.validate()
        return document
    }

    func read(_ source: URL) throws -> Document { try decode(readData(source)) }

    private func readData(_ source: URL) throws -> Data {
        let size = try source.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard size <= Self.maxBytes else { throw SettingsFileError.invalid("Settings files must be smaller than 1 MB.") }
        return try Data(contentsOf: source)
    }

    func load() throws -> Document {
        let data = try readData(url), document = try decode(data)
        try data.write(to: recoveryURL, options: .atomic)
        loadedData = data
        return document
    }

    func initial(_ fallback: Document) -> (Document, String?) {
        do {
            if FileManager.default.fileExists(atPath: url.path) { return (try load(), nil) }
            // A removed file is restored from the last valid settings at launch.
            let value = (try? read(recoveryURL)) ?? fallback
            try save(value, replacing: true)
            return (value, nil)
        } catch {
            return ((try? read(recoveryURL)) ?? fallback, "Could not load \(url.lastPathComponent). \(error.localizedDescription)")
        }
    }

    func encoded(_ document: Document) throws -> Data {
        try document.validate()
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        let data = try encoder.encode(document) + Data("\n".utf8)
        guard data.count <= Self.maxBytes else { throw SettingsFileError.invalid("Settings files must be smaller than 1 MB.") }
        return data
    }

    func save(_ document: Document, replacing: Bool = false) throws {
        let data = try encoded(document)
        if !replacing {
            guard let loadedData, (try? readData(url)) == loadedData else {
                throw SettingsFileError.invalid("The file changed on disk. Reload it before saving.")
            }
        }
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try data.write(to: url, options: .atomic)
        loadedData = data
        // Failure to refresh the recovery copy must not report a failed primary save.
        try? data.write(to: recoveryURL, options: .atomic)
    }

    func export(_ document: Document, to destination: URL) throws {
        try encoded(document).write(to: destination, options: .atomic)
    }
}

@MainActor protocol FileSettings: ObservableObject {
    var fileURL: URL { get }
    var fileError: String? { get }
    func reload()
    func importFile(_ url: URL)
    func exportFile(_ url: URL)
}

struct SettingsFileControls<Preferences: FileSettings>: View {
    @ObservedObject var preferences: Preferences
    var beforeAction: () -> Void = {}
    var body: some View {
        HStack {
            Button("Open File") { beforeAction(); NSWorkspace.shared.open(preferences.fileURL) }
            Button("Show in Finder") { beforeAction(); NSWorkspace.shared.activateFileViewerSelecting([preferences.fileURL]) }
            Button("Reload") { beforeAction(); preferences.reload() }
            Spacer()
            Button("Import…") {
                beforeAction()
                let panel = NSOpenPanel(); panel.allowedContentTypes = [.json]; panel.allowsMultipleSelection = false
                if panel.runModal() == .OK, let url = panel.url { preferences.importFile(url) }
            }
            Button("Export…") {
                beforeAction()
                let panel = NSSavePanel(); panel.allowedContentTypes = [.json]; panel.nameFieldStringValue = preferences.fileURL.lastPathComponent
                if panel.runModal() == .OK, let url = panel.url { preferences.exportFile(url) }
            }
        }.controlSize(.small)
    }
}
