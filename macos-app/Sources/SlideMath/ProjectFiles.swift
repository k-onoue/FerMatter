import AppKit
import CryptoKit
import SwiftUI
import Combine

struct ProjectFileNode: Identifiable {
    let path: String
    let directory: Bool
    let children: [ProjectFileNode]
    var id: String { path }
    var name: String { URL(fileURLWithPath: path).lastPathComponent }
}

struct SourceFileBuffer: Codable, Identifiable {
    var path: String
    var text: String
    var original: Data
    var hasBOM: Bool
    var crlf: Bool
    var id: String { path }
    var encoded: Data {
        let normalized = text.replacingOccurrences(of: "\r\n", with: "\n")
        let body = crlf ? normalized.replacingOccurrences(of: "\n", with: "\r\n") : text
        return (hasBOM ? Data([0xef, 0xbb, 0xbf]) : Data()) + Data(body.utf8)
    }
    var dirty: Bool { encoded != original }
}

private struct SourceEditorKey: Hashable {
    let pane: Int
    let path: String
}

enum ProjectFileError: LocalizedError {
    case outsideProject, unsupported, tooLarge, conflict, exists, invalidName
    var errorDescription: String? {
        switch self {
        case .outsideProject: "This path is outside the project."
        case .unsupported: "This file is not editable UTF-8 text."
        case .tooLarge: "This file is too large for the source editor (2 MB maximum)."
        case .conflict: "This file changed on disk. Your edits are kept. Reload the file or save a copy."
        case .exists: "A file or folder with this name already exists."
        case .invalidName: "Enter a file or folder name without path separators."
        }
    }
}

@MainActor
final class ProjectFiles: ObservableObject {
    @Published private(set) var root: URL?
    @Published private(set) var tree: [ProjectFileNode] = []
    @Published private(set) var buffers: [SourceFileBuffer] = []
    @Published private(set) var paneTabs: [Int: [String]] = [0: []]
    @Published var draggingSourceTab: SourceTabDrag?
    @Published var pendingTabClose: SourceTabClose?
    @Published private(set) var paneSelections: [Int: String] = [:]
    @Published private(set) var sourceLayout = SourcePaneLayout.pane(0)
    private var nextPaneID = 1
    var splitSource: Bool { sourceLayout.panes.count > 1 }
    var selected: String? { currentPath }
    @Published var activeSourcePane = 0
    @Published var expanded = Set<String>()
    @Published var treeSelection = Set<String>()
    @Published private(set) var treeVersion = 0
    @Published var error: String?
    @Published private(set) var conflicts = Set<String>()
    @Published private(set) var completions: [SourceCompletion] = []
    var format: ProjectFormat = .latex
    var onSave: (() -> Void)?
    var onRelocate: (([FileRelocation]) -> Void)?
    let editorPreferences: EditorPreferences
    private var preferenceObservation: AnyCancellable?
    private var autoSaveTask: DispatchWorkItem?
    private var pendingAutoSaves = Set<String>()
    var wantsEditorFocus = true
    private var undoFiles: [FileHistoryEntry] = []
    private var redoFiles: [FileHistoryEntry] = []
    var operations: FileOperations?
    private var cutClipboardChange: Int?
    private var cutPaths: [String] = []
    var undoFileTitle: String? { undoFiles.last?.title }
    var redoFileTitle: String? { redoFiles.last?.title }
    private let support: URL
    private var controllers: [SourceEditorKey: SourceEditorController] = [:]
    private var persistTask: DispatchWorkItem?
    private var indexTask: Task<Void, Never>?
    private var diskCompletions: [SourceCompletion] = []
    private var unreadableDrafts = false
    static let excluded = Set([".git", ".build", "build", "node_modules", "__pycache__", "Cache", "Exports", ".DS_Store"])
    static let extensions = Set(["tex", "sty", "cls", "bib", "typ", "txt", "md", "json", "toml", "yaml", "yml", "csv", "bst", "cfg", "def", "clo", "ist"])

