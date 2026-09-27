import AppKit
import SwiftUI
import UniformTypeIdentifiers

private final class FileTreeItem: NSObject {
    let node: ProjectFileNode
    let children: [FileTreeItem]
    init(_ node: ProjectFileNode) { self.node = node; children = node.children.map(FileTreeItem.init) }
}

private enum FileTreeAction: Int {
    case newFile, newFolder, rename, copy, cut, paste, duplicate, trash, relativePath, fullPath, reveal, openDefault, otherApp, undo, redo
}

struct NativeProjectFileTree: NSViewRepresentable {
    @ObservedObject var files: ProjectFiles
    let create: (String, Bool) -> Void
    func makeCoordinator() -> Coordinator { Coordinator(files: files, create: create) }
    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView(); scroll.hasVerticalScroller = true; scroll.drawsBackground = false
        let outline = FileOutlineView()
        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("name"))
        column.resizingMask = .autoresizingMask; outline.addTableColumn(column); outline.outlineTableColumn = column
        outline.headerView = nil; outline.rowHeight = 28; outline.indentationPerLevel = 13
        outline.allowsMultipleSelection = true; outline.allowsEmptySelection = true
        outline.backgroundColor = .clear; outline.style = .sourceList
        outline.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
        outline.treeCoordinator = context.coordinator
        outline.dataSource = context.coordinator; outline.delegate = context.coordinator
        outline.target = context.coordinator; outline.doubleAction = #selector(Coordinator.openItem)
        outline.registerForDraggedTypes([.fileURL])
        outline.setDraggingSourceOperationMask([.copy, .move], forLocal: true)
        outline.setDraggingSourceOperationMask(.copy, forLocal: false)
        outline.setAccessibilityLabel("Project Files")
        scroll.documentView = outline; context.coordinator.outline = outline
        return scroll
    }
    func updateNSView(_ view: NSScrollView, context: Context) { context.coordinator.update() }

    @MainActor final class Coordinator: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate, NSTextFieldDelegate {
        let files: ProjectFiles
        let create: (String, Bool) -> Void
        weak var outline: FileOutlineView?
        private var items: [FileTreeItem] = []
        private var byPath: [String: FileTreeItem] = [:]
        private var version = -1
        private var changing = false
        private var renaming: String?
        private var cancellingRename = false
        init(files: ProjectFiles, create: @escaping (String, Bool) -> Void) { self.files = files; self.create = create }
        func update() {
            guard let outline, renaming == nil else { return }
            changing = true
            if version != files.treeVersion {
                items = files.tree.map(FileTreeItem.init); byPath = [:]
                func index(_ items: [FileTreeItem]) { for item in items { byPath[item.node.path] = item; index(item.children) } }
                index(items); outline.reloadData(); version = files.treeVersion
            }
            for path in files.expanded.sorted() { if let item = byPath[path] { outline.expandItem(item) } }
            let rows = IndexSet(files.treeSelection.compactMap { byPath[$0].map { outline.row(forItem: $0) } }.filter { $0 >= 0 })
            if rows != outline.selectedRowIndexes { outline.selectRowIndexes(rows, byExtendingSelection: false) }
            for row in 0..<outline.numberOfRows {
                guard let item = outline.item(atRow: row) as? FileTreeItem,
                      let cell = outline.view(atColumn: 0, row: row, makeIfNecessary: false) as? FileNameCell else { continue }
                cell.dirty.isHidden = !files.buffers.contains { $0.path == item.node.path && $0.dirty }
            }
            changing = false
        }
        func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int { (item as? FileTreeItem)?.children.count ?? items.count }
        func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any { ((item as? FileTreeItem)?.children ?? items)[index] }
        func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool { (item as? FileTreeItem)?.node.directory == true }
        func outlineView(_ outlineView: NSOutlineView, viewFor tableColumn: NSTableColumn?, item: Any) -> NSView? {
            guard let item = item as? FileTreeItem else { return nil }
            let cell = FileNameCell()
            cell.textField?.stringValue = item.node.name; cell.textField?.delegate = self
            cell.textField?.setAccessibilityLabel(item.node.path)
            cell.imageView?.image = NSImage(systemSymbolName: item.node.directory ? "folder" : "doc.text", accessibilityDescription: item.node.directory ? "Folder" : "File")
            cell.toolTip = item.node.path
            cell.dirty.isHidden = !files.buffers.contains { $0.path == item.node.path && $0.dirty }
            return cell
        }
        func outlineViewSelectionDidChange(_ notification: Notification) {
            guard !changing, let outline else { return }
            files.treeSelection = Set(outline.selectedRowIndexes.compactMap { (outline.item(atRow: $0) as? FileTreeItem)?.node.path })
            if files.treeSelection.count == 1, let path = files.treeSelection.first, !files.isDirectory(path), isEditable(path) {
                files.open(path, focusEditor: false)
            }
        }
        func outlineViewItemDidExpand(_ notification: Notification) {
            guard !changing, let item = notification.userInfo?["NSObject"] as? FileTreeItem else { return }
            files.expanded.insert(item.node.path)
        }
        func outlineViewItemDidCollapse(_ notification: Notification) {
            guard !changing, let item = notification.userInfo?["NSObject"] as? FileTreeItem else { return }
            files.expanded.remove(item.node.path)
        }
        private func isEditable(_ path: String) -> Bool {
            ProjectFiles.extensions.contains((path as NSString).pathExtension.lowercased()) || ["latexmkrc", ".latexmkrc", "Makefile"].contains((path as NSString).lastPathComponent)
        }
        @objc func openItem() {
            guard let path = files.selectedTreePaths.first else { return }
            if files.isDirectory(path), let item = byPath[path], let outline {
                if outline.isItemExpanded(item) { outline.collapseItem(item) } else { outline.expandItem(item) }
            } else if isEditable(path) {
                files.open(path)
                if let view = files.controller(path).view { view.window?.makeFirstResponder(view) }
            } else { openDefault() }
        }
        func beginRename() {
            guard files.treeSelection.count == 1, let path = files.treeSelection.first, let item = byPath[path], let outline else { return }
            let row = outline.row(forItem: item)
            guard row >= 0, let cell = outline.view(atColumn: 0, row: row, makeIfNecessary: true) as? FileNameCell, let field = cell.textField else { return }
            renaming = path; cancellingRename = false; field.isEditable = true; field.isSelectable = true
            outline.window?.makeFirstResponder(field)
            if let editor = field.currentEditor() {
                let name = item.node.name as NSString
                let length = item.node.directory || name.pathExtension.isEmpty ? name.length : (name.deletingPathExtension as NSString).length
                editor.selectedRange = NSRange(location: 0, length: length)
            }
        }
        func control(_ control: NSControl, textView: NSTextView, doCommandBy commandSelector: Selector) -> Bool {
            if commandSelector == #selector(NSResponder.cancelOperation(_:)) {
                cancellingRename = true; outline?.window?.makeFirstResponder(outline); return true
            }
            if commandSelector == #selector(NSResponder.insertNewline(_:)) {
                outline?.window?.makeFirstResponder(outline); return true
            }
            return false
        }
        func controlTextDidEndEditing(_ notification: Notification) {
            guard let path = renaming, let field = notification.object as? NSTextField else { return }
            renaming = nil; field.isEditable = false; field.isSelectable = false
            if !cancellingRename { files.attempt { try files.rename(path, to: field.stringValue) } }
            version = -1; update()
        }
        func menu(for event: NSEvent) -> NSMenu {
            guard let outline else { return NSMenu() }
            let row = outline.row(at: outline.convert(event.locationInWindow, from: nil))
            if row >= 0, !outline.selectedRowIndexes.contains(row) { outline.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false) }
            if row < 0 { outline.deselectAll(nil); files.treeSelection = [] }
            outline.window?.makeFirstResponder(outline)
            let menu = NSMenu(); menu.autoenablesItems = false
            let selected = !files.treeSelection.isEmpty
            func add(_ title: String, _ action: FileTreeAction, _ key: String = "", enabled: Bool = true) {
                let item = NSMenuItem(title: title, action: #selector(performAction(_:)), keyEquivalent: key)
                item.target = self; item.tag = action.rawValue; item.isEnabled = enabled; menu.addItem(item)
            }
            if files.treeSelection.count <= 1 {
                add("New File…", .newFile); add("New Folder…", .newFolder); menu.addItem(.separator())
            }
            if selected {
                add("Rename", .rename, enabled: files.treeSelection.count == 1)
                add("Cut", .cut, "x"); add("Copy", .copy, "c")
            }
            add("Paste", .paste, "v", enabled: !ProjectFiles.clipboardURLs(.general).isEmpty)
            if selected {
                add("Duplicate", .duplicate, "d"); menu.addItem(.separator())
                add("Copy Relative Path", .relativePath); add("Copy Path", .fullPath)
                add("Reveal in Finder", .reveal)
                let open = NSMenuItem(title: "Open With", action: nil, keyEquivalent: "")
                open.submenu = openWithMenu(); menu.addItem(open)
                menu.addItem(.separator()); add("Move to Trash", .trash, "\u{8}")
            }
            if let title = files.undoFileTitle {
                menu.addItem(.separator()); add("Undo " + title, .undo, "z")
            }
            if let title = files.redoFileTitle { add("Redo " + title, .redo) }
            return menu
        }
        @objc private func performAction(_ sender: NSMenuItem) {
            guard let action = FileTreeAction(rawValue: sender.tag) else { return }
            switch action {
            case .newFile: create(files.destinationFolder, false)
            case .newFolder: create(files.destinationFolder, true)
            case .rename: beginRename()
            case .copy: files.copyTreeSelection(cut: false)
            case .cut: files.copyTreeSelection(cut: true)
            case .paste: files.attempt { try files.pasteTreeSelection() }
            case .duplicate: files.attempt { try files.duplicateSelection() }
            case .trash: files.attempt { try files.trashSelection() }
            case .relativePath: files.copyTreePaths(relative: true)
            case .fullPath: files.copyTreePaths(relative: false)
            case .reveal: files.revealTreeSelection()
            case .openDefault: openDefault()
            case .otherApp: chooseApp()
            case .undo: files.undoFileOperation()
            case .redo: files.redoFileOperation()
            }
            update()
        }
        private var selectedURLs: [URL] { files.selectedTreePaths.compactMap { try? files.entryURL($0) } }
        private func openDefault() { for url in selectedURLs { NSWorkspace.shared.open(url) } }
        private func openWithMenu() -> NSMenu {
            let menu = NSMenu()
            let defaultItem = NSMenuItem(title: "Default Application", action: #selector(performAction(_:)), keyEquivalent: "")
            defaultItem.tag = FileTreeAction.openDefault.rawValue; defaultItem.target = self; menu.addItem(defaultItem)
            let urls = selectedURLs
            if let first = urls.first {
                var apps = Set(NSWorkspace.shared.urlsForApplications(toOpen: first))
                for url in urls.dropFirst() { apps.formIntersection(NSWorkspace.shared.urlsForApplications(toOpen: url)) }
                for app in apps.sorted(by: { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }) {
                    let item = NSMenuItem(title: app.deletingPathExtension().lastPathComponent, action: #selector(openWithApp(_:)), keyEquivalent: "")
                    item.target = self; item.representedObject = app; menu.addItem(item)
                }
            }
            menu.addItem(.separator())
            let other = NSMenuItem(title: "Other…", action: #selector(performAction(_:)), keyEquivalent: "")
            other.tag = FileTreeAction.otherApp.rawValue; other.target = self; menu.addItem(other)
            return menu
        }
        @objc private func openWithApp(_ sender: NSMenuItem) { if let app = sender.representedObject as? URL { launch(app, files: selectedURLs) } }
        private func launch(_ app: URL, files urls: [URL]) {
            NSWorkspace.shared.open(urls, withApplicationAt: app, configuration: .init()) { [weak self] _, error in
                if let error { DispatchQueue.main.async { self?.files.error = error.localizedDescription } }
            }
        }
        private func chooseApp() {
            guard let window = outline?.window else { return }
            let panel = NSOpenPanel(); panel.allowedContentTypes = [.applicationBundle]
            panel.directoryURL = URL(fileURLWithPath: "/Applications"); panel.prompt = "Open With"
            let urls = selectedURLs
            panel.beginSheetModal(for: window) { [weak self] response in
                if response == .OK, let app = panel.url { self?.launch(app, files: urls) }
            }
        }
        func outlineView(_ outlineView: NSOutlineView, pasteboardWriterForItem item: Any) -> NSPasteboardWriting? {
            guard let item = item as? FileTreeItem else { return nil }
            return (try? files.entryURL(item.node.path)) as NSURL?
        }
        func outlineView(_ outlineView: NSOutlineView, validateDrop info: NSDraggingInfo, proposedItem item: Any?, proposedChildIndex index: Int) -> NSDragOperation {
            guard !(info.draggingPasteboard.types ?? []).isEmpty else { return [] }
            var destination = item as? FileTreeItem
            if let current = destination, !current.node.directory {
                let parent = (current.node.path as NSString).deletingLastPathComponent
                destination = byPath[parent]
            }
            outlineView.setDropItem(destination, dropChildIndex: NSOutlineViewDropOnItemIndex)
            let urls = ProjectFiles.clipboardURLs(info.draggingPasteboard)
            guard !urls.isEmpty else { return [] }
            let target = destination.flatMap { try? files.resolved($0.node.path) } ?? files.root
            guard let target, urls.allSatisfy({ target != $0.resolvingSymlinksInPath() && !target.path.hasPrefix($0.resolvingSymlinksInPath().path + "/") }) else { return [] }
            let local = (info.draggingSource as? NSOutlineView) === outlineView
            return local && !NSEvent.modifierFlags.contains(.option) ? .move : .copy
        }
        func outlineView(_ outlineView: NSOutlineView, acceptDrop info: NSDraggingInfo, item: Any?, childIndex index: Int) -> Bool {
            let local = (info.draggingSource as? NSOutlineView) === outlineView
            let moving = local && !NSEvent.modifierFlags.contains(.option)
            do {
                try files.transfer(ProjectFiles.clipboardURLs(info.draggingPasteboard), to: (item as? FileTreeItem)?.node.path ?? "", move: moving)
                files.error = nil; update(); return true
            } catch { files.error = error.localizedDescription; return false }
        }
    }
}

private final class FileNameCell: NSTableCellView {
    let dirty = NSImageView()
    override init(frame: NSRect) {
        super.init(frame: frame)
        let icon = NSImageView(), name = NSTextField(labelWithString: "")
        name.font = .systemFont(ofSize: 12); name.lineBreakMode = .byTruncatingMiddle
        name.isEditable = false; name.isSelectable = false
        name.cell?.wraps = false; name.cell?.isScrollable = true
        icon.contentTintColor = .secondaryLabelColor
        dirty.image = NSImage(systemSymbolName: "circle.fill", accessibilityDescription: "Unsaved")
        dirty.contentTintColor = .controlAccentColor
        for view in [icon, name, dirty] { view.translatesAutoresizingMaskIntoConstraints = false; addSubview(view) }
        imageView = icon; textField = name
        NSLayoutConstraint.activate([
            icon.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 2), icon.centerYAnchor.constraint(equalTo: centerYAnchor), icon.widthAnchor.constraint(equalToConstant: 15), icon.heightAnchor.constraint(equalToConstant: 15),
            name.leadingAnchor.constraint(equalTo: icon.trailingAnchor, constant: 6), name.centerYAnchor.constraint(equalTo: centerYAnchor), name.trailingAnchor.constraint(equalTo: dirty.leadingAnchor, constant: -4),
            dirty.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4), dirty.centerYAnchor.constraint(equalTo: centerYAnchor), dirty.widthAnchor.constraint(equalToConstant: 5), dirty.heightAnchor.constraint(equalToConstant: 5)
        ])
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
}

