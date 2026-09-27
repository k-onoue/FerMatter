import AppKit
import CryptoKit
import SwiftUI
import UniformTypeIdentifiers

@MainActor
final class LibraryStore: ObservableObject {
    @Published var cards: [Equation] = []
    @Published var selection: String? { didSet {
        if !updatingSelection { selectedIDs = Set(selection.map { [$0] } ?? []); selectionAnchor = selection }
        if showPDF && !selectingFromPDF { navigatePDFToSelection() }; prepareExportResources()
    } }
    @Published var selectedIDs = Set<String>()
    private var selectionAnchor: String?
    private var updatingSelection = false
    weak var canvasResponder: NSView?
    var canvasColumns = 1
    @Published var root = ""
    @Published var main = ""
    @Published var engine = "project"
    @Published var format: ProjectFormat = .latex
    @Published var candidates: [String] = []
    @Published var projectCompletions: [SourceCompletion] = []
    private var completionOperation = UUID()
    @Published var hasRC = true
    @Published var query = ""
    @Published var kind = "all"
    @Published var origin = "all"
    @Published var sort = "source"
    @Published var compact = false
    @Published var showingProjects = true
    @Published var showingNewProject = false
    @Published var projects: [ProjectSummary] = []
    @Published var sections: [EquationSection] = []
    @Published var sectionAssignments: [String: String] = [:]
    @Published var sectionFilter = "all"
    @Published var outline: [DocumentHeading] = []
    @Published var collapsedOutline: Set<String> = []
    @Published var exportFormat: ImageExportFormat = .svg
    @Published var exportFontSize: Double = 0
    @Published var sourceFontSize: Double?
    @Published var fontMetricsBusy = false
    @Published var busy = false
    @Published var status = ""
    @Published var warnings: [String] = []
    @Published var alert: String?
    @Published var editor: Equation?
    @Published var preview: Equation?
    @Published var previewBusy = false
    @Published var showInspector = true
    @Published var showPDF = false
    @Published var editingProject = false
    let files: ProjectFiles
    let paneWidths = PaneWidths()
    @Published var scrollToEquation: String?
    let pdf = PDFSession()
    private var selectingFromPDF = false
    private var customOrder: [String] = []
    private let rendering = WorkerSession()
    private let previewRendering = WorkerSession()
    private let fontMeasurement = WorkerSession()
    private var saveTask: DispatchWorkItem?
    private var renderFinished = 0
    private var renderTotal = 0
    private var scanThenRender = false

    let support: URL
    private let storage: ProjectStorage
    var projectDirectory: URL { storage.directory(for: root) }
    var selected: Equation? { cards.first { $0.id == selection } }
    var selectedCards: [Equation] { filtered.filter { selectedIDs.contains($0.id) } }

    func selectCard(_ id: String, modifiers: NSEvent.ModifierFlags = []) {
        let ordered = filtered.map(\.id)
        guard let target = ordered.firstIndex(of: id) else { return }
        var ids = selectedIDs
        if modifiers.contains(.shift), let anchor = selectionAnchor, let start = ordered.firstIndex(of: anchor) {
            let range = Set(ordered[min(start, target)...max(start, target)])
            ids = modifiers.contains(.command) ? ids.union(range) : range
        } else if modifiers.contains(.command) {
            if !ids.insert(id).inserted { ids.remove(id) }
            selectionAnchor = id
        } else { ids = [id]; selectionAnchor = id }
        setCardSelection(ids, primary: ids.contains(id) ? id : ordered.first { ids.contains($0) })
        focusCanvas()
    }

    func setCardSelection(_ ids: Set<String>, primary: String? = nil) {
        let visible = filtered.map(\.id), valid = ids.intersection(visible)
        updatingSelection = true
        selectedIDs = valid
        selection = primary.flatMap { valid.contains($0) ? $0 : nil } ?? visible.first { valid.contains($0) }
        updatingSelection = false
    }
    func selectAllCards() { setCardSelection(Set(filtered.map(\.id))); selectionAnchor = filtered.first?.id; focusCanvas() }
    func clearCardSelection() { setCardSelection([]); selectionAnchor = nil; focusCanvas() }
    func focusCanvas() { if let view = canvasResponder { view.window?.makeFirstResponder(view) } }
    func retainVisibleSelection() { setCardSelection(selectedIDs, primary: selection) }
    func assignSelected(to label: String?) {
        for card in selectedCards { sectionAssignments[card.id] = label }
        saveNow()
    }

