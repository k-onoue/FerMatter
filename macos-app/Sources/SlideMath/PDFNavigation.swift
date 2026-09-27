import AppKit
import PDFKit
import SwiftUI

@MainActor
final class PDFSession: ObservableObject {
    @Published var document: DocumentPDF?
    @Published var building = false
    @Published var error: String?
    @Published var notice: String?
    @Published var page = 0
    @Published var pageCount = 0
    @Published var scale = 1.0
    weak var view: PDFView?
    var onSource: ((String, Int, Int) -> Void)?
    var onTarget: ((String) -> Void)?
    var onReady: (() -> Void)?
    private let compilation = WorkerSession()
    private let navigation = WorkerSession()
    private var marker: PDFAnnotation?
    private var markerPage: PDFPage?
    private var markerToken = UUID()

    var canNavigate: Bool { document?.synchronized == true && !building && error == nil }

    func cancel() {
        compilation.cancel(); navigation.cancel(); building = false
    }

    func invalidate() {
        cancel(); clearMarker(); error = "Source changed. Rebuild the PDF to navigate."
    }

    func reset() {
        cancel(); clearMarker(); document = nil; error = nil; notice = nil; page = 0; pageCount = 0
    }

    func build(root: String, main: String, engine: String, cache: URL, force: Bool = false) {
        guard !root.isEmpty, !main.isEmpty else { return }
        cancel(); building = true; error = nil; notice = nil
        compilation.start(["action": "pdf_build", "root": root, "main": main, "engine": engine,
                    "cache": cache.path, "force": force], receive: { [weak self] event in
            guard let self else { return }
            if let document = event.document {
                self.document = document
                self.notice = document.warnings.first
            }
            if event.event == "error" { self.error = event.message ?? "PDF compilation failed." }
        }, finished: { [weak self] in
            guard let self else { return }
            self.building = false
            if self.error == nil { self.onReady?() }
        })
    }

    func forward(id: String, file: String, line: Int, column: Int = 0) {
        request(["action": "pdf_forward", "targetID": id, "file": file, "line": line, "column": column])
    }

    func reverse(page: Int, x: Double, y: Double) {
        request(["action": "pdf_reverse", "page": page, "x": x, "y": y])
    }

    private func request(_ fields: [String: Any]) {
        guard canNavigate, let document, let data = try? JSONEncoder().encode(document),
              let json = try? JSONSerialization.jsonObject(with: data) else { return }
        var request = fields; request["document"] = json
        navigation.start(request, receive: { [weak self] event in
            guard let self else { return }
            switch event.event {
            case "pdf_location": if let anchor = event.anchor { self.reveal(anchor) }
            case "pdf_source":
                if let file = event.file, let line = event.line { self.onSource?(file, line, event.column ?? -1) }
            case "pdf_target": if let id = event.id { self.onTarget?(id) }
            case "pdf_unmapped": self.notice = "No source position at this location."
            case "error": self.notice = event.message
            default: break
            }
        })
    }

    static func pagePoint(x: Double, y: Double, bounds: NSRect) -> NSPoint {
        NSPoint(x: bounds.minX + x, y: bounds.maxY - y)
    }

    func reveal(_ anchor: PDFAnchor) {
        guard let view, let page = view.document?.page(at: anchor.page - 1) else { return }
        notice = document?.warnings.first
        let point = Self.pagePoint(x: anchor.x, y: anchor.y, bounds: page.bounds(for: .mediaBox))
        view.go(to: NSRect(x: point.x - 40, y: point.y - 35, width: 80, height: 80), on: page)
        clearMarker()
        let annotation = PDFAnnotation(bounds: NSRect(x: point.x - 7, y: point.y - 7, width: 14, height: 14), forType: .circle, withProperties: nil)
        annotation.color = .systemBlue
        let border = PDFBorder(); border.lineWidth = 2; annotation.border = border
        annotation.shouldPrint = false
        page.addAnnotation(annotation); marker = annotation; markerPage = page
        let token = markerToken
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in
            guard let self, self.markerToken == token else { return }; self.clearMarker()
        }
    }

    private func clearMarker() {
        markerToken = UUID()
        if let marker { markerPage?.removeAnnotation(marker) }
        marker = nil; markerPage = nil
    }

    func updatePage() {
        guard let view, let document = view.document else { return }
        pageCount = document.pageCount
        page = view.currentPage.map { document.index(for: $0) + 1 } ?? 0
        scale = view.scaleFactor
    }
    func previousPage() { view?.goToPreviousPage(nil) }
    func nextPage() { view?.goToNextPage(nil) }
    func zoomIn() { view?.autoScales = false; view?.zoomIn(nil) }
    func zoomOut() { view?.autoScales = false; view?.zoomOut(nil) }
    func fit() { view?.autoScales = true }
}

