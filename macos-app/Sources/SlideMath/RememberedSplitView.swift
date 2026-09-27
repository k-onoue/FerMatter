import SwiftUI
import AppKit

enum TrailingPane: String { case pdf, inspector }

@MainActor final class PaneWidths: ObservableObject {
    @Published private(set) var pdf: Double
    @Published private(set) var inspector: Double
    private let defaults: UserDefaults
    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        pdf = max(350, defaults.object(forKey: "pdfPaneWidth") as? Double ?? 500)
        inspector = min(600, max(280, defaults.object(forKey: "inspectorPaneWidth") as? Double ?? 315))
    }
    func preferred(_ pane: TrailingPane) -> Double { pane == .pdf ? pdf : inspector }
    func set(_ width: Double, for pane: TrailingPane) {
        guard width.isFinite else { return }
        if pane == .pdf { pdf = max(350, width); defaults.set(pdf, forKey: "pdfPaneWidth") }
        else { inspector = min(600, max(280, width)); defaults.set(inspector, forKey: "inspectorPaneWidth") }
    }
    func fitted(_ pane: TrailingPane, available: Double) -> Double { min(preferred(pane), max(0, available - 690)) }
}

struct RememberedSplitView<Leading: View, Trailing: View>: View {
    @ObservedObject var widths: PaneWidths
    let pane: TrailingPane?
    @ViewBuilder var leading: () -> Leading
    @ViewBuilder var trailing: () -> Trailing
    @State private var dragStart: Double?
    var body: some View {
        GeometryReader { geometry in
            HStack(spacing: 0) {
                leading().frame(maxWidth: .infinity, maxHeight: .infinity)
                if let pane {
                    let width = widths.fitted(pane, available: geometry.size.width)
                    Rectangle().fill(.primary.opacity(0.12)).frame(width: 1)
                        .padding(.horizontal, 2).contentShape(Rectangle())
                        .onHover { hovering in if hovering { NSCursor.resizeLeftRight.push() } else { NSCursor.pop() } }
                        .gesture(DragGesture(minimumDistance: 0, coordinateSpace: .named("librarySplit")).onChanged { value in
                            if dragStart == nil { dragStart = width }
                            widths.set(min(max(0, geometry.size.width - 690), (dragStart ?? width) - value.translation.width), for: pane)
                        }.onEnded { _ in dragStart = nil })
                        .accessibilityLabel("Resize " + (pane == .pdf ? "PDF" : "Inspector"))
                        .accessibilityValue("\(Int(width)) points")
                        .accessibilityAdjustableAction { direction in
                            widths.set(width + (direction == .increment ? 20 : -20), for: pane)
                        }
                    trailing().frame(width: width)
                }
            }.coordinateSpace(name: "librarySplit")
        }.onChange(of: pane) { _, _ in dragStart = nil }
    }
}