    func copySelection() {
        if selectedCards.count == 1, let card = selectedCards.first { copyImage(card); return }
        do {
            let urls = try selectedCards.map { try imageForExport($0) as NSURL }
            guard !urls.isEmpty else { return }
            NSPasteboard.general.clearContents(); NSPasteboard.general.writeObjects(urls)
        } catch { alert = error.localizedDescription }
    }
    func exportSelection() {
        let items = selectedCards
        if items.count == 1, let card = items.first { export(card); return }
        guard !items.isEmpty else { return }
        let panel = NSOpenPanel(); panel.canChooseFiles = false; panel.canChooseDirectories = true
        panel.canCreateDirectories = true; panel.prompt = "Export \(items.count) Images"
        // Attach the chooser to the library so a floating window cannot take
        // keyboard focus away from its Go to Folder sheet.
        if let window = NSApp.keyWindow ?? NSApp.mainWindow {
            panel.beginSheetModal(for: window) { [weak self] response in
                guard response == .OK, let folder = panel.url else { return }
                self?.exportImages(items, to: folder)
            }
        } else if panel.runModal() == .OK, let folder = panel.url { exportImages(items, to: folder) }
    }

    private func exportImages(_ items: [Equation], to folder: URL) {
        do {
            for card in items {
                let name = String(card.name.map { "/:\\".contains($0) || $0.isNewline ? "-" : $0 }.prefix(100))
                let stem = name.isEmpty ? "Equation" : name
                var url = folder.appendingPathComponent(stem + "." + exportFormat.fileExtension), count = 2
                while FileManager.default.fileExists(atPath: url.path) { url = folder.appendingPathComponent(stem + " \(count)." + exportFormat.fileExtension); count += 1 }
                try Data(contentsOf: imageForExport(card)).write(to: url, options: .withoutOverwriting)
            }
        } catch { alert = error.localizedDescription }
    }
    var projectName: String { URL(fileURLWithPath: root).lastPathComponent }
    var sectionTitle: String {
        if sectionFilter == "unsectioned" { return "Unlabeled" }
        if sectionFilter == "outline:none" { return "No heading" }
        if sectionFilter.hasPrefix("outline:") { return outline.first { "outline:" + $0.id == sectionFilter }?.title ?? allItemsTitle }
        return sections.first { $0.id == sectionFilter }?.name ?? allItemsTitle
    }
    var visibleOutline: [DocumentHeading] {
        outline.filter { heading in heading.path.dropLast().allSatisfy { !collapsedOutline.contains($0) } }
    }
    func outlineCount(_ heading: DocumentHeading) -> Int { cards.filter { ($0.outlinePath ?? []).contains(heading.id) }.count }
    func outlineTitle(for card: Equation) -> String? {
        let titles = (card.outlinePath ?? []).compactMap { id in outline.first { $0.id == id }?.title }
        return titles.isEmpty ? nil : titles.joined(separator: " › ")
    }
    func toggleOutline(_ heading: DocumentHeading) {
        if collapsedOutline.contains(heading.id) { collapsedOutline.remove(heading.id) }
        else { collapsedOutline.insert(heading.id) }
        saveNow()
    }
    private func matchesNavigation(_ card: Equation) -> Bool {
        if sectionFilter == "all" { return true }
        if sectionFilter == "outline:none" { return (card.outlinePath ?? []).isEmpty }
        if sectionFilter.hasPrefix("outline:") { return (card.outlinePath ?? []).contains(String(sectionFilter.dropFirst(8))) }
        return sectionFilter == "unsectioned" ? sectionAssignments[card.id] == nil : sectionAssignments[card.id] == sectionFilter
    }
    var hasFigures: Bool { cards.contains { $0.kind == "figure" } }
    var allItemsTitle: String { hasFigures ? "All items" : "All equations" }
    func countLabel(_ items: [Equation]) -> String {
        items.contains { $0.kind == "figure" } ? "\(items.count) items" : equationCountLabel(items.count)
    }
    var readyCount: Int { cards.filter { $0.status == "ready" }.count }
    var failedCount: Int { cards.filter { $0.status == "failed" }.count }
    var canRender: Bool { format == .typst || hasRC || engine != "project" }
    var filtered: [Equation] {
        let ranks = Dictionary(uniqueKeysWithValues: customOrder.enumerated().map { ($1, $0) })
        return cards.filter {
            (kind == "all" || $0.kind == kind) && (origin == "all" || $0.origin == origin) &&
            matchesNavigation($0) &&
            (query.isEmpty || ($0.name + " " + $0.tex + " " + $0.location).localizedCaseInsensitiveContains(query))
        }.sorted {
            sort == "custom" ? (ranks[$0.id] ?? Int.max) < (ranks[$1.id] ?? Int.max) : $0.sourceOrder < $1.sourceOrder
        }
    }

