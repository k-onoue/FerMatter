import AppKit

extension String {
    var endsInSourceNewline: Bool { unicodeScalars.last.map(CharacterSet.newlines.contains) ?? false }
}

extension AssistedTextView {
    static let wholeLinePasteboard = NSPasteboard.PasteboardType("local.slidemath.whole-line")
    var newline: String { string.contains("\r\n") ? "\r\n" : "\n" }
    func precedingLineBreak(at position: Int) -> NSRange {
        let source = string as NSString
        let length = position >= 2 && source.character(at: position - 1) == 10 && source.character(at: position - 2) == 13 ? 2 : 1
        return NSRange(location: position - length, length: length)
    }

    /// Exclude a following line when a selection ends at its first column.
    func selectedLines() -> NSRange {
        let selection = selectedRange()
        let extent = NSRange(location: selection.location, length: max(0, selection.length - 1))
        return (string as NSString).lineRange(for: extent)
    }

    func replaceSource(_ range: NSRange, with text: String, selection: NSRange, action: String) {
        guard isEditable else { return }
        dismissSuggestions(); breakUndoCoalescing()
        insertText(text, replacementRange: range)
        setSelectedRange(selection); scrollRangeToVisible(selection)
        breakUndoCoalescing(); undoManager?.setActionName(action)
    }

    func copySourceLine(cut: Bool) {
        let range = selectedLines(), source = string as NSString
        var text = source.substring(with: range)
        if !text.endsInSourceNewline { text += newline }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
        NSPasteboard.general.setString("1", forType: Self.wholeLinePasteboard)
        if cut { deleteSourceLines() }
    }

    func deleteSourceLines() {
        var range = selectedLines()
        let source = string as NSString
        if NSMaxRange(range) == source.length, range.location > 0,
           !source.substring(with: range).endsInSourceNewline {
            // The last line has no terminator; remove its preceding line break.
            let previous = precedingLineBreak(at: range.location)
            range = NSRange(location: previous.location, length: NSMaxRange(range) - previous.location)
        }
        replaceSource(range, with: "", selection: NSRange(location: range.location, length: 0), action: "Delete Line")
    }

    func duplicateSourceLines(down: Bool) {
        let range = selectedLines(), selection = selectedRange(), source = string as NSString
        let block = source.substring(with: range)
        let separator = block.endsInSourceNewline ? "" : newline
        let inserted = down ? separator + block : block + separator
        let location = down ? NSMaxRange(range) : range.location
        let offset = down ? range.length + (separator as NSString).length : 0
        replaceSource(NSRange(location: location, length: 0), with: inserted,
                      selection: NSRange(location: selection.location + offset, length: selection.length), action: "Copy Line")
    }

    func moveSourceLines(down: Bool) {
        let source = string as NSString, range = selectedLines(), selection = selectedRange()
        guard down ? NSMaxRange(range) < source.length : range.location > 0 else { return }
        let adjacent = source.lineRange(for: NSRange(location: down ? NSMaxRange(range) : range.location - 1, length: 0))
        let first = down ? range : adjacent, second = down ? adjacent : range
        let a = source.substring(with: first), b = source.substring(with: second)
        let eol = a.hasSuffix("\r\n") ? "\r\n" : a.hasSuffix("\n") ? "\n" : "\r"
        let bTerminated = b.endsInSourceNewline
        let replacement = bTerminated ? b + a : b + eol + String(a.dropLast(eol.count))
        let delta = down ? (b as NSString).length + (bTerminated ? 0 : (eol as NSString).length) : -adjacent.length
        replaceSource(NSUnionRange(first, second), with: replacement,
                      selection: NSRange(location: selection.location + delta, length: min(selection.length, (replacement as NSString).length - (selection.location + delta - first.location))), action: "Move Line")
    }

