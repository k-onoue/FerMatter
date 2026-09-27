import Foundation

enum SVGExportError: LocalizedError {
    case missingFontMetrics, invalidSVG
    var errorDescription: String? {
        switch self {
        case .missingFontMetrics: return "Font metrics are not ready. Check Project Settings and try again."
        case .invalidSVG: return "This SVG has no supported dimensions."
        }
    }
}

enum SVGExporter {
    static func pointSize(_ data: Data) throws -> CGSize {
        let document = try XMLDocument(data: data)
        guard let root = document.rootElement(), root.localName == "svg",
              let width = dimensions(root.attribute(forName: "width")?.stringValue),
              let height = dimensions(root.attribute(forName: "height")?.stringValue),
              width.isFinite, height.isFinite, width > 0, height > 0 else { throw SVGExportError.invalidSVG }
        return CGSize(width: width, height: height)
    }
    static func scaled(_ data: Data, to points: Double, sourceTeXPoints: Double, sourcePointsPerInch: Double = 72.27) throws -> Data {
        guard points.isFinite, sourceTeXPoints.isFinite, sourcePointsPerInch.isFinite,
              points > 0, sourceTeXPoints > 0, sourcePointsPerInch > 0 else { throw SVGExportError.invalidSVG }
        let document = try XMLDocument(data: data, options: .nodePreserveAll)
        guard let root = document.rootElement(), root.localName == "svg",
              let width = root.attribute(forName: "width"), let height = root.attribute(forName: "height"),
              let w = dimensions(width.stringValue), let h = dimensions(height.stringValue) else { throw SVGExportError.invalidSVG }
        // TeX uses 72.27 pt/in; Typst, SVG and slide applications use 72 pt/in.
        let scale = points / (sourceTeXPoints * 72 / sourcePointsPerInch)
        width.stringValue = String(format: "%.6fpt", locale: Locale(identifier: "en_US_POSIX"), w * scale)
        height.stringValue = String(format: "%.6fpt", locale: Locale(identifier: "en_US_POSIX"), h * scale)
        return document.xmlData(options: .nodePreserveAll)
    }

    private static func dimensions(_ value: String?) -> Double? {
        guard let value else { return nil }
        if value.hasSuffix("pt") { return Double(value.dropLast(2)) }
        if value.hasSuffix("px"), let pixels = Double(value.dropLast(2)) { return pixels * 72 / 96 }
        if let pixels = Double(value) { return pixels * 72 / 96 }
        return nil
    }
}
