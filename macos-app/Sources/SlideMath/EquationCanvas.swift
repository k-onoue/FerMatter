import SwiftUI
import AppKit

private struct CardBounds: PreferenceKey {
    static var defaultValue: [String: Anchor<CGRect>] = [:]
    static func reduce(value: inout [String: Anchor<CGRect>], nextValue: () -> [String: Anchor<CGRect>]) { value.merge(nextValue(), uniquingKeysWith: { _, new in new }) }
}

struct EquationCanvas: View {
    @EnvironmentObject private var store: LibraryStore
    var body: some View {
        GeometryReader { viewport in
            ScrollViewReader { scrolling in
                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: store.compact ? 190 : 260), spacing: 16)], spacing: 16) {
                        ForEach(store.filtered) { card in
                            EquationTile(card: card).anchorPreference(key: CardBounds.self, value: .bounds) { [card.id: $0] }
                        }
                    }.padding(.horizontal, 22).padding(.bottom, 22)
                        .frame(minHeight: viewport.size.height, alignment: .top)
                        .backgroundPreferenceValue(CardBounds.self) { anchors in
                            GeometryReader { proxy in CanvasMarquee(store: store, frames: anchors.mapValues { proxy[$0] }) }
                        }
                }.id(store.kind + store.origin + store.query + store.sectionFilter)
                    .background(CanvasKeyboardBridge(store: store))
                    .onChange(of: store.scrollToEquation) { _, id in
                        if let id { withAnimation { scrolling.scrollTo(id, anchor: .center) } }
                    }
            }
            .onAppear { updateColumns(viewport.size.width) }
            .onChange(of: viewport.size.width) { _, width in updateColumns(width) }
            .onChange(of: store.compact) { _, _ in updateColumns(viewport.size.width) }
        }.onChange(of: store.filtered.map(\.id)) { _, _ in store.retainVisibleSelection() }
    }
    private func updateColumns(_ width: Double) {
        store.canvasColumns = max(1, Int((width - 44 + 16) / (store.compact ? 206 : 276)))
    }
}

private struct CanvasMarquee: View {
    @ObservedObject var store: LibraryStore
    let frames: [String: CGRect]
    @State private var rectangle: CGRect?
    @State private var initial = Set<String>()
    @State private var modifiers: NSEvent.ModifierFlags = []
    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear.contentShape(Rectangle())
                .onTapGesture { store.clearCardSelection() }
                .gesture(DragGesture(minimumDistance: 3).onChanged { value in
                    if rectangle == nil { initial = store.selectedIDs; modifiers = NSEvent.modifierFlags; store.focusCanvas() }
                    let rect = CGRect(x: min(value.startLocation.x, value.location.x), y: min(value.startLocation.y, value.location.y),
                                      width: abs(value.location.x - value.startLocation.x), height: abs(value.location.y - value.startLocation.y))
                    rectangle = rect
                    let hits = Set(frames.filter { $0.value.intersects(rect) }.keys)
                    let selected = modifiers.contains(.command) ? initial.symmetricDifference(hits) : modifiers.contains(.shift) ? initial.union(hits) : hits
                    store.setCardSelection(selected, primary: store.selection)
                }.onEnded { _ in rectangle = nil })
            if let rectangle {
                Rectangle().fill(.blue.opacity(0.12)).overlay(Rectangle().stroke(.blue.opacity(0.8), lineWidth: 1))
                    .frame(width: rectangle.width, height: rectangle.height).offset(x: rectangle.minX, y: rectangle.minY).allowsHitTesting(false)
            }
        }
    }
}

private struct CanvasKeyboardBridge: NSViewRepresentable {
    let store: LibraryStore
    func makeNSView(context: Context) -> CanvasKeyboardView { CanvasKeyboardView() }
    func updateNSView(_ view: CanvasKeyboardView, context: Context) { view.store = store; store.canvasResponder = view }
}

private final class CanvasKeyboardView: NSView {
    weak var store: LibraryStore?
    override var acceptsFirstResponder: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override func selectAll(_ sender: Any?) { store?.selectAllCards() }
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard window?.firstResponder === self else { return false }
        if event.modifierFlags.contains(.command), event.charactersIgnoringModifiers == "a" { store?.selectAllCards(); return true }
        return super.performKeyEquivalent(with: event)
    }
    override func keyDown(with event: NSEvent) {
        guard let store else { return }
        if event.keyCode == 53 { store.clearCardSelection(); return }
        if [123, 124, 125, 126].contains(event.keyCode), !store.filtered.isEmpty {
            let ids = store.filtered.map(\.id)
            let current = ids.firstIndex(of: store.selection ?? "")
            let step = [125, 126].contains(event.keyCode) ? store.canvasColumns : 1
            let delta = [123, 126].contains(event.keyCode) ? -step : step
            let index = current.map { min(ids.count - 1, max(0, $0 + delta)) } ?? 0
            store.selectCard(ids[index], modifiers: event.modifierFlags); store.scrollToEquation = ids[index]
            return
        }
        super.keyDown(with: event)
    }
}
