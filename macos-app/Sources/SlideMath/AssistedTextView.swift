import AppKit
import SwiftUI

final class AssistedTextView: NSTextView {
    // Each source tab/pane owns its edit history. Synchronizing another pane must
    // not clear the window-wide undo stack for the editor the user is typing in.
    private let sourceUndoManager = UndoManager()
    override var undoManager: UndoManager? { sourceUndoManager }
    private var sourceChangeObservers: [NSObjectProtocol] = []
    private var editingMonitor: Any?
    var onSourceChange: ((String) -> Void)? {
        didSet {
            guard sourceChangeObservers.isEmpty, onSourceChange != nil else { return }
            for (name, object) in [(NSText.didChangeNotification, self as AnyObject),
                                   (Notification.Name.NSUndoManagerDidUndoChange, sourceUndoManager),
                                   (Notification.Name.NSUndoManagerDidRedoChange, sourceUndoManager)] {
                sourceChangeObservers.append(NotificationCenter.default.addObserver(forName: name, object: object, queue: .main) { [weak self] _ in
                    guard let self else { return }; self.onSourceChange?(self.string)
                })
            }
        }
    }
    deinit {
        sourceChangeObservers.forEach(NotificationCenter.default.removeObserver)
        if let editingMonitor { NSEvent.removeMonitor(editingMonitor) }
    }
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if let editingMonitor { NSEvent.removeMonitor(editingMonitor); self.editingMonitor = nil }
        guard window != nil else { return }
        editingMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self, let window = self.window, window === NSApp.keyWindow,
                  window.firstResponder === self, !self.isHiddenOrHasHiddenAncestor else { return event }
            return self.handleSourceKey(event) ? nil : event
        }
    }
    override func copy(_ sender: Any?) {
        if selectedRange().length == 0 { copySourceLine(cut: false) } else { super.copy(sender) }
    }
    override func cut(_ sender: Any?) {
        if selectedRange().length == 0 { copySourceLine(cut: true) } else { super.cut(sender) }
    }
    override func paste(_ sender: Any?) {
        if selectedRange().length == 0, NSPasteboard.general.string(forType: Self.wholeLinePasteboard) != nil,
           let text = NSPasteboard.general.string(forType: .string) {
            let start = selectedLines().location
            replaceSource(NSRange(location: start, length: 0), with: text,
                          selection: NSRange(location: start + (text as NSString).length, length: 0), action: "Paste")
        } else { super.paste(sender) }
    }
    override func validateUserInterfaceItem(_ item: NSValidatedUserInterfaceItem) -> Bool {
        if item.action == #selector(undo(_:)) { return isEditable && sourceUndoManager.canUndo }
        if item.action == #selector(redo(_:)) { return isEditable && sourceUndoManager.canRedo }
        if item.action == #selector(copy(_:)) { return true }
        if item.action == #selector(cut(_:)) { return isEditable }
        return super.validateUserInterfaceItem(item)
    }
    override func validateMenuItem(_ item: NSMenuItem) -> Bool {
        if item.action == #selector(undo(_:)) {
            item.title = sourceUndoManager.undoMenuItemTitle
            return isEditable && sourceUndoManager.canUndo
        }
        if item.action == #selector(redo(_:)) {
            item.title = sourceUndoManager.redoMenuItemTitle
            return isEditable && sourceUndoManager.canRedo
        }
        return super.validateMenuItem(item)
    }
    // AppKit's window-level actions otherwise use the window's history instead
    // of the independent history owned by this source editor.
    @objc func undo(_ sender: Any?) {
        guard isEditable, sourceUndoManager.canUndo else { return }
        dismissSuggestions(); fields = []; breakUndoCoalescing()
        sourceUndoManager.undo()
        dismissSuggestions()
    }
    @objc func redo(_ sender: Any?) {
        guard isEditable, sourceUndoManager.canRedo else { return }
        dismissSuggestions(); fields = []; breakUndoCoalescing()
        sourceUndoManager.redo()
        dismissSuggestions()
    }
    override func deleteToBeginningOfLine(_ sender: Any?) {
        let range = selectedRange(), source = string as NSString
        guard range.length == 0, range.location > 0 else { super.deleteToBeginningOfLine(sender); return }
        let start = source.lineRange(for: range).location
        let deletion = start == range.location ? precedingLineBreak(at: start) : NSRange(location: start, length: range.location - start)
        replaceSource(deletion, with: "", selection: NSRange(location: deletion.location, length: 0), action: "Delete to Line Start")
    }
    var onSourceFocus: (() -> Void)?
    override func becomeFirstResponder() -> Bool {
        let accepted = super.becomeFirstResponder()
        if accepted { onSourceFocus?() }
        return accepted
    }
    var sourceFormat: ProjectFormat = .latex
    var sourceContext: SourceContext = .document
    var projectCompletions: [SourceCompletion] = []
    private var completionWork: DispatchWorkItem?
    private var completionPanel: CompletionPanel?
    private(set) var suggestions: [SourceCompletion] = []
    private var suggestionIndex = 0
    private var suggestionRange = NSRange(location: 0, length: 0)
    private var insertingCompletion = false
    private var fields: [NSRange] = []
    private var activeField = 0
    private var completionEnd = 0

    private var catalog: [SourceCompletion] {
        ProjectCompletions.unique(ProjectCompletions.definitions(in: string, format: sourceFormat) + projectCompletions + SourceCompletion.catalog(sourceFormat, context: sourceContext))
    }

    override func didChangeText() {
        super.didChangeText()
        completionWork?.cancel()
        guard !insertingCompletion else { return }
        let work = DispatchWorkItem { [weak self] in self?.showSuggestions() }
        completionWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15, execute: work)
    }

    override func complete(_ sender: Any?) { showSuggestions(manual: true) }

    func showSuggestions(manual: Bool = false) {
        guard !hasMarkedText(), selectedRange().length == 0, window?.firstResponder === self else { dismissSuggestions(); return }
        let range = rangeForUserCompletion
        guard range.length >= (manual ? 1 : 2), NSMaxRange(range) <= (string as NSString).length else { dismissSuggestions(); return }
        let prefix = (string as NSString).substring(with: range)
        let matches = catalog.filter { $0.trigger.hasPrefix(prefix) }
        guard !matches.isEmpty else { dismissSuggestions(); return }
        suggestions = Array(matches.prefix(8)); suggestionIndex = 0; suggestionRange = range
        displaySuggestions()
    }

    private func displaySuggestions() {
        guard let window else { return }
        let panel = completionPanel ?? CompletionPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        completionPanel = panel
        panel.isReleasedWhenClosed = false; panel.hasShadow = true; panel.isOpaque = false; panel.backgroundColor = .clear
        panel.hidesOnDeactivate = true; panel.level = window.level; panel.appearance = effectiveAppearance
        panel.collectionBehavior = [.fullScreenAuxiliary, .transient]
        let height = CGFloat(suggestions.count * 30 + 12), width: CGFloat = 340
        panel.contentView = NSHostingView(rootView: CompletionList(items: suggestions, selected: suggestionIndex, choose: { [weak self] index in
            self?.acceptSuggestion(at: index)
        }))
        let caret = firstRect(forCharacterRange: selectedRange(), actualRange: nil)
        let screen = window.screen?.visibleFrame ?? window.frame
        let x = max(screen.minX, min(caret.minX, screen.maxX - width))
        let y = caret.minY - height - 4 >= screen.minY ? caret.minY - height - 4 : caret.maxY + 4
        panel.setFrame(NSRect(x: x, y: y, width: width, height: height), display: true)
        if panel.parent !== window { window.addChildWindow(panel, ordered: .above) }
        panel.orderFront(nil)
    }

    func acceptSuggestion(at index: Int? = nil) {
        let index = index ?? suggestionIndex
        guard suggestions.indices.contains(index) else { return }
        let completion = suggestions[index], range = suggestionRange
        dismissSuggestions()
        applyCompletion(completion, replacing: range)
    }

    func dismissSuggestions() {
        completionWork?.cancel(); completionWork = nil; suggestions = []
        if let panel = completionPanel { panel.parent?.removeChildWindow(panel); panel.orderOut(nil) }
    }

    override func keyDown(with event: NSEvent) {
        if !suggestions.isEmpty {
            switch event.keyCode {
            case 53: dismissSuggestions(); return
            case 125: suggestionIndex = (suggestionIndex + 1) % suggestions.count; displaySuggestions(); return
            case 126: suggestionIndex = (suggestionIndex + suggestions.count - 1) % suggestions.count; displaySuggestions(); return
            case 36, 48:
                if !event.modifierFlags.contains(.command) { acceptSuggestion(); return }
                dismissSuggestions()
            default:
                if event.modifierFlags.contains(.command) || [123, 124].contains(event.keyCode) { dismissSuggestions() }
            }
        }
        super.keyDown(with: event)
    }
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if !suggestions.isEmpty && event.keyCode == 53 { dismissSuggestions(); return true }
        return super.performKeyEquivalent(with: event)
    }
    override func cancelOperation(_ sender: Any?) {
        if !suggestions.isEmpty { dismissSuggestions() }
        else { showSuggestions(manual: true) }
    }
    override func mouseDown(with event: NSEvent) { dismissSuggestions(); super.mouseDown(with: event) }
    override func resignFirstResponder() -> Bool { dismissSuggestions(); return super.resignFirstResponder() }
    override func viewWillMove(toWindow newWindow: NSWindow?) { dismissSuggestions(); super.viewWillMove(toWindow: newWindow) }

    override var rangeForUserCompletion: NSRange {
        SourceCompletion.completionRange(in: string, caret: selectedRange().location, format: sourceFormat)
    }
    override func completions(forPartialWordRange charRange: NSRange, indexOfSelectedItem index: UnsafeMutablePointer<Int>) -> [String]? {
        let prefix = (string as NSString).substring(with: charRange)
        guard !prefix.isEmpty else { return [] }
        index.pointee = 0
        return catalog.map(\.trigger).filter { $0.hasPrefix(prefix) }
    }
    override func insertCompletion(_ word: String, forPartialWordRange range: NSRange, movement: Int, isFinal: Bool) {
        // Native completion normally previews partial replacements. Keep them out
        // of the source and undo history until the user accepts a candidate.
        guard isFinal, movement != NSCancelTextMovement,
              let completion = catalog.first(where: { $0.trigger == word }) else { return }
        applyCompletion(completion, replacing: range)
    }

    func applyCompletion(_ completion: SourceCompletion, replacing range: NSRange) {
        dismissSuggestions(); insertingCompletion = true
        defer { insertingCompletion = false }
        let expansion = completion.expansion
        fields = []
        breakUndoCoalescing()
        insertText(expansion.text, replacementRange: range)
        breakUndoCoalescing()
        fields = expansion.fields.map { NSRange(location: range.location + $0.location, length: $0.length) }
        activeField = 0; completionEnd = range.location + (expansion.text as NSString).length
        window?.makeFirstResponder(self)
        setSelectedRange(fields.first ?? NSRange(location: completionEnd, length: 0))
        scrollRangeToVisible(selectedRange())
    }

    override func shouldChangeText(in affectedCharRange: NSRange, replacementString: String?) -> Bool {
        let allowed = super.shouldChangeText(in: affectedCharRange, replacementString: replacementString)
        if allowed, !fields.isEmpty {
            let current = fields[activeField]
            if affectedCharRange.location >= current.location && NSMaxRange(affectedCharRange) <= NSMaxRange(current) {
                let delta = ((replacementString ?? "") as NSString).length - affectedCharRange.length
                fields[activeField].length += delta; completionEnd += delta
                for index in fields.indices where index > activeField { fields[index].location += delta }
            } else { fields = [] }
        }
        return allowed
    }

    override func insertTab(_ sender: Any?) {
        if advanceField(backwards: false) { return }
        if selectedRange().length > 0 { indentSourceLines(outdent: false) }
        else { insertText("  ", replacementRange: selectedRange()) }
    }
    override func insertBacktab(_ sender: Any?) {
        if advanceField(backwards: true) { return }
        indentSourceLines(outdent: true)
    }
    private func advanceField(backwards: Bool) -> Bool {
        guard !fields.isEmpty else { return false }
        let current = fields[activeField], selection = selectedRange()
        guard selection.location >= current.location, NSMaxRange(selection) <= NSMaxRange(current), completionEnd <= (string as NSString).length else { fields = []; return false }
        activeField += backwards ? -1 : 1
        if activeField >= fields.count {
            fields = []; setSelectedRange(NSRange(location: completionEnd, length: 0))
        } else {
            activeField = max(activeField, 0); setSelectedRange(fields[activeField])
        }
        scrollRangeToVisible(selectedRange()); return true
    }

    func highlight() {
        guard !hasMarkedText(), let storage = textStorage else { return }
        let full = NSRange(location: 0, length: storage.length)
        storage.beginEditing()
        storage.addAttributes([.foregroundColor: NSColor.textColor, .font: NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)], range: full)
        let rules: [(String, NSColor)] = [
            (sourceFormat == .latex ? #"\\[a-zA-Z]+\*?"# : #"#?[a-zA-Z][a-zA-Z.]*\b(?=\s*\()|#[a-zA-Z]+"#, .systemPurple),
            (#"[{}\[\]$&]"#, .systemTeal),
            (#"\b[0-9]+(?:\.[0-9]+)?\b"#, .systemOrange),
            (sourceFormat == .latex ? #"(?<!\\)%[^\n]*"# : #"//[^\n]*|/\*[\s\S]*?\*/"#, .secondaryLabelColor)
        ]
        for (pattern, color) in rules {
            guard let regex = try? NSRegularExpression(pattern: pattern) else { continue }
            for match in regex.matches(in: string, range: full) { storage.addAttribute(.foregroundColor, value: color, range: match.range) }
        }
        storage.endEditing()
        typingAttributes = [.foregroundColor: NSColor.textColor, .font: NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)]
    }
    override func viewDidChangeEffectiveAppearance() { super.viewDidChangeEffectiveAppearance(); highlight() }
}
