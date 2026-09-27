import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct SourceTabClose: Identifiable {
    let id = UUID()
    let path: String
    let pane: Int
}

struct SourceTabDrag: Codable {
    static let type = UTType(exportedAs: "local.slidemath.source-tab")
    let project: String
    let pane: Int
    let path: String

}

enum SourceDropZone: Equatable {
    case left, right, top, bottom, center
    static func at(_ point: CGPoint, size: CGSize) -> Self {
        guard size.width > 0, size.height > 0 else { return .center }
        let edges: [(Self, CGFloat)] = [(.left, point.x / size.width), (.right, 1 - point.x / size.width),
                                       (.top, point.y / size.height), (.bottom, 1 - point.y / size.height)]
        guard let closest = edges.min(by: { $0.1 < $1.1 }), closest.1 < 0.24 else { return .center }
        return closest.0
    }
    func rect(in size: CGSize) -> CGRect {
        switch self {
        case .left: CGRect(x: 0, y: 0, width: size.width / 2, height: size.height)
        case .right: CGRect(x: size.width / 2, y: 0, width: size.width / 2, height: size.height)
        case .top: CGRect(x: 0, y: 0, width: size.width, height: size.height / 2)
        case .bottom: CGRect(x: 0, y: size.height / 2, width: size.width, height: size.height / 2)
        case .center: CGRect(origin: .zero, size: size)
        }
    }
}

struct NativeSourceDropZone: NSViewRepresentable {
    let files: ProjectFiles
    let pane: Int
    func makeNSView(context: Context) -> SourceDropView { SourceDropView() }
    func updateNSView(_ view: SourceDropView, context: Context) {
        view.isDraggingTab = { [weak files] in files?.draggingSourceTab != nil }
        view.drop = { [weak files] payload, edge in files?.dropSourceTab(payload, onto: pane, edge: edge) ?? false }
    }
}
final class SourceDropView: NSView {
    var isDraggingTab: (() -> Bool)?
    var drop: ((SourceTabDrag, SourceDropZone) -> Bool)?
    private var zone: SourceDropZone? { didSet { needsDisplay = true } }
    override var isFlipped: Bool { true }
    override init(frame: NSRect) {
        super.init(frame: frame)
        registerForDraggedTypes([NSPasteboard.PasteboardType(SourceTabDrag.type.identifier)])
        setAccessibilityElement(false)
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    override func hitTest(_ point: NSPoint) -> NSView? {
        isDraggingTab?() == true ? super.hitTest(point) : nil
    }
    private func payload(_ sender: NSDraggingInfo) -> SourceTabDrag? {
        guard let data = sender.draggingPasteboard.data(forType: NSPasteboard.PasteboardType(SourceTabDrag.type.identifier)) else { return nil }
        return try? JSONDecoder().decode(SourceTabDrag.self, from: data)
    }
    override func draggingEntered(_ sender: NSDraggingInfo) -> NSDragOperation { draggingUpdated(sender) }
    override func draggingUpdated(_ sender: NSDraggingInfo) -> NSDragOperation {
        guard payload(sender) != nil else { zone = nil; return [] }
        zone = .at(convert(sender.draggingLocation, from: nil), size: bounds.size)
        return zone == .center ? .move : .copy
    }
    override func draggingExited(_ sender: NSDraggingInfo?) { zone = nil }
    override func prepareForDragOperation(_ sender: NSDraggingInfo) -> Bool { payload(sender) != nil }
    override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
        let edge = SourceDropZone.at(convert(sender.draggingLocation, from: nil), size: bounds.size)
        zone = nil
        guard let payload = payload(sender) else { return false }
        // Finish AppKit's drag session before changing the source view hierarchy.
        DispatchQueue.main.async { [drop] in _ = drop?(payload, edge) }
        return true
    }
    override func concludeDragOperation(_ sender: NSDraggingInfo?) { zone = nil }
    override func draw(_ dirtyRect: NSRect) {
        guard let zone else { return }
        let rect = zone.rect(in: bounds.size).insetBy(dx: 4, dy: 4)
        let shape = NSBezierPath(roundedRect: rect, xRadius: 8, yRadius: 8)
        NSColor.controlAccentColor.withAlphaComponent(0.15).setFill(); shape.fill()
        NSColor.controlAccentColor.setStroke(); shape.lineWidth = 2; shape.stroke()
    }
}

struct NativeSourceTab: NSViewRepresentable {
    let files: ProjectFiles
    let buffer: SourceFileBuffer
    let pane: Int
    func makeNSView(context: Context) -> SourceTabView { SourceTabView() }
    func updateNSView(_ view: SourceTabView, context: Context) {
        view.title = (buffer.path as NSString).lastPathComponent; view.dirty = buffer.dirty
        view.payload = SourceTabDrag(project: files.root?.path ?? "", pane: pane, path: buffer.path)
        view.setAccessibilityLabel(view.title)
        view.select = { files.select(buffer.path, pane: pane); files.focusCurrentEditor() }
        view.close = { files.requestCloseTab(buffer.path, pane: pane) }
        view.dragChanged = { [weak files] payload in files?.draggingSourceTab = payload }
        view.needsDisplay = true
    }
}

