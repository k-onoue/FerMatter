import AppKit

/// Number logical lines, leaving continuation rows unnumbered when text wraps.
final class SourceLineRuler: NSRulerView {
    private var observers: [NSObjectProtocol] = []
    private var cachedText = ""
    private(set) var lineStarts = [0]
    override var isOpaque: Bool { false }
    override var isFlipped: Bool { true }

    init(scroll: NSScrollView) {
        super.init(scrollView: scroll, orientation: .verticalRuler)
        clientView = scroll.documentView
        ruleThickness = 44
        scroll.contentView.postsBoundsChangedNotifications = true
        for (name, object) in [(NSText.didChangeNotification, scroll.documentView as AnyObject),
                               (NSTextView.didChangeSelectionNotification, scroll.documentView as AnyObject),
                               (NSView.boundsDidChangeNotification, scroll.contentView as AnyObject)] {
            observers.append(NotificationCenter.default.addObserver(forName: name, object: object, queue: .main) { [weak self] _ in self?.refresh() })
        }
        setAccessibilityElement(false)
    }
    required init(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    deinit { observers.forEach(NotificationCenter.default.removeObserver) }

    func refresh() {
        guard let view = clientView as? NSTextView else { return }
        if cachedText != view.string {
            cachedText = view.string; lineStarts = [0]
            let source = cachedText as NSString
            var position = 0
            while position < source.length {
                let range = source.lineRange(for: NSRange(location: position, length: 0))
                position = NSMaxRange(range)
                if position < source.length || source.substring(with: range).endsInSourceNewline {
                    lineStarts.append(position)
                }
            }
            let width = max(44, CGFloat(String(lineStarts.count).count) * 8 + 22)
            if ruleThickness != width { ruleThickness = width }
        }
        needsDisplay = true
    }

    override func drawHashMarksAndLabels(in rect: NSRect) {
        guard let view = clientView as? NSTextView, let layout = view.layoutManager, let container = view.textContainer else { return }
        layout.ensureLayout(for: container)
        let visible = view.visibleRect
        let glyphs = layout.glyphRange(forBoundingRect: visible.offsetBy(dx: -view.textContainerOrigin.x, dy: -view.textContainerOrigin.y), in: container)
        let characters = layout.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
        let selected = (view.string as NSString).lineRange(for: view.selectedRange()).location
        for (index, start) in lineStarts.enumerated() {
            if start < characters.location && index + 1 < lineStarts.count && lineStarts[index + 1] <= characters.location { continue }
            if start > NSMaxRange(characters) { break }
            let lineRect: NSRect
            if start == (view.string as NSString).length {
                lineRect = layout.extraLineFragmentRect
            } else {
                let glyph = layout.glyphIndexForCharacter(at: start)
                lineRect = layout.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
            }
            let point = convert(NSPoint(x: 0, y: lineRect.minY + view.textContainerOrigin.y), from: view)
            let attributes: [NSAttributedString.Key: Any] = [
                .font: NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .regular),
                .foregroundColor: start == selected ? NSColor.labelColor : NSColor.secondaryLabelColor
            ]
            let number = String(index + 1) as NSString
            let size = number.size(withAttributes: attributes)
            number.draw(at: NSPoint(x: ruleThickness - size.width - 12, y: point.y + 1), withAttributes: attributes)
        }
    }
}
