import XCTest
import AppKit
import ImageIO
@testable import SlideMath

final class ImageExporterTests: XCTestCase {
    private func fixture() throws -> (URL, URL, URL) {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("ImageExportTests-\(UUID())")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: folder) }
        let svg = folder.appendingPathComponent("input.svg"), pdf = folder.appendingPathComponent("input.pdf")
        try Data(#"<svg xmlns="http://www.w3.org/2000/svg" width="100pt" height="40pt"><path fill="red" d="M10 10H90V30H10Z"/></svg>"#.utf8).write(to: svg)
        var box = CGRect(x: 0, y: 0, width: 100, height: 40)
        let context = try XCTUnwrap(CGContext(pdf as CFURL, mediaBox: &box, nil))
        context.beginPDFPage(nil); context.setFillColor(CGColor(colorSpace: CGColorSpace(name: CGColorSpace.sRGB)!, components: [1, 0, 0, 1])!)
        context.fill(CGRect(x: 10, y: 10, width: 80, height: 20)); context.endPDFPage(); context.closePDF()
        return (folder, svg, pdf)
    }

    func testOriginalSVGIsUnchangedAndScaledVectorPDFUsesPhysicalDimensions() throws {
        let (folder, svg, pdf) = try fixture()
        XCTAssertEqual(try ImageExporter.file(svg: svg, pdf: pdf, png: nil, format: .svg, directory: folder), svg)
        let original = try Data(contentsOf: svg)
        let scaled = folder.appendingPathComponent("scaled.svg")
        try SVGExporter.scaled(original, to: 24, sourceTeXPoints: 12, sourcePointsPerInch: 72).write(to: scaled)
        let result = try ImageExporter.file(svg: scaled, pdf: pdf, png: nil, format: .pdf, directory: folder)
        let page = try XCTUnwrap(CGPDFDocument(result as CFURL)?.page(at: 1))
        XCTAssertEqual(page.getBoxRect(.mediaBox).size, CGSize(width: 200, height: 80))
        // No raster XObjects were introduced into the vector PDF.
        let data = try Data(contentsOf: result)
        XCTAssertFalse(String(decoding: data, as: UTF8.self).contains("/Subtype /Image"))
        XCTAssertEqual(try Data(contentsOf: svg), original)
        XCTAssertEqual(try ImageExporter.file(svg: scaled, pdf: pdf, png: nil, format: .pdf, directory: folder), result)
    }

    func testRasterExportsHaveCorrectSizeTransparencyColorAndDPI() throws {
        let (folder, svg, pdf) = try fixture()
        for format in [ImageExportFormat.png, .jpeg] {
            let url = try ImageExporter.file(svg: svg, pdf: pdf, png: nil, format: format, directory: folder)
            let bitmap = try XCTUnwrap(NSBitmapImageRep(data: Data(contentsOf: url)))
            XCTAssertEqual(bitmap.pixelsWide, 300); XCTAssertEqual(bitmap.pixelsHigh, 120)
            let corner = try XCTUnwrap(bitmap.colorAt(x: 0, y: 0)?.usingColorSpace(.sRGB))
            let center = try XCTUnwrap(bitmap.colorAt(x: 150, y: 60)?.usingColorSpace(.sRGB))
            XCTAssertGreaterThan(center.redComponent, 0.9); XCTAssertGreaterThan(center.redComponent, center.greenComponent * 4)
            if format == .png { XCTAssertEqual(corner.alphaComponent, 0, accuracy: 0.01) }
            else { XCTAssertGreaterThan(corner.redComponent, 0.98); XCTAssertGreaterThan(corner.greenComponent, 0.98); XCTAssertEqual(corner.alphaComponent, 1) }
            let source = try XCTUnwrap(CGImageSourceCreateWithURL(url as CFURL, nil))
            let metadata = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any])
            XCTAssertEqual(try XCTUnwrap(metadata[kCGImagePropertyDPIWidth] as? Double), 216, accuracy: 0.1)
        }
    }

    func testLegacyPNGCanBeConvertedButMissingPDFAndHugeRasterFailClearly() throws {
        let (folder, svg, pdf) = try fixture()
        let png = try ImageExporter.file(svg: svg, pdf: pdf, png: nil, format: .png, directory: folder)
        XCTAssertNoThrow(try ImageExporter.file(svg: svg, pdf: nil, png: png, format: .jpeg, directory: folder))
        XCTAssertThrowsError(try ImageExporter.file(svg: svg, pdf: nil, png: png, format: .pdf, directory: folder))
        XCTAssertThrowsError(try ImageExporter.converted(size: CGSize(width: 1e20, height: 10), pdf: pdf, png: nil, format: .png))
    }

    func testDragProviderOffersChosenImageTypeAndExistingFile() throws {
        let (folder, svg, pdf) = try fixture()
        for format in ImageExportFormat.allCases {
            let url = try ImageExporter.file(svg: svg, pdf: pdf, png: nil, format: format, directory: folder)
            let provider = ImageExporter.provider(url: url, format: format, name: "Example")
            XCTAssertTrue(provider.hasItemConformingToTypeIdentifier(format.type.identifier))
            let loaded = expectation(description: format.title)
            provider.loadFileRepresentation(forTypeIdentifier: format.type.identifier) { file, error in
                XCTAssertNil(error)
                XCTAssertTrue(file.map { FileManager.default.fileExists(atPath: $0.path) } ?? false)
                loaded.fulfill()
            }
            wait(for: [loaded], timeout: 5)
        }
    }
}