final class SourceTabView: NSView, NSDraggingSource {
    var title = "", dirty = false
    var payload: SourceTabDrag?
    var select: (() -> Void)?, close: (() -> Void)?
    var dragChanged: ((SourceTabDrag?) -> Void)?
    private var downPoint = CGPoint.zero, closePressed = false, dragging = false
    override var isFlipped: Bool { true }
    override init(frame: NSRect) {
        super.init(frame: frame); setAccessibilityElement(true); setAccessibilityRole(.button)
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    override func accessibilityPerformPress() -> Bool { select?(); return true }
    override func draw(_ dirtyRect: NSRect) {
        let text = title as NSString
        let style = NSMutableParagraphStyle(); style.lineBreakMode = .byTruncatingMiddle
        text.draw(in: CGRect(x: 10, y: (bounds.height - 16) / 2, width: max(0, bounds.width - 36), height: 16),
                  withAttributes: [.font: NSFont.systemFont(ofSize: 12), .foregroundColor: NSColor.labelColor, .paragraphStyle: style])
        if dirty {
            NSColor.controlAccentColor.setFill(); NSBezierPath(ovalIn: CGRect(x: bounds.width - 17, y: bounds.midY - 2.5, width: 5, height: 5)).fill()
        } else {
            NSColor.secondaryLabelColor.setStroke()
            let x = bounds.width - 15, y = bounds.midY
            let path = NSBezierPath(); path.lineWidth = 1.1
            path.move(to: CGPoint(x: x - 3, y: y - 3)); path.line(to: CGPoint(x: x + 3, y: y + 3))
            path.move(to: CGPoint(x: x + 3, y: y - 3)); path.line(to: CGPoint(x: x - 3, y: y + 3)); path.stroke()
        }
    }
    override func mouseDown(with event: NSEvent) {
        downPoint = convert(event.locationInWindow, from: nil); closePressed = downPoint.x > bounds.width - 27; dragging = false
        if !closePressed { dragChanged?(payload) }
    }
    override func mouseUp(with event: NSEvent) {
        guard !dragging else { return }
        dragChanged?(nil)
        guard bounds.contains(convert(event.locationInWindow, from: nil)) else { return }
        if closePressed { close?() } else { select?() }
    }
    override func otherMouseDown(with event: NSEvent) { if event.buttonNumber == 2 { close?() } }
    override func mouseDragged(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        guard !dragging, !closePressed, hypot(point.x - downPoint.x, point.y - downPoint.y) > 4, let payload,
              let data = try? JSONEncoder().encode(payload) else { return }
        dragging = true
        let pasteboard = NSPasteboardItem(); pasteboard.setData(data, forType: NSPasteboard.PasteboardType(SourceTabDrag.type.identifier))
        let item = NSDraggingItem(pasteboardWriter: pasteboard)
        let preview = NSImage(size: bounds.size, flipped: false) { rect in
            NSColor.windowBackgroundColor.setFill(); NSBezierPath(roundedRect: rect, xRadius: 7, yRadius: 7).fill()
            (self.title as NSString).draw(at: CGPoint(x: 10, y: 9), withAttributes: [.font: NSFont.systemFont(ofSize: 12), .foregroundColor: NSColor.labelColor]); return true
        }
        item.setDraggingFrame(bounds, contents: preview)
        dragChanged?(payload)
        beginDraggingSession(with: [item], event: event, source: self)
    }
    func draggingSession(_ session: NSDraggingSession, sourceOperationMaskFor context: NSDraggingContext) -> NSDragOperation { [.copy, .move] }
    func draggingSession(_ session: NSDraggingSession, endedAt screenPoint: NSPoint, operation: NSDragOperation) { dragging = false; dragChanged?(nil) }
}

struct SourceShortcutRouting: NSViewRepresentable {
    @ObservedObject var files: ProjectFiles
    @ObservedObject var shortcuts: ShortcutPreferences
    let active: Bool
    func makeNSView(context: Context) -> SourceShortcutView { SourceShortcutView() }
    func updateNSView(_ view: SourceShortcutView, context: Context) {
        view.active = active
        view.bindings = [.nextSourceTab, .previousSourceTab, .splitSource, .splitSourceDown].map { ($0, shortcuts.binding($0)) }
        view.action = { [weak files] command in
            guard let files, !files.buffers.isEmpty else { return }
            switch command {
            case .nextSourceTab: files.cycleSourceTab(1)
            case .previousSourceTab: files.cycleSourceTab(-1)
            case .splitSource: files.splitSourcePane(.right)
            case .splitSourceDown: files.splitSourcePane(.down)
            default: break
            }
        }
    }
}
final class SourceShortcutView: NSView {
    var active = false
    var bindings: [(AppCommand, ShortcutBinding)] = []
    var action: ((AppCommand) -> Void)?
    private var monitor: Any?
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard window != nil, monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self, self.active, let window = self.window, NSApp.keyWindow === window,
                  window.attachedSheet == nil, let binding = ShortcutBinding.from(event),
                  let command = self.bindings.first(where: { !$0.1.key.isEmpty && $0.1 == binding })?.0 else { return event }
            self.action?(command); return nil
        }
    }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    deinit { if let monitor { NSEvent.removeMonitor(monitor) } }
}
