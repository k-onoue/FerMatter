import XCTest
@testable import SlideMath

final class SVGExporterTests: XCTestCase {
    func testTypstUsesPresentationPointsAndKeepsGlyphReferences() throws {
        let input = Data(##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100pt" height="20pt" viewBox="0 0 100 20"><defs><path id="g1" d="M0 0L10 10"/></defs><use xlink:href="#g1"/></svg>"##.utf8)
        let data = try SVGExporter.scaled(input, to: 24, sourceTeXPoints: 12, sourcePointsPerInch: ProjectFormat.typst.pointsPerInch)
        let root = try XCTUnwrap(XMLDocument(data: data).rootElement())
        XCTAssertEqual(root.attribute(forName: "width")?.stringValue, "200.000000pt")
        XCTAssertEqual(root.attribute(forName: "height")?.stringValue, "40.000000pt")
        XCTAssertEqual(root.attribute(forName: "viewBox")?.stringValue, "0 0 100 20")
        XCTAssertEqual(root.elements(forName: "use").first?.attribute(forLocalName: "href", uri: "http://www.w3.org/1999/xlink")?.stringValue, "#g1")
    }

    func testScalesPhysicalDimensionsAndPreservesVectorGeometry() throws {
        let input = Data(#"<svg xmlns="http://www.w3.org/2000/svg" width="100pt" height="20pt" viewBox="0 0 100 20"><path d="M0 0L10 10"/></svg>"#.utf8)
        let result = try SVGExporter.scaled(input, to: 24, sourceTeXPoints: 10)
        let document = try XMLDocument(data: result)
        let root = try XCTUnwrap(document.rootElement())
        XCTAssertEqual(root.attribute(forName: "width")?.stringValue, "240.900000pt")
        XCTAssertEqual(root.attribute(forName: "height")?.stringValue, "48.180000pt")
        XCTAssertEqual(root.attribute(forName: "viewBox")?.stringValue, "0 0 100 20")
        XCTAssertEqual(root.elements(forName: "path").first?.attribute(forName: "d")?.stringValue, "M0 0L10 10")
        XCTAssertTrue(root.elements(forName: "image").isEmpty)
    }

    func testScalingUsesActualSourceFontAndRejectsUnsupportedDimensions() throws {
        let pixels = Data(#"<svg width="96px" height="48px" viewBox="0 0 96 48"/>"#.utf8)
        let a = try SVGExporter.scaled(pixels, to: 24, sourceTeXPoints: 10)
        let b = try SVGExporter.scaled(pixels, to: 24, sourceTeXPoints: 12)
        XCTAssertNotEqual(a, b)
        let bad = Data(#"<svg width="100%" height="auto"/>"#.utf8)
        XCTAssertThrowsError(try SVGExporter.scaled(bad, to: 24, sourceTeXPoints: 10))
    }
}