    init(support: URL, editorPreferences: EditorPreferences? = nil) {
        self.support = support
        self.editorPreferences = editorPreferences ?? EditorPreferences()
        preferenceObservation = self.editorPreferences.$saveMode.combineLatest(self.editorPreferences.$delay)
            .dropFirst().receive(on: RunLoop.main).sink { [weak self] mode, _ in
                guard let self else { return }
                self.autoSaveTask?.cancel(); self.autoSaveTask = nil
                self.pendingAutoSaves = mode == .afterDelay ? Set(self.buffers.filter(\.dirty).map(\.path)) : []
                self.scheduleAutoSave()
            }
    }
    var currentPath: String? { path(in: activeSourcePane) }
    func path(in pane: Int) -> String? { paneSelections[pane] }
    var current: SourceFileBuffer? { buffers.first { $0.path == currentPath } }
    var dirtyCount: Int { buffers.filter(\.dirty).count }
    var draftURL: URL? {
        root.map { root in
            let key = SHA256.hash(data: Data(root.path.utf8)).map { String(format: "%02x", $0) }.joined()
            return support.appendingPathComponent("SourceDrafts/\(key).json")
        }
    }

    func configure(root: URL, main: String, format: ProjectFormat) {
        let root = root.resolvingSymlinksInPath()
        guard self.root != root else { self.format = format; refresh(); return }
        flushAutoSaves(); persistDrafts(); indexTask?.cancel()
        autoSaveTask?.cancel(); autoSaveTask = nil; pendingAutoSaves = []
        self.root = root; self.format = format
        operations = FileOperations(root: root); undoFiles = []; redoFiles = []; treeSelection = []
        paneTabs = [0: []]; pendingTabClose = nil; paneSelections = [:]; sourceLayout = .pane(0); activeSourcePane = 0; nextPaneID = 1
        cutPaths = []; cutClipboardChange = nil
        buffers = []; controllers = [:]; expanded = []; conflicts = []; error = nil
        unreadableDrafts = false
        diskCompletions = []; completions = []
        if let draftURL, let data = try? Data(contentsOf: draftURL) {
            do { buffers = try JSONDecoder().decode([SourceFileBuffer].self, from: data) }
            catch { unreadableDrafts = true; self.error = "Could not read saved source drafts. The draft file has been kept." }
        }
        paneTabs[0] = buffers.map(\.path)
        refresh()
        if buffers.contains(where: { $0.path == main }) { select(main) }
        else if (try? resolved(main)) != nil { open(main) }
    }

    func resolved(_ path: String) throws -> URL {
        guard let root, !path.isEmpty, !path.hasPrefix("/"), !path.split(separator: "/").contains("..") else { throw ProjectFileError.outsideProject }
        let url = root.appendingPathComponent(path).resolvingSymlinksInPath().standardizedFileURL
        guard url.path.hasPrefix(root.path + "/") else { throw ProjectFileError.outsideProject }
        return url
    }

