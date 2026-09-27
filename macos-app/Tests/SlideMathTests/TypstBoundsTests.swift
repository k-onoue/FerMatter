import XCTest
import AppKit
@testable import SlideMath

final class TypstBoundsTests: XCTestCase {
    func testInlineFractionsRootsAndScriptsHaveTransparentMarginsInEveryDirection() throws {
        guard ProcessInfo.processInfo.environment["SLIDEMATH_COMPILER_TESTS"] == "1" else { throw XCTSkip("Compiler integration disabled") }
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("TypstBounds-\(UUID())")
        let root = base.appendingPathComponent("project")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: base) }
        let worker = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Resources/tex_worker.py")
        for size in [11, 48] {
            let main = root.appendingPathComponent("main.typ")
            let source = "#set text(size: \(size)pt)\n"
            try source.write(to: main, atomically: true, encoding: .utf8)
            let expressions = ["v(t) = (dif x(t)) / (dif t)", "(a/b)/(c/d)", "sqrt(x_1^2+y_2^2)", "sum_(i=1)^n x_i", "vec(a,b,c)"]
            let cards = expressions.enumerated().map { ["id": String($0.offset), "kind": "inline", "tex": $0.element] }
            let request: [String: Any] = ["action": "render", "format": "typst", "root": root.path, "main": "main.typ", "cache": base.appendingPathComponent("cache").path, "cards": cards]
            let input = base.appendingPathComponent("request.json")
            try JSONSerialization.data(withJSONObject: request).write(to: input)
            let process = Process(), output = Pipe()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
            process.arguments = ["python3", "-B", worker.path, input.path]
            process.standardOutput = output; process.standardError = output
            try process.run()
            let data = output.fileHandleForReading.readDataToEndOfFile()
            process.waitUntilExit()
            XCTAssertEqual(process.terminationStatus, 0, String(decoding: data, as: UTF8.self))
            let events = try String(decoding: data, as: UTF8.self).split(separator: "\n").compactMap { line -> [String: Any]? in
                let json = try JSONSerialization.jsonObject(with: Data(line.utf8)) as? [String: Any]
                return json?["event"] as? String == "card" ? json : nil
            }
            XCTAssertEqual(events.count, expressions.count)
            for event in events {
                XCTAssertEqual(event["status"] as? String, "ready", "\(event)")
                let png = try XCTUnwrap(event["pngPath"] as? String)
                let bitmap = try XCTUnwrap(NSBitmapImageRep(data: Data(contentsOf: URL(fileURLWithPath: png))))
                var minX = bitmap.pixelsWide, minY = bitmap.pixelsHigh, maxX = -1, maxY = -1
                for y in 0..<bitmap.pixelsHigh {
                    for x in 0..<bitmap.pixelsWide where (bitmap.colorAt(x: x, y: y)?.alphaComponent ?? 0) > 0.05 {
                        minX = min(minX, x); minY = min(minY, y); maxX = max(maxX, x); maxY = max(maxY, y)
                    }
                }
                let label = "size \(size), equation \(event["id"] ?? "")"
                XCTAssertGreaterThan(maxX, minX, label); XCTAssertGreaterThan(maxY, minY, label)
                XCTAssertGreaterThan(minX, 1, label); XCTAssertGreaterThan(minY, 1, label)
                XCTAssertLessThan(maxX, bitmap.pixelsWide - 2, label); XCTAssertLessThan(maxY, bitmap.pixelsHigh - 2, label)
                let svg = try Data(contentsOf: URL(fileURLWithPath: XCTUnwrap(event["svgPath"] as? String)))
                let dimensions = try SVGExporter.pointSize(svg)
                let pdf = try XCTUnwrap(CGPDFDocument(URL(fileURLWithPath: XCTUnwrap(event["pdfPath"] as? String)) as CFURL)?.page(at: 1))
                XCTAssertEqual(pdf.getBoxRect(.mediaBox).height, dimensions.height, accuracy: 0.01)
            }
            XCTAssertEqual(try String(contentsOf: main), source)
        }
    }
}