    init(support: URL? = nil) {
        self.support = support ?? AppIdentity.applicationSupport
        storage = ProjectStorage(support: self.support)
        files = ProjectFiles(support: self.support)
        files.onSave = { [weak self] in self?.sourceFilesSaved() }
        files.onRelocate = { [weak self] changes in
            guard let self else { return }
            for change in changes {
                let mainURL = URL(fileURLWithPath: self.root).appendingPathComponent(self.main)
                if mainURL.path == change.from.path || mainURL.path.hasPrefix(change.from.path + "/") {
                    let moved = URL(fileURLWithPath: change.to.path + mainURL.path.dropFirst(change.from.path.count))
                    if let path = self.files.relativePath(moved) { self.main = path }
                }
            }
        }
        refreshProjects()
        pdf.onReady = { [weak self] in self?.navigatePDFToSelection() }
        pdf.onSource = { [weak self] file, line, column in self?.selectPDFSource(file: file, line: line, column: column) }
        pdf.onTarget = { [weak self] id in self?.selectPDFTarget(id) }
    }

    func refreshProjects() { projects = storage.summaries() }

    func showProjectInFinder(_ projectRoot: String, openFolder: (URL) -> Bool = { NSWorkspace.shared.open($0) }) {
        var isDirectory: ObjCBool = false
        guard projectRoot.hasPrefix("/"),
              FileManager.default.fileExists(atPath: projectRoot, isDirectory: &isDirectory), isDirectory.boolValue else {
            alert = "Project folder not found: \(projectRoot)"
            return
        }
        let folder = URL(fileURLWithPath: projectRoot, isDirectory: true)
        if !openFolder(folder) { alert = "Could not open the project folder in Finder: \(projectRoot)" }
    }

    func showProjects() {
        files.flushAutoSaves(); files.persistDrafts()
        saveNow(); refreshProjects(); showingProjects = true
    }

    func toggleProjectEditor() {
        editingProject.toggle()
        if editingProject { files.configure(root: URL(fileURLWithPath: root), main: main, format: format) }
        else { files.flushAutoSaves(); files.persistDrafts(); focusCanvas() }
    }

    func sourceFilesSaved() {
        guard files.root?.path == root else { return }
        hasRC = ["latexmkrc", ".latexmkrc"].contains { FileManager.default.fileExists(atPath: URL(fileURLWithPath: root).appendingPathComponent($0).path) }
        scan()
    }

    func addSection(_ name: String) {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { return }
        let section = EquationSection(name: name)
        sections.append(section); sectionFilter = section.id; selection = nil
        saveNow()
    }

    func renameSection(_ section: EquationSection, to name: String) {
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty, let index = sections.firstIndex(where: { $0.id == section.id }) else { return }
        sections[index].name = name; saveNow()
    }

    func deleteSection(_ section: EquationSection) {
        sections.removeAll { $0.id == section.id }
        sectionAssignments = sectionAssignments.filter { $0.value != section.id }
        if sectionFilter == section.id { sectionFilter = "all" }
        saveNow()
    }

    func assign(_ card: Equation, to sectionID: String?) {
        guard sectionID == nil || sections.contains(where: { $0.id == sectionID }) else { return }
        sectionAssignments[card.id] = sectionID
        saveNow()
    }

    func sectionName(for card: Equation) -> String? {
        sections.first { $0.id == sectionAssignments[card.id] }?.name
    }

    func selectSection(_ id: String) {
        selectingFromPDF = true
        defer { selectingFromPDF = false }
        sectionFilter = id
        if !filtered.contains(where: { $0.id == selection }) {
            selection = filtered.first?.id
        }
        if showPDF, let heading = outline.first(where: { "outline:" + $0.id == id }) {
            pdf.forward(id: heading.id, file: heading.file, line: heading.line)
        }
    }

    func togglePDF() {
        showPDF.toggle()
        if showPDF && pdf.document == nil { buildPDF() }
    }

    func toggleInspector() {
        if showPDF { showPDF = false; showInspector = true }
        else { showInspector.toggle() }
    }