    func transformSourceLines(_ transform: (String) -> String, action: String) {
        let source = string as NSString, range = selectedLines(), selection = selectedRange()
        let block = source.substring(with: range) as NSString
        var cursor = 0, output = "", edits: [(NSRange, Int)] = []
        repeat {
            var start = 0, end = 0, contentEnd = 0
            block.getLineStart(&start, end: &end, contentsEnd: &contentEnd, for: NSRange(location: cursor, length: 0))
            let line = block.substring(with: NSRange(location: start, length: contentEnd - start))
            let changed = transform(line)
            // These transforms only add/remove a prefix; preserve caret columns.
            edits.append((NSRange(location: range.location + start, length: max(0, (line as NSString).length - (changed as NSString).length)), (changed as NSString).length - (line as NSString).length))
            output += changed + block.substring(with: NSRange(location: contentEnd, length: end - contentEnd))
            cursor = end
        } while cursor < block.length
        func mapped(_ position: Int) -> Int {
            var result = position
            for (edit, delta) in edits where position >= edit.location {
                result += delta >= 0 ? delta : -min(-delta, position - edit.location)
            }
            return result
        }
        let start = mapped(selection.location), end = mapped(NSMaxRange(selection))
        replaceSource(range, with: output, selection: NSRange(location: start, length: end - start), action: action)
    }

    func indentSourceLines(outdent: Bool) {
        transformSourceLines({ line in
            if !outdent { return "  " + line }
            if line.hasPrefix("\t") { return String(line.dropFirst()) }
            return String(line.dropFirst(line.prefix(2).prefix(while: { $0 == " " }).count))
        }, action: outdent ? "Outdent" : "Indent")
    }

    func toggleSourceComments() {
        let marker = sourceFormat == .latex ? "%" : "//"
        let lines = (string as NSString).substring(with: selectedLines()).components(separatedBy: .newlines).filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
        let uncomment = !lines.isEmpty && lines.allSatisfy { $0.trimmingCharacters(in: .whitespaces).hasPrefix(marker) }
        transformSourceLines({ line in
            if uncomment {
                let indent = String(line.prefix(while: { $0 == " " || $0 == "\t" }))
                let rest = String(line.dropFirst(indent.count))
                guard rest.hasPrefix(marker) else { return line }
                var value = String(rest.dropFirst(marker.count)); if value.hasPrefix(" ") { value.removeFirst() }
                return indent + value
            }
            return marker + " " + line
        }, action: "Toggle Line Comment")
    }

    func insertSourceLine(above: Bool) {
        let source = string as NSString, range = source.lineRange(for: NSRange(location: selectedRange().location, length: 0))
        let line = source.substring(with: range)
        let indent = String(line.prefix(while: { $0 == " " || $0 == "\t" }))
        let location = above ? range.location : NSMaxRange(range)
        let terminated = line.endsInSourceNewline
        let value = above || terminated ? indent + newline : newline + indent
        let caret = location + (indent as NSString).length + (!above && !terminated ? (newline as NSString).length : 0)
        replaceSource(NSRange(location: location, length: 0), with: value, selection: NSRange(location: caret, length: 0), action: "Insert Line")
    }

    /// Run only for the focused source view, before application menu shortcuts.
    func handleSourceKey(_ event: NSEvent) -> Bool {
        guard !hasMarkedText() else { return false }
        let flags = event.modifierFlags.intersection([.command, .shift, .option, .control])
        if flags == .command && event.keyCode == 51 { deleteToBeginningOfLine(nil); return true }
        if (flags == .option || flags == [.option, .shift]), [125, 126].contains(event.keyCode) {
            if flags.contains(.shift) { duplicateSourceLines(down: event.keyCode == 125) }
            else { moveSourceLines(down: event.keyCode == 125) }; return true
        }
        if event.keyCode == 36 && (flags == .command || flags == [.command, .shift]) {
            insertSourceLine(above: flags.contains(.shift)); return true
        }
        let key = event.charactersIgnoringModifiers?.lowercased()
        if key == "z", flags == .command || flags == [.command, .shift] {
            flags.contains(.shift) ? redo(nil) : undo(nil)
            return true
        }
        if flags == [.command, .shift], key == "k" { deleteSourceLines(); return true }
        guard flags == .command else { return false }
        switch key {
        case "c": copy(nil)
        case "x": cut(nil)
        case "v": paste(nil)
        case "[": indentSourceLines(outdent: true)
        case "]": indentSourceLines(outdent: false)
        case "/": toggleSourceComments()
        case "f":
            let item = NSMenuItem(); item.tag = Int(NSFindPanelAction.showFindPanel.rawValue); performFindPanelAction(item)
        default: return false
        }
        return true
    }
}