struct DocumentPDFView: NSViewRepresentable {
    @ObservedObject var session: PDFSession
    func makeCoordinator() -> Coordinator { Coordinator(session) }
    func makeNSView(context: Context) -> PDFView {
        let view = PDFView()
        view.displayMode = .singlePageContinuous
        view.displayDirection = .vertical
        view.displaysPageBreaks = true
        view.autoScales = true
        view.backgroundColor = .clear
        context.coordinator.attach(view)
        return view
    }
    func updateNSView(_ view: PDFView, context: Context) {
        guard let path = session.document?.pdfPath, context.coordinator.path != path else { return }
        context.coordinator.path = path
        view.document = PDFDocument(url: URL(fileURLWithPath: path))
        view.autoScales = true
        DispatchQueue.main.async { session.updatePage(); session.onReady?() }
    }
    static func dismantleNSView(_ view: PDFView, coordinator: Coordinator) { coordinator.detach() }

    @MainActor final class Coordinator {
        let session: PDFSession
        var path: String?
        var monitor: Any?
        var notifications: [NSObjectProtocol] = []
        init(_ session: PDFSession) { self.session = session }
        func attach(_ view: PDFView) {
            session.view = view
            for name in [Notification.Name.PDFViewPageChanged, .PDFViewScaleChanged] {
                notifications.append(NotificationCenter.default.addObserver(forName: name, object: view, queue: .main) { [weak self] _ in
                    MainActor.assumeIsolated { self?.session.updatePage() }
                })
            }
            monitor = NSEvent.addLocalMonitorForEvents(matching: .leftMouseDown) { [weak self, weak view] event in
                guard let self, let view, event.window === view.window, event.modifierFlags.contains(.command) else { return event }
                let local = view.convert(event.locationInWindow, from: nil)
                guard view.bounds.contains(local), self.session.canNavigate,
                      let page = view.page(for: local, nearest: false), let document = view.document else { return event }
                let point = view.convert(local, to: page), bounds = page.bounds(for: .mediaBox)
                self.session.reverse(page: document.index(for: page) + 1, x: point.x - bounds.minX, y: bounds.maxY - point.y)
                return nil
            }
        }
        func detach() {
            if let monitor { NSEvent.removeMonitor(monitor) }; monitor = nil
            notifications.forEach { NotificationCenter.default.removeObserver($0) }; notifications = []
            session.view = nil
        }
    }
}

struct PDFPane: View {
    @EnvironmentObject private var store: LibraryStore
    @ObservedObject var session: PDFSession
    @State private var showDetails = false
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                Text("PDF").font(.headline)
                if session.building { ProgressView().controlSize(.small) }
                Spacer()
                Button(action: session.previousPage) { Image(systemName: "chevron.left") }.disabled(session.page <= 1).help("Previous Page").accessibilityLabel("Previous PDF Page")
                Text("\(session.page) / \(session.pageCount)").monospacedDigit().font(.caption).frame(minWidth: 40)
                Button(action: session.nextPage) { Image(systemName: "chevron.right") }.disabled(session.page >= session.pageCount).help("Next Page").accessibilityLabel("Next PDF Page")
                Button { store.buildPDF(force: true) } label: { Image(systemName: "arrow.clockwise") }.disabled(session.building).help("Rebuild PDF").accessibilityLabel("Rebuild PDF")
                Button { store.showPDF = false } label: { Image(systemName: "xmark") }.help("Close PDF").accessibilityLabel("Close PDF")
            }.buttonStyle(.plain).padding(16)
            Divider()
            if session.document != nil {
                DocumentPDFView(session: session).help("Command-click near an equation to select it")
            } else {
                Spacer()
                if session.building { Text("Compiling PDF…").foregroundStyle(.secondary) }
                else { Image(systemName: "doc.text").font(.system(size: 36, weight: .light)).foregroundStyle(.secondary) }
                Spacer()
            }
            Divider()
            HStack(spacing: 12) {
                if session.error != nil || session.notice != nil {
                    Button { showDetails.toggle() } label: { Image(systemName: "exclamationmark.circle").foregroundStyle(.orange) }
                        .help(session.error == nil ? "Navigation details" : "PDF compilation failed")
                        .accessibilityLabel("PDF details")
                        .popover(isPresented: $showDetails) {
                            ScrollView { Text(session.error ?? session.notice ?? "").font(.caption).textSelection(.enabled).padding(18) }.frame(width: 380, height: 230)
                        }
                    if session.error != nil { Text("Build failed").font(.caption).foregroundStyle(.secondary) }
                }
                Spacer()
                Button(action: session.zoomOut) { Image(systemName: "minus.magnifyingglass") }.help("Zoom Out").accessibilityLabel("Zoom PDF Out")
                Text("\(Int(session.scale * 100))%").monospacedDigit().font(.caption)
                Button(action: session.zoomIn) { Image(systemName: "plus.magnifyingglass") }.help("Zoom In").accessibilityLabel("Zoom PDF In")
                Button("Fit", action: session.fit).help("Fit Page")
            }.buttonStyle(.plain).disabled(session.document == nil && session.error == nil).padding(13)
        }
    }
}