    func buildPDF(force: Bool = false) {
        pdf.build(root: root, main: main, engine: engine,
                  cache: projectDirectory.appendingPathComponent("Documents"), force: force)
    }

    func navigatePDFToSelection() {
        guard let selected else { return }
        let source = selected.parentID.flatMap { id in cards.first { $0.id == id } } ?? selected
        guard source.origin == "source", let file = source.file, let line = source.line else { return }
        // Block openings often contain only an environment or label; query its body.
        let bodyLine = source.kind != "inline" ? (line + (source.endLine ?? line)) / 2 : line
        pdf.forward(id: source.id, file: file, line: bodyLine, column: bodyLine == line ? (source.column ?? 0) : 0)
    }

    func selectPDFTarget(_ id: String) {
        if editingProject {
            if let card = cards.first(where: { $0.id == id }), let file = card.file { files.open(file, line: card.line) }
            else if let heading = outline.first(where: { $0.id == id }) { files.open(heading.file, line: heading.line) }
            return
        }
        selectingFromPDF = true
        defer { selectingFromPDF = false }
        if let card = cards.first(where: { $0.id == id }) {
            query = ""; kind = "all"; origin = "all"
            if !matchesNavigation(card) { sectionFilter = "all" }
            selection = card.id
            // Changing filters recreates the grid; deliver scrolling on its next update.
            scrollToEquation = nil
            DispatchQueue.main.async { [weak self] in self?.scrollToEquation = card.id }
        } else if let heading = outline.first(where: { $0.id == id }) {
            collapsedOutline.subtract(heading.path)
            query = ""; kind = "all"; origin = "all"; sectionFilter = "outline:" + id
            selection = filtered.first?.id
        }
    }

    func selectPDFSource(file: String, line: Int, column: Int) {
        if editingProject { files.open(file, line: line); return }
        if let card = nearestSourceEquation(file: file, line: line, column: column) { selectPDFTarget(card.id) }
        else if let heading = outline.filter({ $0.file == file && $0.line <= line }).max(by: { $0.line < $1.line }) { selectPDFTarget(heading.id) }
        else { pdf.notice = "No equation near this source position." }
    }

    func nearestSourceEquation(file: String, line: Int, column: Int) -> Equation? {
        func distance(_ card: Equation) -> Int {
            let start = card.line ?? 1, end = card.endLine ?? start
            let rows = max(start - line, line - end, 0)
            let columns = column > 0 && start == end ? max((card.column ?? 1) - column, column - (card.endColumn ?? Int.max), 0) : 0
            return rows * 10000 + columns
        }
        return cards.filter { $0.origin == "source" && $0.file == file && distance($0) < 80000 }
            .min { distance($0) == distance($1) ? $0.sourceOrder < $1.sourceOrder : distance($0) < distance($1) }
    }

    func openProject() {
        let panel = NSOpenPanel()
        panel.title = "Open Project"
        panel.prompt = "Open"
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = false
        panel.allowedContentTypes = [.zip, .folder, UTType(filenameExtension: "tex") ?? .plainText, UTType(filenameExtension: "typ") ?? .plainText]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        if url.pathExtension.lowercased() == "zip" {
            start(["action": "import", "path": url.path, "destination": support.appendingPathComponent("Imports").path]) { [weak self] event in
                if let path = event.root { self?.load(URL(fileURLWithPath: path)) }
            }
        } else { load(url, rescan: false) }
    }