final class FileOutlineView: NSOutlineView, NSMenuItemValidation {
    fileprivate weak var treeCoordinator: NativeProjectFileTree.Coordinator?
    override func menu(for event: NSEvent) -> NSMenu? { treeCoordinator?.menu(for: event) }
    @objc func copy(_ sender: Any?) { treeCoordinator?.files.copyTreeSelection(cut: false) }
    @objc func cut(_ sender: Any?) { treeCoordinator?.files.copyTreeSelection(cut: true) }
    @objc func paste(_ sender: Any?) { if let files = treeCoordinator?.files { files.attempt { try files.pasteTreeSelection() } } }
    @objc func undo(_ sender: Any?) { treeCoordinator?.files.undoFileOperation() }
    @objc func redo(_ sender: Any?) { treeCoordinator?.files.redoFileOperation() }
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard window?.firstResponder === self else { return super.performKeyEquivalent(with: event) }
        return handleKey(event) || super.performKeyEquivalent(with: event)
    }
    override func keyDown(with event: NSEvent) { if !handleKey(event) { super.keyDown(with: event) } }
    private func handleKey(_ event: NSEvent) -> Bool {
        guard let coordinator = treeCoordinator else { return false }
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if flags.contains(.command), !flags.contains(.option), !flags.contains(.control) {
            switch event.charactersIgnoringModifiers?.lowercased() {
            case "c": copy(nil)
            case "x": cut(nil)
            case "v": paste(nil)
            case "a": selectAll(nil)
            case "z": flags.contains(.shift) ? redo(nil) : undo(nil)
            case "d": coordinator.files.attempt { try coordinator.files.duplicateSelection() }
            default:
                guard event.keyCode == 51 else { return false }
                coordinator.files.attempt { try coordinator.files.trashSelection() }
            }
            return true
        }
        if [36, 120].contains(event.keyCode), flags.intersection([.command, .option, .control, .shift]).isEmpty { coordinator.beginRename(); return true }
        return false
    }
    func validateMenuItem(_ menuItem: NSMenuItem) -> Bool {
        guard let files = treeCoordinator?.files else { return false }
        switch menuItem.action {
        case #selector(copy(_:)), #selector(cut(_:)): return !files.treeSelection.isEmpty
        case #selector(paste(_:)): return !ProjectFiles.clipboardURLs(.general).isEmpty
        case #selector(undo(_:)):
            menuItem.title = files.undoFileTitle.map { "Undo " + $0 } ?? "Undo"; return files.undoFileTitle != nil
        case #selector(redo(_:)):
            menuItem.title = files.redoFileTitle.map { "Redo " + $0 } ?? "Redo"; return files.redoFileTitle != nil
        default: return true
        }
    }
}
