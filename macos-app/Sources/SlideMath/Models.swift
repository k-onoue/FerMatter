import Foundation

enum ProjectFormat: String, Codable {
    case latex, typst
    var displayName: String { self == .typst ? "Typst" : "LaTeX" }
    var pointsPerInch: Double { self == .typst ? 72 : 72.27 }
}

struct TypstContextFrame: Codable, Hashable {
    var file: String
    var offset: Int
}

struct DocumentHeading: Codable, Identifiable, Hashable {
    var id: String
    var title: String
    var level: Int
    var file: String
    var line: Int
    var path: [String]
}

func equationCountLabel(_ count: Int) -> String {
    "\(count) \(count == 1 ? "equation" : "equations")"
}

struct Equation: Codable, Identifiable, Hashable {
    var id: String
    var name: String
    var tex: String
    var kind: String
    var origin: String
    var sourceOrder: Double
    var file: String?
    var line: Int?
    var endLine: Int?
    var contextStart: Int?
    var context: [String]?
    var parentID: String?
    var status: String
    var svgPath: String?
    var pngPath: String?
    var error: String?
    var logPath: String?
    var typstContext: [TypstContextFrame]? = nil
    var sourceFontSize: Double? = nil
    var outlinePath: [String]? = nil
    var sourceOffset: Int? = nil
    var sourceEnd: Int? = nil
    var column: Int? = nil
    var endColumn: Int? = nil
    var pdfPath: String? = nil
    // nil inherits the project default; zero preserves the original dimensions.
    var exportFontSize: Double? = nil

    var location: String { file.map { "\($0):\(line ?? 1)" } ?? "Custom equation" }
    static func draft() -> Equation {
        Equation(id: "custom-" + UUID().uuidString, name: "Untitled equation", tex: "", kind: "block", origin: "custom", sourceOrder: 0, status: "pending")
    }
}

struct SavedProject: Codable {
    var root: String
    var main: String
    var engine: String
    var cards: [Equation]
    var customOrder: [String]
    var sections: [EquationSection]? = nil
    var sectionAssignments: [String: String]? = nil
    var sourceFontSize: Double? = nil
    var exportFontSize: Double? = nil
    var format: ProjectFormat? = nil
    var outline: [DocumentHeading]? = nil
    var collapsedOutline: [String]? = nil
    var exportFormat: ImageExportFormat? = nil
    var resolvedFormat: ProjectFormat { format ?? (main.hasSuffix(".typ") ? .typst : .latex) }
}

struct EquationSection: Codable, Identifiable, Hashable {
    var id = UUID().uuidString
    var name: String
}

struct ProjectSummary: Identifiable {
    var id: String { root }
    var root: String
    var count: Int
    var figureCount: Int = 0
    var updated: Date
    var preview: Equation?
    var format: ProjectFormat = .latex
    var name: String { URL(fileURLWithPath: root).lastPathComponent }
    var available: Bool { FileManager.default.fileExists(atPath: root) }
    var updatedLabel: String {
        let formatter = RelativeDateTimeFormatter()
        formatter.locale = Locale(identifier: "en")
        formatter.unitsStyle = .short
        return formatter.localizedString(for: updated, relativeTo: Date())
    }
}

struct WorkerEvent: Decodable {
    var event: String
    var cards: [Equation]?
    var candidates: [String]?
    var main: String?
    var root: String?
    var warnings: [String]?
    var files: [String]?
    var hasRC: Bool?
    var id: String?
    var status: String?
    var svgPath: String?
    var pngPath: String?
    var pdfPath: String?
    var error: String?
    var message: String?
    var logPath: String?
    var fontSize: Double?
    var format: ProjectFormat?
    var outline: [DocumentHeading]?
    var document: DocumentPDF?
    var anchor: PDFAnchor?
    var file: String?
    var line: Int?
    var column: Int?
    var previewStart: Int?
    var previewEnd: Int?
}

struct PDFAnchor: Codable, Equatable {
    var targetID: String
    var page: Int
    var x: Double
    var y: Double
    var width: Double? = nil
    var height: Double? = nil
}

struct DocumentPDF: Codable {
    var pdfPath: String
    var sourceRoot: String
    var format: ProjectFormat
    var synchronized: Bool
    var anchors: [PDFAnchor]
    var warnings: [String]
}