    func load(_ url: URL, rescan: Bool = true) {
        guard FileManager.default.fileExists(atPath: url.path) else { alert = "Project folder not found: \(url.path)"; return }
        let resolved = url.resolvingSymlinksInPath()
        let isDirectory = (try? resolved.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory ?? false
        let isMainFile = !isDirectory && ["tex", "typ"].contains(resolved.pathExtension.lowercased())
        guard isDirectory || isMainFile else { alert = "Select a project folder, .tex file, or .typ file."; return }
        let folder = isMainFile ? resolved.deletingLastPathComponent() : resolved
        let requestedMain = isMainFile ? resolved.lastPathComponent : nil
        saveNow()
        let saved = try? storage.load(root: folder.path)
        if let requestedMain, let saved, saved.resolvedFormat != (requestedMain.hasSuffix(".typ") ? ProjectFormat.typst : .latex) {
            alert = "This folder already has a \(saved.resolvedFormat.displayName) library. Keep LaTeX and Typst projects in separate folders."
            return
        }
        files.flushAutoSaves(); files.persistDrafts(); editingProject = false
        cancelPreview(); editor = nil; pdf.reset()
        stop()
        saveNow()
        root = folder.path
        projectCompletions = []
        main = ""; cards = []; candidates = []; customOrder = []; query = ""; warnings = []
        sections = []; sectionAssignments = [:]; sectionFilter = "all"; kind = "all"; origin = "all"
        outline = []; collapsedOutline = []
        sourceFontSize = nil; exportFontSize = 0; exportFormat = .svg; format = .latex
        showingProjects = false
        var refreshTypstBounds = false
        if let saved {
            main = saved.main; engine = saved.engine; cards = saved.cards; customOrder = saved.customOrder
            format = saved.resolvedFormat
            sections = saved.sections ?? []; sectionAssignments = saved.sectionAssignments ?? [:]
            outline = saved.outline ?? []; collapsedOutline = Set(saved.collapsedOutline ?? [])
            sourceFontSize = saved.sourceFontSize; exportFontSize = saved.exportFontSize ?? 0
            exportFormat = saved.exportFormat ?? .svg
            if format == .typst {
                refreshTypstBounds = cards.contains { card in
                    guard card.kind == "inline", card.status == "ready", let path = card.svgPath,
                          let file = FileHandle(forReadingAtPath: path) else { return false }
                    defer { try? file.close() }
                    let header = (try? file.read(upToCount: 512)) ?? Data()
                    return !String(decoding: header, as: UTF8.self).contains("data-slide-math-bounds=\"glyph\"")
                }
            }
            for index in cards.indices where cards[index].status == "ready" {
                if !FileManager.default.fileExists(atPath: cards[index].svgPath ?? "") || !FileManager.default.fileExists(atPath: cards[index].pngPath ?? "") { cards[index].status = "pending" }
            }
        } else { engine = "project" }
        let changedMain = requestedMain != nil && requestedMain != main
        if let requestedMain { main = requestedMain; format = main.hasSuffix(".typ") ? .typst : .latex }
        hasRC = ["latexmkrc", ".latexmkrc"].contains { FileManager.default.fileExists(atPath: folder.appendingPathComponent($0).path) }
        candidates = main.isEmpty ? [] : [main]
        refreshCompletions()
        selection = cards.first?.id
        UserDefaults.standard.set(root, forKey: "lastProject")
        status = ""
        if rescan || cards.isEmpty || changedMain { scan() }
        else if saved?.outline == nil && FileManager.default.fileExists(atPath: folder.appendingPathComponent(main).path) { scanProject(render: refreshTypstBounds) }
        else if refreshTypstBounds && canRender { renderAll(); if showPDF { buildPDF() } }
        else { prepareFontMetrics(); prepareExportResources(); if showPDF { buildPDF() } }
    }

    func scan() {
        scanProject(render: true)
    }

    private func scanProject(render: Bool) {
        guard !root.isEmpty else { return }
        pdf.invalidate()
        status = "Scanning…"
        var request: [String: Any] = ["action": "scan", "root": root]
        if !main.isEmpty { request["main"] = main; request["format"] = format.rawValue }
        scanThenRender = render
        start(request) { [weak self] event in
            guard let self, event.event == "scanned", let extracted = event.cards else { return }
            let old = Dictionary(uniqueKeysWithValues: self.cards.map { ($0.id, $0) })
            let sources = Dictionary(uniqueKeysWithValues: extracted.map { ($0.id, $0) })
            self.outline = event.outline ?? []
            let headingIDs = Set(self.outline.map(\.id))
            self.collapsedOutline.formIntersection(headingIDs)
            self.cards = extracted.map { card in
                var card = card
                if let previous = old[card.id] {
                    card.svgPath = previous.svgPath; card.pngPath = previous.pngPath; card.pdfPath = previous.pdfPath
                    card.exportFontSize = previous.exportFontSize
                    if !render { card.status = previous.status; card.sourceFontSize = previous.sourceFontSize; card.error = previous.error; card.logPath = previous.logPath }
                }
                return card
            } + self.cards.filter { $0.origin == "custom" }.map { card in
                var card = card
                if let parent = card.parentID, let source = sources[parent] {
                    card.typstContext = source.typstContext; card.outlinePath = source.outlinePath
                } else { card.outlinePath = (card.outlinePath ?? []).filter { headingIDs.contains($0) } }
                return card
            }
            let ids = Set(self.cards.map(\.id))
            self.sectionAssignments = self.sectionAssignments.filter { ids.contains($0.key) }
            self.customOrder = self.customOrder.filter { ids.contains($0) }
            self.customOrder += self.cards.map(\.id).filter { !self.customOrder.contains($0) }
            self.main = event.main ?? self.main
            self.format = event.format ?? self.format
            self.refreshCompletions()
            self.candidates = event.candidates ?? []
            self.hasRC = event.hasRC ?? false
            self.warnings = event.warnings ?? []
            if self.sectionFilter.hasPrefix("outline:"), self.sectionFilter != "outline:none", !headingIDs.contains(String(self.sectionFilter.dropFirst(8))) { self.sectionFilter = "all" }
            if self.selection == nil || !ids.contains(self.selection!) { self.selection = self.cards.first?.id }
            self.status = ""
            self.saveNow()
        } finished: { [weak self] in
            guard let self else { return }
            if self.showPDF { self.buildPDF() }
            guard self.scanThenRender else { self.prepareFontMetrics(); return }
            self.scanThenRender = false
            if self.canRender { self.renderAll() }
            else { self.status = "Select a compiler in Project Settings." }
        }
    }

    func renderAll() {
        guard !cards.isEmpty, canRender else { return }
        sourceFontSize = nil
        prepareFontMetrics()
        for index in cards.indices { cards[index].status = "pending"; cards[index].error = nil; cards[index].sourceFontSize = nil }
        renderFinished = 0; renderTotal = cards.count
        status = "Rendering 0 / \(renderTotal)"
        start(renderRequest(cards)) { [weak self] event in
            guard let self, event.event == "card", let index = self.cards.firstIndex(where: { $0.id == event.id }) else { return }
            self.apply(event, to: &self.cards[index])
            self.renderFinished += 1
            self.status = "Rendering \(self.renderFinished) / \(self.renderTotal)"
            self.scheduleSave()
        } finished: { [weak self] in
            guard let self else { return }
            self.status = ""
            self.saveNow()
        }
    }

    private func renderRequest(_ items: [Equation]) -> [String: Any] {
        let data = try! JSONEncoder().encode(items)
        return ["action": "render", "root": root, "main": main, "engine": engine, "format": format.rawValue,
                "cache": projectDirectory.appendingPathComponent("Cache").path,
                "cards": try! JSONSerialization.jsonObject(with: data)]
    }

    private func start(_ request: [String: Any], receive: @escaping (WorkerEvent) -> Void, finished: @escaping () -> Void = {}) {
        busy = true
        rendering.start(request, receive: { [weak self] event in
            guard let self else { return }
            if event.event == "error" { self.alert = event.message; self.scanThenRender = false; self.status = "" }
            receive(event)
        }, finished: { [weak self] in
            guard let self else { return }
            self.busy = false; finished()
        })
    }

    func stop() {
        rendering.cancel(); busy = false; scanThenRender = false
        fontMeasurement.cancel(); fontMetricsBusy = false
        pdf.cancel()
        status = "Paused"
    }

    private func apply(_ event: WorkerEvent, to card: inout Equation) {
        card.status = event.status ?? "failed"; card.svgPath = event.svgPath
        card.pngPath = event.pngPath; card.pdfPath = event.pdfPath; card.error = event.error; card.logPath = event.logPath
        card.sourceFontSize = event.fontSize
    }

    func newFigure() {
        refreshCompletions()
        var draft = Equation.draft(); draft.name = "Untitled figure"; draft.kind = "figure"
        draft.sourceOrder = (cards.map(\.sourceOrder).max() ?? 0) + 1
        draft.tex = format == .typst
            ? "#import \"@preview/cetz:0.5.2\"\n#cetz.canvas({\n  import cetz.draw: *\n  circle((0, 0), radius: 1, stroke: blue + 1pt)\n})"
            : "\\begin{tikzpicture}\n  \\draw[blue, thick] (0,0) circle (1cm);\n\\end{tikzpicture}"
        preview = nil; editor = draft
    }

    func beginEdit(_ source: Equation?, duplicate: Bool = false) {
        refreshCompletions()
        preview = nil
        var draft = source ?? .draft()
        if draft.origin == "source" || duplicate {
            draft.id = "custom-" + UUID().uuidString
            draft.parentID = source?.parentID ?? source?.id
            draft.origin = "custom"
            draft.name += duplicate ? " copy" : " edited"
            let position = source?.sourceOrder ?? Double(cards.count)
            let next = cards.map(\.sourceOrder).filter { $0 > position }.min() ?? (position + 2)
            draft.sourceOrder = (position + next) / 2
        } else if source == nil { draft.sourceOrder = (cards.map(\.sourceOrder).max() ?? 0) + 1 }
        editor = draft
    }

    func refreshCompletions() {
        let root = root, format = format, token = UUID()
        completionOperation = token
        guard !root.isEmpty else { projectCompletions = []; return }
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let entries = ProjectCompletions.load(root: URL(fileURLWithPath: root), format: format)
            DispatchQueue.main.async {
                guard let self, self.root == root, self.format == format, self.completionOperation == token else { return }
                self.projectCompletions = entries
            }
        }
    }

