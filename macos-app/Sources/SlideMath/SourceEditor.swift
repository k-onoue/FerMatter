import AppKit
import SwiftUI

@MainActor final class SourceEditorController: ObservableObject {
    weak var view: AssistedTextView?
    var retainedScroll: NSScrollView?
    func find() {
        guard let view else { return }
        view.window?.makeFirstResponder(view)
        let item = NSMenuItem(); item.tag = Int(NSFindPanelAction.showFindPanel.rawValue)
        view.performFindPanelAction(item)
    }
    func reveal(line: Int) {
        guard let view else { return }
        let lines = view.string.components(separatedBy: "\n")
        let row = min(max(0, line - 1), max(0, lines.count - 1))
        let offset = lines.prefix(row).reduce(0) { $0 + ($1 as NSString).length + 1 }
        let range = NSRange(location: offset, length: (lines[row] as NSString).length)
        view.setSelectedRange(range); view.scrollRangeToVisible(range)
        view.window?.makeFirstResponder(view)
    }
}

struct AssistedSourceEditor: View {
    @Binding var text: String
    let label: String
    let format: ProjectFormat
    var context: SourceContext = .document
    var projectCompletions: [SourceCompletion] = []
    @StateObject private var controller = SourceEditorController()
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(label).font(.system(size: 11, weight: .medium)).foregroundStyle(.secondary)
                Spacer()
            }
            SourceTextEditor(text: $text, label: label, format: format, context: context, controller: controller, projectCompletions: projectCompletions)
                .clipShape(RoundedRectangle(cornerRadius: 10))
                .modifier(GlassSurface(radius: 10))
        }
    }
}

/// A source editor keeps smart quotes and substitutions out of compiler input.
struct SourceTextEditor: NSViewRepresentable {
    @Binding var text: String
    var label: String
    var format: ProjectFormat = .latex
    var context: SourceContext = .document
    var controller: SourceEditorController? = nil
    var projectCompletions: [SourceCompletion] = []
    var isVisible = true
    var onFocus: (() -> Void)?
    func makeNSView(context: Context) -> SourceEditorContainer {
        let container = SourceEditorContainer(scroll: controller?.retainedScroll)
        controller?.retainedScroll = container.scroll
        let scroll = container.scroll
        let view = scroll.documentView as! AssistedTextView
        view.font = .monospacedSystemFont(ofSize: 13, weight: .regular)
        view.isRichText = false; view.allowsUndo = true
        view.usesFindBar = true; view.isIncrementalSearchingEnabled = true
        view.isAutomaticQuoteSubstitutionEnabled = false; view.isAutomaticDashSubstitutionEnabled = false
        view.isAutomaticTextReplacementEnabled = false; view.isAutomaticSpellingCorrectionEnabled = false
        view.isContinuousSpellCheckingEnabled = false; view.isAutomaticLinkDetectionEnabled = false
        view.textContainerInset = NSSize(width: 8, height: 10)
        view.setAccessibilityLabel(label)
        view.isAutomaticTextCompletionEnabled = false
        return container
    }
    func updateNSView(_ container: SourceEditorContainer, context: Context) {
        let scroll = container.scroll
        let view = scroll.documentView as! AssistedTextView
        scroll.isHidden = !isVisible
        if !isVisible { view.dismissSuggestions() }
        if view.string != text { view.string = text; view.undoManager?.removeAllActions() }
        view.sourceFormat = format; view.sourceContext = self.context; controller?.view = view
        view.setAccessibilityLabel(label)
        view.projectCompletions = projectCompletions
        view.onSourceFocus = onFocus
        view.onSourceChange = { [binding = $text, weak view] value in
            binding.wrappedValue = value; view?.highlight()
        }
        view.drawsBackground = false; view.backgroundColor = .clear
        scroll.drawsBackground = false; scroll.contentView.drawsBackground = false
        view.highlight()
        (scroll.verticalRulerView as? SourceLineRuler)?.refresh()
    }

}

final class SourceEditorContainer: NSView {
    let scroll: NSScrollView
    init(scroll: NSScrollView? = nil) {
        self.scroll = scroll ?? AssistedTextView.scrollableTextView()
        super.init(frame: .zero)
        self.scroll.frame = bounds; self.scroll.autoresizingMask = [.width, .height]
        self.scroll.drawsBackground = false; self.scroll.contentView.drawsBackground = false
        if self.scroll.verticalRulerView == nil {
            self.scroll.verticalRulerView = SourceLineRuler(scroll: self.scroll)
            self.scroll.hasVerticalRuler = true; self.scroll.rulersVisible = true
        }
        addSubview(self.scroll)
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    override func hitTest(_ point: NSPoint) -> NSView? {
        scroll.isHidden ? nil : super.hitTest(point)
    }
}