    func refresh() {
        guard let root else { return }
        var count = 0
        func nodes(_ folder: URL, prefix: String, depth: Int) -> [ProjectFileNode] {
            guard depth < 30, count < 10_000 else { return [] }
            let keys: Set<URLResourceKey> = [.isDirectoryKey, .isSymbolicLinkKey]
            guard let urls = try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: Array(keys)) else { return [] }
            return urls.compactMap { url -> ProjectFileNode? in
                let name = url.lastPathComponent
                guard !Self.excluded.contains(name), !name.hasPrefix(".") || name == ".latexmkrc", count < 10_000,
                      let info = try? url.resourceValues(forKeys: keys) else { return nil }
                count += 1
                let path = prefix + name, directory = info.isDirectory == true
                return ProjectFileNode(path: path, directory: directory,
                    children: directory && info.isSymbolicLink != true ? nodes(url, prefix: path + "/", depth: depth + 1) : [])
            }.sorted { $0.directory != $1.directory ? $0.directory : $0.name.localizedStandardCompare($1.name) == .orderedAscending }
        }
        tree = nodes(root, prefix: "", depth: 0); treeVersion += 1
        treeSelection = treeSelection.filter { (try? entryURL($0)).map(FileOperations.exists) == true }
        for index in buffers.indices {
            guard let url = try? resolved(buffers[index].path), let data = try? Data(contentsOf: url) else {
                conflicts.insert(buffers[index].path); continue
            }
            guard data != buffers[index].original else { conflicts.remove(buffers[index].path); continue }
            if buffers[index].dirty { conflicts.insert(buffers[index].path) }
            else if let updated = try? Self.buffer(path: buffers[index].path, data: data) { buffers[index] = updated; conflicts.remove(updated.path) }
        }
        indexTask?.cancel()
        let format = self.format
        indexTask = Task { [weak self] in
            let items = await Task.detached { ProjectCompletions.load(root: root, format: format) }.value
            guard !Task.isCancelled, let self, self.root == root else { return }
            self.diskCompletions = items; self.updateCompletions()
        }
    }

    static func buffer(path: String, data: Data) throws -> SourceFileBuffer {
        guard data.count <= 2_000_000 else { throw ProjectFileError.tooLarge }
        let bom = data.starts(with: [0xef, 0xbb, 0xbf])
        guard let text = String(data: bom ? data.dropFirst(3) : data, encoding: .utf8), !text.contains("\0") else { throw ProjectFileError.unsupported }
        let crlf = text.contains("\r\n") && !text.replacingOccurrences(of: "\r\n", with: "").contains("\n")
        return SourceFileBuffer(path: path, text: text, original: data, hasBOM: bom, crlf: crlf)
    }

    func open(_ path: String, line: Int? = nil, focusEditor: Bool = true) {
        do {
            if !buffers.contains(where: { $0.path == path }) {
                let url = try resolved(path)
                guard Self.extensions.contains(url.pathExtension.lowercased()) || ["latexmkrc", ".latexmkrc", "Makefile"].contains(url.lastPathComponent) else { throw ProjectFileError.unsupported }
                let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
                guard size <= 2_000_000 else { throw ProjectFileError.tooLarge }
                buffers.append(try Self.buffer(path: path, data: Data(contentsOf: url)))
            }
            select(path, focusEditor: focusEditor); error = nil
            if let line { DispatchQueue.main.async { [weak self] in self?.controller(path).reveal(line: line) } }
        } catch { self.error = error.localizedDescription }
    }

    func select(_ path: String, focusEditor: Bool = true, pane: Int? = nil) {
        wantsEditorFocus = focusEditor
        if let pane { activeSourcePane = pane }
        if !(paneTabs[activeSourcePane] ?? []).contains(path) { paneTabs[activeSourcePane, default: []].append(path) }
        paneSelections[activeSourcePane] = path
        if focusEditor { treeSelection = [path] }
        let parts = path.split(separator: "/")
        for index in 1..<parts.count { expanded.insert(parts.prefix(index).joined(separator: "/")) }
    }

    func controller(_ path: String, pane: Int? = nil) -> SourceEditorController {
        let key = SourceEditorKey(pane: pane ?? activeSourcePane, path: path)
        if let controller = controllers[key] { return controller }
        let controller = SourceEditorController(); controllers[key] = controller; return controller
    }

    func update(_ path: String, text: String) {
        guard let index = buffers.firstIndex(where: { $0.path == path }), buffers[index].text != text else { return }
        buffers[index].text = text; updateCompletions()
        if editorPreferences.saveMode == .afterDelay { pendingAutoSaves.insert(path); scheduleAutoSave() }
        persistTask?.cancel()
        let work = DispatchWorkItem { [weak self] in self?.persistDrafts() }
        persistTask = work; DispatchQueue.main.asyncAfter(deadline: .now() + 0.5, execute: work)
    }

    private func scheduleAutoSave() {
        autoSaveTask?.cancel()
        guard editorPreferences.saveMode == .afterDelay, !pendingAutoSaves.isEmpty else { return }
        let project = root
        let work = DispatchWorkItem { [weak self] in
            guard let self, self.root == project else { return }
            self.flushAutoSaves()
        }
        autoSaveTask = work
        DispatchQueue.main.asyncAfter(deadline: .now() + editorPreferences.delay, execute: work)
    }

    func flushAutoSaves() {
        autoSaveTask?.cancel(); autoSaveTask = nil
        guard editorPreferences.saveMode == .afterDelay else { pendingAutoSaves = []; return }
        let paths = pendingAutoSaves.sorted(); pendingAutoSaves = []
        var saved = false
        for path in paths where buffers.contains(where: { $0.path == path && $0.dirty }) {
            do { try save(path, notify: false); saved = true }
            catch { self.error = error.localizedDescription }
        }
        if saved { refresh(); onSave?() }
    }

    private func updateCompletions() {
        completions = ProjectCompletions.unique(buffers.flatMap { ProjectCompletions.definitions(in: $0.text, format: format, origin: $0.path) } + diskCompletions)
    }

    func save(_ path: String, notify: Bool = true) throws {
        pendingAutoSaves.remove(path)
        guard let index = buffers.firstIndex(where: { $0.path == path }), buffers[index].dirty else { return }
        let url = try resolved(path)
        guard try Data(contentsOf: url) == buffers[index].original else { conflicts.insert(path); throw ProjectFileError.conflict }
        let data = buffers[index].encoded
        try data.write(to: url, options: .atomic)
        buffers[index].original = data; conflicts.remove(path)
        persistDrafts()
        if notify { refresh(); onSave?() }
    }

    func saveCurrent() {
        guard let selected = currentPath else { return }
        do { try save(selected); error = nil } catch { self.error = error.localizedDescription }
    }

    func reload(_ path: String) {
        pendingAutoSaves.remove(path)
        do {
            guard let index = buffers.firstIndex(where: { $0.path == path }) else { return }
            buffers[index] = try Self.buffer(path: path, data: Data(contentsOf: resolved(path)))
            conflicts.remove(path); error = nil; updateCompletions(); persistDrafts()
        } catch { self.error = error.localizedDescription }
    }

    func saveCopy() {
        guard let current else { return }
        let panel = NSSavePanel(); panel.nameFieldStringValue = URL(fileURLWithPath: current.path).lastPathComponent
        guard let window = controller(current.path).view?.window ?? NSApp.keyWindow else { return }
        panel.beginSheetModal(for: window) { [weak self] response in
            guard response == .OK, let url = panel.url, let self else { return }
            self.attempt {
                guard url.resolvingSymlinksInPath() != (try self.resolved(current.path)) else { throw ProjectFileError.conflict }
                try current.encoded.write(to: url, options: .atomic)
            }
        }
    }

    @discardableResult func create(name: String, parent: String, directory: Bool) throws -> String {
        try validateName(name)
        let path = parent.isEmpty ? name : parent + "/" + name, url = try resolved(path)
        guard !FileOperations.exists(url) else { throw ProjectFileError.exists }
        try performFiles([.create(url, directory: directory)], title: directory ? "New Folder" : "New File")
        if !parent.isEmpty { expanded.insert(parent) }
        treeSelection = [path]; if !directory { open(path) }; return path
    }

    func persistDrafts() {
        persistTask?.cancel(); persistTask = nil
        guard let draftURL, !unreadableDrafts else { return }
        do {
            try FileManager.default.createDirectory(at: draftURL.deletingLastPathComponent(), withIntermediateDirectories: true)
            try JSONEncoder().encode(buffers.filter(\.dirty)).write(to: draftURL, options: .atomic)
        } catch { self.error = "Could not keep source drafts: " + error.localizedDescription }
    }
}