    func renderPreview(_ draft: Equation) {
        cancelPreview()
        guard !draft.tex.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, !root.isEmpty else { return }
        previewBusy = true; preview = nil
        previewRendering.start(renderRequest([draft]), receive: { [weak self] event in
            guard let self else { return }
            if event.event == "card" { var result = draft; self.apply(event, to: &result); self.preview = result }
            if event.event == "error" { var result = draft; result.status = "failed"; result.error = event.message; self.preview = result }
        }, finished: { [weak self] in
            guard let self else { return }
            self.previewBusy = false
        })
    }

    func cancelPreview() { previewRendering.cancel(); previewBusy = false }

    func saveDraft(_ draft: Equation) {
        var result = draft
        if let preview, preview.tex == draft.tex, preview.kind == draft.kind {
            result = preview; result.name = draft.name; result.exportFontSize = draft.exportFontSize
        }
        else { result.status = "pending"; result.svgPath = nil; result.pngPath = nil; result.pdfPath = nil }
        if let index = cards.firstIndex(where: { $0.id == result.id }) { cards[index] = result }
        else {
            cards.append(result); customOrder.append(result.id)
            if sections.contains(where: { $0.id == sectionFilter }) { sectionAssignments[result.id] = sectionFilter }
            else if let parent = result.parentID { sectionAssignments[result.id] = sectionAssignments[parent] }
        }
        selection = result.id; editor = nil; cancelPreview()
        query = ""
        if origin == "source" { origin = "all" }
        if kind != "all" && kind != result.kind { kind = "all" }
        if sectionFilter.hasPrefix("outline:"), !matchesNavigation(result) { sectionFilter = "all" }
        if !busy { status = "" }
        saveNow()
        if result.status == "pending" { renderAll() }
    }

