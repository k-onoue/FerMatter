import AppKit
import CryptoKit
import ImageIO
import UniformTypeIdentifiers

enum ImageExportFormat: String, Codable, CaseIterable, Identifiable {
    case svg, png, jpeg, pdf
    var id: String { rawValue }
    var title: String { rawValue.uppercased() }
    var fileExtension: String { self == .jpeg ? "jpg" : rawValue }
    var type: UTType {
        switch self { case .svg: .svg; case .png: .png; case .jpeg: .jpeg; case .pdf: .pdf }
    }
}

enum ImageExportError: LocalizedError {
    case missingPDF, conversion, tooLarge
    var errorDescription: String? {
        switch self {
        case .missingPDF: "Render this item again to export PDF."
        case .conversion: "Could not create the image. Try rendering this item again."
        case .tooLarge: "This image is too large. Choose a smaller font size or a vector format."
        }
    }
}

enum ImageExporter {
    /// SVG stays byte-for-byte identical at the original size. All other formats
    /// use its physical dimensions, including the presentation font-size setting.
    static func file(svg: URL, pdf: URL?, png: URL?, format: ImageExportFormat, directory: URL) throws -> URL {
        if format == .svg { return svg }
        let svgData = try Data(contentsOf: svg)
        let size = try SVGExporter.pointSize(svgData)
        let source = pdf ?? png
        guard let source else { throw ImageExportError.conversion }
        var fingerprint = svgData
        fingerprint.append(try Data(contentsOf: source))
        fingerprint.append(Data((format.rawValue + "-216dpi-v1").utf8))
        let key = SHA256.hash(data: fingerprint).map { String(format: "%02x", $0) }.joined().prefix(24)
        let file = directory.appendingPathComponent("equation-\(key).\(format.fileExtension)")
        if FileManager.default.fileExists(atPath: file.path) { return file }
        let data = try converted(size: size, pdf: pdf, png: png, format: format)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try data.write(to: file, options: .atomic)
        return file
    }

    static func converted(size: CGSize, pdf: URL?, png: URL?, format: ImageExportFormat) throws -> Data {
        let page = pdf.flatMap { CGPDFDocument($0 as CFURL)?.page(at: 1) }
        let bounds = CGRect(origin: .zero, size: size)
        if format == .pdf {
            guard let page else { throw ImageExportError.missingPDF }
            let data = NSMutableData()
            var box = bounds
            guard let consumer = CGDataConsumer(data: data), let context = CGContext(consumer: consumer, mediaBox: &box, nil) else { throw ImageExportError.conversion }
            context.beginPDFPage(nil)
            draw(page, in: context, bounds: bounds)
            context.endPDFPage(); context.closePDF()
            return data as Data
        }
        guard format == .png || format == .jpeg else { throw ImageExportError.conversion }
        let scale = 3.0
        let w = ceil(size.width * scale), h = ceil(size.height * scale)
        guard w.isFinite, h.isFinite, w > 0, h > 0, w <= 16384, h <= 16384, w * h <= 32_000_000 else { throw ImageExportError.tooLarge }
        guard let context = CGContext(data: nil, width: Int(w), height: Int(h), bitsPerComponent: 8, bytesPerRow: 0,
                                      space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { throw ImageExportError.conversion }
        context.scaleBy(x: w / size.width, y: h / size.height)
        if format == .jpeg {
            context.setFillColor(CGColor(gray: 1, alpha: 1)); context.fill(bounds)
        }
        if let page { draw(page, in: context, bounds: bounds) }
        else if let png, let source = CGImageSourceCreateWithURL(png as CFURL, nil), let image = CGImageSourceCreateImageAtIndex(source, 0, nil) {
            context.interpolationQuality = .high; context.draw(image, in: bounds)
        } else { throw ImageExportError.conversion }
        guard let image = context.makeImage() else { throw ImageExportError.conversion }
        let data = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(data, format.type.identifier as CFString, 1, nil) else { throw ImageExportError.conversion }
        CGImageDestinationAddImage(destination, image, [kCGImagePropertyDPIWidth: w / size.width * 72,
                                                       kCGImagePropertyDPIHeight: h / size.height * 72,
                                                       kCGImageDestinationLossyCompressionQuality: 0.95] as CFDictionary)
        guard CGImageDestinationFinalize(destination) else { throw ImageExportError.conversion }
        return data as Data
    }

    private static func draw(_ page: CGPDFPage, in context: CGContext, bounds: CGRect) {
        context.saveGState()
        context.concatenate(page.getDrawingTransform(.mediaBox, rect: bounds, rotate: 0, preserveAspectRatio: true))
        context.drawPDFPage(page)
        context.restoreGState()
    }

    static func provider(url: URL, format: ImageExportFormat, name: String) -> NSItemProvider {
        let provider = NSItemProvider(object: url as NSURL)
        provider.suggestedName = name.replacingOccurrences(of: "/", with: "-") + "." + format.fileExtension
        provider.registerFileRepresentation(forTypeIdentifier: format.type.identifier, fileOptions: [], visibility: .all) { completion in
            completion(url, false, nil); return nil
        }
        return provider
    }
}
