import AppKit
import SwiftUI

struct SourcePreviewInput: Equatable {
    var root: String
    var main: String
    var engine: String
    var format: ProjectFormat
    var file: String
    var cache: String
    var sources: [String: String]
    var diskRevision: Int

    func request(at offset: Int) -> [String: Any] {
        ["action": "source_preview", "root": root, "main": main, "engine": engine,
         "format": format.rawValue, "file": file, "cache": cache, "sources": sources, "offset": offset]
    }
}

/// Independent of the library's render session and of other source panes.
@MainActor final class SourcePreviewModel: ObservableObject {
    @Published private(set) var card: Equation?
    @Published private(set) var busy = false
    @Published private(set) var error: String?
    private(set) var range: Range<Int>?
    var onChange: (() -> Void)?
    private let session: WorkerSession
    private var pending: DispatchWorkItem?
    private var input: SourcePreviewInput?
    private var offset: Int?

    init(session: WorkerSession? = nil) { self.session = session ?? WorkerSession() }

    func update(_ input: SourcePreviewInput, offset: Int, delay: TimeInterval = 0.4) {
        if self.input == input, self.offset == offset || range?.contains(offset) == true {
            self.offset = offset
            onChange?()
            return
        }
        cancel()
        self.input = input; self.offset = offset
        let work = DispatchWorkItem { [weak self] in self?.render(input, offset: offset) }
        pending = work
        if delay == 0 { work.perform() }
        else { DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: work) }
    }

    private func render(_ input: SourcePreviewInput, offset: Int) {
        pending = nil; busy = true
        session.start(input.request(at: offset), receive: { [weak self] event in
            guard let self else { return }
            switch event.event {
            case "source_preview":
                self.card = event.cards?.first
                if let start = event.previewStart, let end = event.previewEnd, start < end { self.range = start..<end }
            case "card":
                guard var card = self.card, card.id == event.id else { return }
                card.status = event.status ?? "failed"; card.svgPath = event.svgPath
                card.pngPath = event.pngPath; card.pdfPath = event.pdfPath
                card.sourceFontSize = event.fontSize; card.error = event.error
                self.card = card; self.error = event.error
            case "error": self.error = event.message
            default: break
            }
            self.onChange?()
        }, finished: { [weak self] in
            guard let self else { return }
            self.busy = false; self.onChange?()
        })
    }

    func cancel() {
        pending?.cancel(); pending = nil; session.cancel()
        input = nil; offset = nil; range = nil; card = nil; error = nil; busy = false
        onChange?()
    }

    deinit { pending?.cancel() }
}

struct SourcePreviewConfiguration {
    var input: SourcePreviewInput
    var format: ImageExportFormat
    var changeFormat: (ImageExportFormat) -> Void
    var drag: (Equation) -> NSItemProvider
    var copy: (Equation) -> Void
}

@MainActor final class SourceMathPreviewController {
    let model: SourcePreviewModel
    private weak var view: AssistedTextView?
    private var configuration: SourcePreviewConfiguration?
    private var panel: NSPanel?
    private var observers: [NSObjectProtocol] = []
    private var dismissedOffset: Int?
    var isEnabled: Bool { configuration != nil }

    init(model: SourcePreviewModel? = nil) { self.model = model ?? SourcePreviewModel() }

    func configure(view: AssistedTextView, configuration: SourcePreviewConfiguration?) {
        self.view = view
        if observers.isEmpty {
            for name in [NSTextView.didChangeSelectionNotification, NSText.didChangeNotification] {
                observers.append(NotificationCenter.default.addObserver(forName: name, object: view, queue: .main) { [weak self] _ in
                    MainActor.assumeIsolated { self?.caretChanged() }
                })
            }
            if let clip = view.enclosingScrollView?.contentView {
                clip.postsBoundsChangedNotifications = true
                observers.append(NotificationCenter.default.addObserver(forName: NSView.boundsDidChangeNotification, object: clip, queue: .main) { [weak self] _ in
                    MainActor.assumeIsolated { self?.display() }
                })
            }
            model.onChange = { [weak self] in self?.display() }
        }
        let changed = self.configuration?.input != configuration?.input
        self.configuration = configuration
        if changed { dismissedOffset = nil }
        guard configuration != nil else { hide(); return }
        caretChanged()
    }