    func delete(_ card: Equation) {
        guard card.origin == "custom" else { return }
        cards.removeAll { $0.id == card.id }; customOrder.removeAll { $0 == card.id }
        sectionAssignments[card.id] = nil
        selection = cards.first?.id
        if !busy { status = "" }
        saveNow()
    }

    func move(_ card: Equation, by offset: Int) {
        let visible = filtered.map(\.id)
        guard let position = visible.firstIndex(of: card.id), visible.indices.contains(position + offset),
              let first = customOrder.firstIndex(of: card.id), let second = customOrder.firstIndex(of: visible[position + offset]) else { return }
        customOrder.swapAt(first, second); objectWillChange.send(); saveNow()
    }

    func export(_ card: Equation) {
        guard card.status == "ready" else { return }
        let panel = NSSavePanel(); panel.allowedContentTypes = [exportFormat.type]
        panel.nameFieldStringValue = card.name.replacingOccurrences(of: "/", with: "-") + "." + exportFormat.fileExtension
        if panel.runModal() == .OK, let destination = panel.url {
            do { try Data(contentsOf: imageForExport(card)).write(to: destination, options: .atomic) }
            catch { alert = error.localizedDescription }
        }
    }

    func exportSize(for card: Equation) -> Double {
        card.exportFontSize ?? (card.kind == "figure" ? 0 : exportFontSize)
    }

    func setExportFontSize(_ size: Double?, for card: Equation) {
        guard size == nil || (size!.isFinite && size! >= 0 && size! <= 288),
              let index = cards.firstIndex(where: { $0.id == card.id }) else { return }
        cards[index].exportFontSize = size
        saveNow(); prepareFontMetrics()
    }

    func canExport(_ card: Equation) -> Bool {
        card.status == "ready" && (exportFormat != .pdf || card.pdfPath.map { FileManager.default.fileExists(atPath: $0) } == true) && (exportSize(for: card) == 0 || card.sourceFontSize != nil || sourceFontSize != nil)
    }