private struct FileHistoryEntry {
    let operations: [FileMutation]
    let title: String
    let buffers: [SourceFileBuffer]
    let selection: Set<String>
}

extension ProjectFiles {
    /// For tree operations, resolve the parent but operate on a symlink itself.
    func entryURL(_ path: String) throws -> URL {
        guard let root, !path.isEmpty, !path.hasPrefix("/"), !path.split(separator: "/").contains("..") else { throw ProjectFileError.outsideProject }
        let url = root.appendingPathComponent(path).standardizedFileURL
        let parent = url.deletingLastPathComponent().resolvingSymlinksInPath()
        guard parent == root || parent.path.hasPrefix(root.path + "/") else { throw ProjectFileError.outsideProject }
        return parent.appendingPathComponent(url.lastPathComponent)
    }
    func relativePath(_ url: URL) -> String? {
        guard let root, url.path.hasPrefix(root.path + "/") else { return nil }
        return String(url.path.dropFirst(root.path.count + 1))
    }
    func isDirectory(_ path: String) -> Bool {
        guard let url = try? entryURL(path) else { return false }
        return (try? url.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey])).map { $0.isDirectory == true && $0.isSymbolicLink != true } ?? false
    }
    var destinationFolder: String {
        guard treeSelection.count == 1, let path = treeSelection.first else { return "" }
        return isDirectory(path) ? path : (path as NSString).deletingLastPathComponent
    }
    var selectedTreePaths: [String] { treeSelection.sorted { $0.localizedStandardCompare($1) == .orderedAscending } }
    static func topLevel(_ paths: [String]) -> [String] {
        let unique = Set(paths)
        return unique.sorted().filter { path in !unique.contains { $0 != path && path.hasPrefix($0 + "/") } }
    }
    func validateName(_ name: String) throws {
        guard !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, name != ".", name != "..",
              !name.contains("/"), !name.contains("\\"), !name.contains(":"), !name.contains(where: { $0.isNewline || $0 == "\0" }) else { throw ProjectFileError.invalidName }
    }
    func attempt(_ action: () throws -> Void) {
        do { try action(); error = nil } catch { self.error = error.localizedDescription }
    }
    func rename(_ path: String, to name: String) throws {
        try validateName(name)
        let from = try entryURL(path), to = from.deletingLastPathComponent().appendingPathComponent(name)
        guard from.path != to.path else { return }
        try performFiles([.move(from, to)], title: "Rename")
        treeSelection = Set(relativePath(to).map { [$0] } ?? [])
    }
    private func uniqueDestination(_ url: URL, reserved: inout Set<String>) -> URL {
        var target = url, suffix = 2
        let isFolder = url.hasDirectoryPath
        let ext = isFolder ? "" : url.pathExtension
        let stem = ext.isEmpty ? url.lastPathComponent : url.deletingPathExtension().lastPathComponent
        while FileOperations.exists(target) || reserved.contains(target.path.lowercased()) {
            target = url.deletingLastPathComponent().appendingPathComponent(stem + " \(suffix)" + (ext.isEmpty ? "" : "." + ext), isDirectory: isFolder)
            suffix += 1
        }
        reserved.insert(target.path.lowercased()); return target
    }
    @discardableResult func transfer(_ sources: [URL], to folder: String, move: Bool) throws -> [URL] {
        guard let root else { throw ProjectFileError.outsideProject }
        let target = folder.isEmpty ? root : try resolved(folder)
        guard folder.isEmpty || isDirectory(folder) else { throw FileActionError.destinationMissing }
        let unique = Dictionary(sources.filter(\.isFileURL).map { ($0.standardizedFileURL.path, $0.standardizedFileURL) }, uniquingKeysWith: { first, _ in first })
        let paths = Self.topLevel(Array(unique.keys))
        var reserved = Set<String>(), mutations: [FileMutation] = [], destinations: [URL] = []
        for path in paths {
            let source = unique[path]!
            if move {
                guard let path = relativePath(source), try entryURL(path) == source else { throw ProjectFileError.outsideProject }
                if source.deletingLastPathComponent() == target { continue }
            }
            let physical = source.resolvingSymlinksInPath()
            guard target != physical, !target.path.hasPrefix(physical.path + "/") else { throw FileActionError.recursive }
            let directory = (try? source.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
            let destination = uniqueDestination(target.appendingPathComponent(source.lastPathComponent, isDirectory: directory), reserved: &reserved)
            mutations.append(move ? .move(source, destination) : .copy(source, destination)); destinations.append(destination)
        }
        guard !mutations.isEmpty else { return [] }
        try performFiles(mutations, title: move ? "Move" : "Copy")
        if !folder.isEmpty { expanded.insert(folder) }
        treeSelection = Set(destinations.compactMap(relativePath)); return destinations
    }
    func duplicateSelection() throws {
        var mutations: [FileMutation] = [], destinations: [URL] = [], reserved = Set<String>()
        for path in Self.topLevel(selectedTreePaths) {
            let source = try entryURL(path)
            let destination = uniqueDestination(URL(fileURLWithPath: source.path, isDirectory: isDirectory(path)), reserved: &reserved)
            mutations.append(.copy(source, destination)); destinations.append(destination)
        }
        guard !mutations.isEmpty else { return }
        try performFiles(mutations, title: "Duplicate"); treeSelection = Set(destinations.compactMap(relativePath))
    }
    func trashSelection() throws {
        let paths = Self.topLevel(selectedTreePaths)
        guard !paths.isEmpty else { return }
        try performFiles(paths.map { .trash(try entryURL($0)) }, title: "Move to Trash")
        treeSelection = []
    }
    private func performFiles(_ mutations: [FileMutation], title: String) throws {
        guard let operations else { throw ProjectFileError.outsideProject }
        let selection = treeSelection
        let receipt = try operations.perform(mutations)
        let removed = adopt(receipt.relocations, restoring: [])
        undoFiles.append(FileHistoryEntry(operations: receipt.inverse, title: title, buffers: removed, selection: selection)); redoFiles = []
        finishFileOperation(receipt.relocations)
    }
    func undoFileOperation() { replayFileHistory(undo: true) }
    func redoFileOperation() { replayFileHistory(undo: false) }
    private func replayFileHistory(undo: Bool) {
        guard let entry = undo ? undoFiles.last : redoFiles.last, let operations else { return }
        attempt {
            let selection = treeSelection
            let receipt = try operations.perform(entry.operations)
            let removed = adopt(receipt.relocations, restoring: entry.buffers)
            let reverse = FileHistoryEntry(operations: receipt.inverse, title: entry.title, buffers: removed, selection: selection)
            if undo { undoFiles.removeLast(); redoFiles.append(reverse) }
            else { redoFiles.removeLast(); undoFiles.append(reverse) }
            treeSelection = entry.selection
            finishFileOperation(receipt.relocations)
        }
    }
    private func adopt(_ relocations: [FileRelocation], restoring: [SourceFileBuffer]) -> [SourceFileBuffer] {
        var removed: [SourceFileBuffer] = []
        for relocation in relocations {
            guard let old = relativePath(relocation.from) else { continue }
            let new = relativePath(relocation.to)
            func remap(_ path: String) -> String? {
                guard path == old || path.hasPrefix(old + "/") else { return path }
                return new.map { $0 + path.dropFirst(old.count) }
            }
            pendingAutoSaves = Set(pendingAutoSaves.compactMap(remap))
            let keptDrafts = Set(buffers.filter { $0.dirty && remap($0.path) == nil }.map(\.path))
            buffers = buffers.compactMap { buffer in
                guard let path = remap(buffer.path) else {
                    if buffer.dirty { return buffer }
                    removed.append(buffer); return nil
                }
                var copy = buffer; copy.path = path; return copy
            }
            controllers = Dictionary(controllers.compactMap { key, value in
                let path = keptDrafts.contains(key.path) ? key.path : remap(key.path)
                return path.map { (SourceEditorKey(pane: key.pane, path: $0), value) }
            }, uniquingKeysWith: { _, new in new })
            expanded = Set(expanded.compactMap(remap)); conflicts = Set(conflicts.compactMap(remap))
            treeSelection = Set(treeSelection.compactMap(remap))
            paneSelections = paneSelections.compactMapValues { keptDrafts.contains($0) ? $0 : remap($0) }
            paneTabs = paneTabs.mapValues { $0.compactMap { keptDrafts.contains($0) ? $0 : remap($0) } }
        }
        for buffer in restoring where !buffers.contains(where: { $0.path == buffer.path }) { buffers.append(buffer) }
        for buffer in restoring where !(paneTabs[activeSourcePane] ?? []).contains(buffer.path) {
            paneTabs[activeSourcePane, default: []].append(buffer.path)
        }
        for pane in sourceLayout.panes where paneSelections[pane] == nil { paneSelections[pane] = paneTabs[pane]?.first }
        return removed
    }
    private func finishFileOperation(_ relocations: [FileRelocation]) {
        wantsEditorFocus = false
        persistDrafts(); refresh(); onRelocate?(relocations); onSave?()
    }
    func copyTreeSelection(cut: Bool, pasteboard: NSPasteboard = .general) {
        attempt {
            let paths = Self.topLevel(selectedTreePaths), urls = try paths.map(entryURL)
            guard !urls.isEmpty else { return }
            pasteboard.clearContents(); pasteboard.writeObjects(urls as [NSURL])
            cutPaths = cut ? paths : []; cutClipboardChange = cut ? pasteboard.changeCount : nil
        }
    }
    func pasteTreeSelection(pasteboard: NSPasteboard = .general) throws {
        let moving = cutClipboardChange == pasteboard.changeCount && !cutPaths.isEmpty
        let urls = moving ? try cutPaths.map(entryURL) : Self.clipboardURLs(pasteboard)
        guard !urls.isEmpty else { return }
        _ = try transfer(urls, to: destinationFolder, move: moving)
        if moving { cutPaths = []; cutClipboardChange = nil; pasteboard.clearContents() }
    }
    static func clipboardURLs(_ pasteboard: NSPasteboard) -> [URL] {
        (pasteboard.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL]) ?? []
    }
    func copyTreePaths(relative: Bool) {
        let paths = selectedTreePaths.compactMap { path in relative ? path : (try? entryURL(path).path) }
        guard !paths.isEmpty else { return }
        NSPasteboard.general.clearContents(); NSPasteboard.general.setString(paths.joined(separator: "\n"), forType: .string)
        cutPaths = []; cutClipboardChange = nil
    }
    func revealTreeSelection() {
        let urls = selectedTreePaths.compactMap { try? entryURL($0) }
        if !urls.isEmpty { NSWorkspace.shared.activateFileViewerSelecting(urls) }
    }

    func tabs(in pane: Int) -> [SourceFileBuffer] {
        (paneTabs[pane] ?? []).compactMap { path in buffers.first { $0.path == path } }
    }
    func cycleSourceTab(_ offset: Int) {
        let tabs = paneTabs[activeSourcePane] ?? []
        guard !tabs.isEmpty else { return }
        let index = tabs.firstIndex { $0 == currentPath } ?? 0
        select(tabs[(index + offset + tabs.count) % tabs.count]); focusCurrentEditor()
    }
    @discardableResult func splitSourcePane(_ axis: SourceSplitAxis, pane: Int? = nil, path: String? = nil, before: Bool = false) -> Int {
        let target = pane ?? activeSourcePane
        guard sourceLayout.panes.contains(target) else { return activeSourcePane }
        let newPane = nextPaneID; nextPaneID += 1
        let path = path ?? self.path(in: target)
        paneTabs[newPane] = path.map { [$0] } ?? []; paneSelections[newPane] = path
        sourceLayout = sourceLayout.splitting(target, axis: axis, adding: newPane, before: before)
        activeSourcePane = newPane; wantsEditorFocus = true; focusCurrentEditor()
        return newPane
    }
    func closeSourcePane(_ pane: Int) {
        let ids = sourceLayout.panes
        guard ids.count > 1, let index = ids.firstIndex(of: pane), let layout = sourceLayout.removing(pane) else { return }
        let neighbor = layout.panes[min(index, layout.panes.count - 1)]
        for path in paneTabs[pane] ?? [] where !(paneTabs[neighbor] ?? []).contains(path) {
            paneTabs[neighbor, default: []].append(path)
        }
        if paneSelections[neighbor] == nil { paneSelections[neighbor] = paneTabs[neighbor]?.first }
        sourceLayout = layout; paneSelections[pane] = nil; paneTabs[pane] = nil
        controllers = controllers.filter { $0.key.pane != pane }
        if activeSourcePane == pane { activeSourcePane = neighbor }
        wantsEditorFocus = true; focusCurrentEditor()
    }
    func requestCloseTab(_ path: String, pane: Int) {
        guard (paneTabs[pane] ?? []).contains(path) else { return }
        if pendingAutoSaves.contains(path) { flushAutoSaves() }
        let elsewhere = paneTabs.contains { $0.key != pane && $0.value.contains(path) }
        if !elsewhere, buffers.first(where: { $0.path == path })?.dirty == true {
            pendingTabClose = SourceTabClose(path: path, pane: pane)
        } else { closeTab(path, pane: pane) }
    }
    func confirmCloseTab(_ request: SourceTabClose? = nil, saveChanges: Bool) {
        guard let request = request ?? pendingTabClose else { return }
        if saveChanges {
            do { try save(request.path) } catch { self.error = error.localizedDescription; pendingTabClose = nil; return }
        }
        pendingTabClose = nil; closeTab(request.path, pane: request.pane)
    }
    private func closeTab(_ path: String, pane: Int) {
        guard let index = paneTabs[pane]?.firstIndex(of: path) else { return }
        paneTabs[pane]?.remove(at: index)
        let remaining = paneTabs[pane] ?? []
        if paneSelections[pane] == path { paneSelections[pane] = remaining.isEmpty ? nil : remaining[min(index, remaining.count - 1)] }
        controllers[SourceEditorKey(pane: pane, path: path)] = nil
        if !paneTabs.values.contains(where: { $0.contains(path) }) {
            pendingAutoSaves.remove(path)
            buffers.removeAll { $0.path == path }; conflicts.remove(path); persistDrafts(); updateCompletions()
        }
        activeSourcePane = pane
        if remaining.isEmpty && splitSource { closeSourcePane(pane) }
        wantsEditorFocus = true; focusCurrentEditor()
    }
    func dropSourceTab(_ payload: SourceTabDrag, onto pane: Int, edge: SourceDropZone) -> Bool {
        guard payload.project == root?.path, sourceLayout.panes.contains(pane),
              (paneTabs[payload.pane] ?? []).contains(payload.path) else { return false }
        if edge == .center {
            select(payload.path, pane: pane)
            if payload.pane != pane { closeTab(payload.path, pane: payload.pane); activeSourcePane = pane }
        } else {
            // Splitting by dragging keeps the source tab, like Split Right/Down.
            splitSourcePane(edge == .left || edge == .right ? .right : .down, pane: pane,
                            path: payload.path, before: edge == .left || edge == .top)
        }
        wantsEditorFocus = true; focusCurrentEditor(); return true
    }
    func focusCurrentEditor() {
        DispatchQueue.main.async { [weak self] in
            guard let self, self.wantsEditorFocus, let path = self.currentPath, let view = self.controller(path).view else { return }
            view.window?.makeFirstResponder(view)
        }
    }
}