    func caretChanged() {
        guard let view, let configuration, view.window?.firstResponder === view,
              !view.isHiddenOrHasHiddenAncestor, view.selectedRange().length == 0, !view.hasMarkedText() else { hide(); return }
        let offset = view.selectedRange().location
        guard dismissedOffset != offset else { return }
        dismissedOffset = nil
        model.update(configuration.input, offset: offset)
    }

    func dismiss() {
        dismissedOffset = view?.selectedRange().location
        hide()
    }

    func hide() { model.cancel(); closePanel() }
    private func closePanel() {
        if let panel { panel.parent?.removeChildWindow(panel); panel.orderOut(nil) }
    }

    private func display() {
        guard let view, let window = view.window, let configuration,
              window.firstResponder === view, !view.isHiddenOrHasHiddenAncestor,
              view.suggestions.isEmpty, model.card != nil || model.error != nil else { closePanel(); return }
        let caret = view.firstRect(forCharacterRange: view.selectedRange(), actualRange: nil)
        let visible = window.convertToScreen(view.convert(view.visibleRect, to: nil))
        guard caret.intersects(visible) else { closePanel(); return }
        let panel = self.panel ?? SourcePreviewPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        self.panel = panel
        panel.isReleasedWhenClosed = false; panel.hasShadow = true
        panel.isOpaque = false; panel.backgroundColor = .clear
        panel.hidesOnDeactivate = true; panel.level = window.level; panel.appearance = view.effectiveAppearance
        panel.collectionBehavior = [.fullScreenAuxiliary, .transient]
        panel.contentView = NSHostingView(rootView: SourceMathPreviewView(model: model, configuration: configuration, close: { [weak self] in self?.dismiss() }))
        let width: CGFloat = 310, height: CGFloat = model.error == nil ? 160 : 210
        let screen = window.screen?.visibleFrame ?? window.frame
        let x = max(screen.minX, min(caret.minX, screen.maxX - width))
        let y = caret.maxY + height + 8 <= screen.maxY ? caret.maxY + 8 : max(screen.minY, caret.minY - height - 8)
        panel.setFrame(NSRect(x: x, y: y, width: width, height: height), display: true)
        if panel.parent !== window { window.addChildWindow(panel, ordered: .above) }
        panel.orderFront(nil)
    }

    deinit {
        observers.forEach(NotificationCenter.default.removeObserver)
        if let panel { DispatchQueue.main.async { panel.parent?.removeChildWindow(panel); panel.orderOut(nil) } }
    }
}

private final class SourcePreviewPanel: NSPanel {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

private struct SourceMathPreviewView: View {
    @ObservedObject var model: SourcePreviewModel
    let configuration: SourcePreviewConfiguration
    let close: () -> Void
    var body: some View {
        VStack(spacing: 10) {
            HStack(spacing: 12) {
                Text("Preview").font(.caption.weight(.medium)).foregroundStyle(.secondary)
                if model.busy { ProgressView().controlSize(.mini) }
                Spacer()
                Menu {
                    ForEach(ImageExportFormat.allCases) { format in
                        Button(format.title) { configuration.changeFormat(format) }
                    }
                } label: { Text(configuration.format.title).font(.caption) }
                    .menuStyle(.borderlessButton).fixedSize().accessibilityLabel("Preview Export Format")
                Button {
                    if let card = model.card { configuration.copy(card) }
                } label: { Image(systemName: "doc.on.clipboard") }
                    .disabled(model.card?.status != "ready").help("Copy \(configuration.format.title)")
                    .accessibilityLabel("Copy Preview")
                Button(action: close) { Image(systemName: "xmark") }.help("Close Preview").accessibilityLabel("Close Preview")
            }.buttonStyle(.plain)
            if let card = model.card, card.status == "ready" {
                EquationImage(card: card).frame(maxWidth: .infinity, maxHeight: .infinity)
                    .contentShape(Rectangle()).onDrag { configuration.drag(card) }
                    .help("Drag \(configuration.format.title)").accessibilityLabel("Equation Preview")
            } else if let error = model.error {
                ScrollView { Text(error).font(.system(size: 11, design: .monospaced)).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading) }
            } else { Spacer(); ProgressView().controlSize(.small); Spacer() }
        }.padding(14).background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.primary.opacity(0.1)))
    }
}