    func svgForExport(_ card: Equation) throws -> URL {
        guard let path = card.svgPath else { throw CocoaError(.fileNoSuchFile) }
        let original = URL(fileURLWithPath: path)
        let points = exportSize(for: card)
        guard points > 0 else { return original }
        guard let size = card.sourceFontSize ?? sourceFontSize else { throw SVGExportError.missingFontMetrics }
        let data = try SVGExporter.scaled(Data(contentsOf: original), to: points, sourceTeXPoints: size, sourcePointsPerInch: format.pointsPerInch)
        let directory = projectDirectory.appendingPathComponent("Exports")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let key = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined().prefix(20)
        let file = directory.appendingPathComponent("equation-\(key)-\(points)pt.svg")
        if !FileManager.default.fileExists(atPath: file.path) { try data.write(to: file, options: .atomic) }
        return file
    }

    func prepareExportResources() {
        guard exportFormat == .pdf, !busy, canRender, let card = selected, card.status == "ready",
              FileManager.default.fileExists(atPath: URL(fileURLWithPath: root).appendingPathComponent(main).path),
              card.pdfPath.map({ FileManager.default.fileExists(atPath: $0) }) != true else { return }
        status = "Preparing PDF…"
        start(renderRequest([card])) { [weak self] event in
            guard let self, event.event == "card", let index = self.cards.firstIndex(where: { $0.id == event.id }) else { return }
            if event.status == "ready" { self.apply(event, to: &self.cards[index]); self.saveNow() }
            else { self.alert = event.error ?? "Could not prepare PDF." }
        } finished: { [weak self] in
            guard let self else { return }
            self.status = ""
            if self.selection != card.id { self.prepareExportResources() }
        }
    }

    func imageForExport(_ card: Equation) throws -> URL {
        let pdf = card.pdfPath.flatMap { FileManager.default.fileExists(atPath: $0) ? URL(fileURLWithPath: $0) : nil }
        return try ImageExporter.file(svg: svgForExport(card), pdf: pdf,
                                      png: card.pngPath.map { URL(fileURLWithPath: $0) }, format: exportFormat,
                                      directory: projectDirectory.appendingPathComponent("Exports"))
    }

    func dragProvider(_ card: Equation) -> NSItemProvider {
        guard canExport(card) else { return NSItemProvider() }
        do { return ImageExporter.provider(url: try imageForExport(card), format: exportFormat, name: card.name) }
        catch { alert = error.localizedDescription; return NSItemProvider() }
    }

    func copyImage(_ card: Equation) {
        do {
            let url = try imageForExport(card)
            let pasteboard = NSPasteboard.general
            pasteboard.clearContents()
            pasteboard.writeObjects([url as NSURL])
            pasteboard.setData(try Data(contentsOf: url), forType: NSPasteboard.PasteboardType(exportFormat.type.identifier))
        } catch { alert = error.localizedDescription }
    }

    func prepareFontMetrics() {
        guard sourceFontSize == nil, !main.isEmpty, canRender else { return }
        var probe = Equation.draft(); probe.id = "font-probe"; probe.tex = "x"; probe.kind = "inline"
        var request = renderRequest([probe]); request["requireFontMetrics"] = true
        fontMetricsBusy = true
        fontMeasurement.start(request, receive: { [weak self] event in
            guard let self else { return }
            if let size = event.fontSize, size > 0 { self.sourceFontSize = size; self.saveNow() }
        }, finished: { [weak self] in
            guard let self else { return }
            self.fontMetricsBusy = false
        })
    }

    func saveNow() {
        saveTask?.cancel()
        guard !root.isEmpty else { return }
        do {
            let project = SavedProject(root: root, main: main, engine: engine, cards: cards, customOrder: customOrder, sections: sections, sectionAssignments: sectionAssignments, sourceFontSize: sourceFontSize, exportFontSize: exportFontSize, format: format, outline: outline, collapsedOutline: Array(collapsedOutline).sorted(), exportFormat: exportFormat)
            try storage.save(project)
        } catch { alert = "Could not save: \(error.localizedDescription)" }
    }

    private func scheduleSave() {
        saveTask?.cancel()
        let task = DispatchWorkItem { [weak self] in self?.saveNow() }
        saveTask = task; DispatchQueue.main.asyncAfter(deadline: .now() + 0.5, execute: task)
    }
}
