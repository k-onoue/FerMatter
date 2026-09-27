import AppKit
import SwiftUI

enum AppCommand: String, CaseIterable, Identifiable, Codable {
    case open, projects, newProject, newEquation, newFigure, find, edit, duplicate, saveEquation, copySVG, exportSVG, rescan
    case togglePDF, rebuildPDF, previousPage, nextPage, zoomIn, zoomOut, fitPDF, inspector, pin, fullScreen, projectSource
    case nextSourceTab, previousSourceTab, splitSource, splitSourceDown
    var id: String { rawValue }
    var title: String {
        switch self {
        case .open: "Open Project…"
        case .projects: "Projects"
        case .newProject: "New Project…"
        case .newEquation: "New Equation"
        case .newFigure: "New Figure"
        case .find: "Find…"
        case .edit: "Edit Selection"
        case .duplicate: "Duplicate Selection"
        case .saveEquation: "Save"
        case .copySVG: "Copy Image"
        case .exportSVG: "Export Image…"
        case .rescan: "Rescan"
        case .togglePDF: "Toggle PDF"
        case .rebuildPDF: "Rebuild PDF"
        case .previousPage: "Previous PDF Page"
        case .nextPage: "Next PDF Page"
        case .zoomIn: "Zoom PDF In"
        case .zoomOut: "Zoom PDF Out"
        case .fitPDF: "Fit PDF"
        case .inspector: "Toggle Inspector"
        case .pin: "Always on Top"
        case .fullScreen: "Toggle Full Screen"
        case .projectSource: "Toggle Project Source"
        case .nextSourceTab: "Next Source Tab"
        case .previousSourceTab: "Previous Source Tab"
        case .splitSource: "Split Editor Right"
        case .splitSourceDown: "Split Editor Down"
        }
    }
    var defaultShortcut: ShortcutBinding {
        switch self {
        case .open: .init("o")
        case .projects: .init("o", shift: true)
        case .newProject: .init("n", shift: true)
        case .newEquation: .init("n")
        case .newFigure: .init("n", option: true)
        case .find: .init("f")
        case .edit: .init("e")
        case .duplicate: .init("d")
        case .saveEquation: .init("s")
        case .copySVG: .init("c", shift: true)
        case .exportSVG: .init("s", shift: true)
        case .rescan: .init("r")
        case .togglePDF: .init("p", option: true)
        case .rebuildPDF: .init("r", shift: true)
        case .previousPage: .init("[")
        case .nextPage: .init("]")
        case .zoomIn: .init("=", shift: true)
        case .zoomOut: .init("-")
        case .fitPDF: .init("0")
        case .inspector: .init("i", option: true)
        case .pin: .init("p", shift: true)
        case .fullScreen: .init("f", control: true)
        case .projectSource: .init("e", option: true)
        case .nextSourceTab: .init("\t", command: false, control: true)
        case .previousSourceTab: .init("\t", command: false, shift: true, control: true)
        case .splitSource: .init("\\")
        case .splitSourceDown: .init("\\", option: true)
        }
    }
}

struct ShortcutBinding: Codable, Equatable, Hashable {
    var key: String
    var command: Bool
    var shift: Bool
    var option: Bool
    var control: Bool
    init(_ key: String, command: Bool = true, shift: Bool = false, option: Bool = false, control: Bool = false) {
        self.key = key.lowercased(); self.command = command; self.shift = shift; self.option = option; self.control = control
    }
    static let unassigned = ShortcutBinding("", command: false)
    var modifiers: EventModifiers {
        var result: EventModifiers = []
        if command { result.insert(.command) }; if shift { result.insert(.shift) }
        if option { result.insert(.option) }; if control { result.insert(.control) }
        return result
    }
    var shortcut: KeyboardShortcut? {
        guard let character = key.first else { return nil }
        // AppKit matches shifted punctuation by its produced character. A menu
        // equivalent of '=' + Shift does not receive Command-+ on macOS.
        let plain = Array("`1234567890-=[]\\;',./"), shifted = Array("~!@#$%^&*()_+{}|:\"<>?")
        if shift, let index = plain.firstIndex(of: character) {
            return KeyboardShortcut(KeyEquivalent(shifted[index]), modifiers: modifiers.subtracting(.shift))
        }
        return KeyboardShortcut(KeyEquivalent(character), modifiers: modifiers)
    }
    var label: String {
        guard !key.isEmpty else { return "—" }
        let name = ["\t": "⇥", "\u{F700}": "↑", "\u{F701}": "↓", "\u{F702}": "←", "\u{F703}": "→" ][key] ?? (key == "=" && shift ? "+" : key.uppercased())
        return (control ? "⌃" : "") + (option ? "⌥" : "") + (shift && key != "=" ? "⇧" : "") + (command ? "⌘" : "") + name
    }
    static func from(_ event: NSEvent) -> ShortcutBinding? {
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        guard let text = event.keyCode == 48 ? "\t" : event.charactersIgnoringModifiers, text.count == 1 else { return nil }
        // Store the unshifted symbol so the menu equivalent and recorder agree.
        let shifted = "~!@#$%^&*()_+{}|:\"<>?", base = "`1234567890-=[]\\;',./"
        var key = text.lowercased()
        if flags.contains(.shift), let index = Array(shifted).firstIndex(of: Character(key)) { key = String(Array(base)[index]) }
        return ShortcutBinding(key, command: flags.contains(.command), shift: flags.contains(.shift), option: flags.contains(.option), control: flags.contains(.control))
    }
}

struct ShortcutDocument: SettingsDocument, Equatable {
    var schemaVersion = 1
    var kind = "mathlidelux.keybindings"
    var bindings: [String: ShortcutBinding] = [:]
    var extensions: [String: SettingsValue]? = nil

    func binding(_ command: AppCommand) -> ShortcutBinding { bindings[command.rawValue] ?? command.defaultShortcut }
    func validation(_ binding: ShortcutBinding, for command: AppCommand) -> String? {
        guard !binding.key.isEmpty else { return nil }
        let allowed = "abcdefghijklmnopqrstuvwxyz0123456789`-=[]\\;',./\t\u{F700}\u{F701}\u{F702}\u{F703}"
        guard binding.key.count == 1, allowed.contains(binding.key), binding.command || binding.control else { return "Use ⌘ or ⌃ with a letter, number, symbol, or arrow." }
        let native = ["q", "w", "m", "h", ",", "c", "v", "x", "a", "z", "s", "p", " ", "\t"]
        if binding.command && !binding.control && !binding.option && !binding.shift && native.contains(binding.key) && !(command == .saveEquation && binding.key == "s") { return "Reserved for macOS or text editing." }
        if binding == ShortcutBinding("z", shift: true) || binding == ShortcutBinding("h", option: true) { return "Reserved for macOS or text editing." }
        if let other = AppCommand.allCases.first(where: { $0 != command && self.binding($0) == binding }) { return "Already used by \(other.title)." }
        return nil
    }
    func validate() throws {
        guard schemaVersion == 1, kind == "mathlidelux.keybindings" else {
            throw SettingsFileError.invalid("Unsupported keyboard shortcuts format or version.")
        }
        for id in bindings.keys where AppCommand(rawValue: id) == nil {
            throw SettingsFileError.invalid("Unknown command: \(id).")
        }
        for command in AppCommand.allCases {
            if let error = validation(binding(command), for: command) {
                throw SettingsFileError.invalid("\(command.title): \(error)")
            }
        }
    }
}

@MainActor
final class ShortcutPreferences: FileSettings {
    @Published private var document: ShortcutDocument
    @Published private(set) var fileError: String?
    private let file: SettingsFile<ShortcutDocument>
    var fileURL: URL { file.url }
    init(defaults: UserDefaults = .standard, directory: URL = AppIdentity.settingsDirectory) {
        file = SettingsFile(directory: directory, name: "keybindings.json")
        let legacy = defaults.data(forKey: "keyboardShortcuts").flatMap { try? JSONDecoder().decode([String: ShortcutBinding].self, from: $0) } ?? [:]
        let migrated = ShortcutDocument(bindings: legacy)
        let fallback = (try? migrated.validate()) != nil ? migrated : ShortcutDocument()
        (document, fileError) = file.initial(fallback)
        // Keep the legacy value as a migration backup; the JSON file is authoritative.
    }
    func binding(_ command: AppCommand) -> ShortcutBinding { document.binding(command) }
    func validation(_ binding: ShortcutBinding, for command: AppCommand) -> String? { document.validation(binding, for: command) }
    @discardableResult func set(_ binding: ShortcutBinding, for command: AppCommand) -> String? {
        if let error = validation(binding, for: command) { return error }
        var updated = document; updated.bindings[command.rawValue] = binding
        do { try file.save(updated); document = updated; fileError = nil; return nil }
        catch { fileError = error.localizedDescription; return fileError }
    }
    func reset() { replace(ShortcutDocument()) }
    func reload() {
        do { document = try file.load(); fileError = nil }
        catch { fileError = error.localizedDescription }
    }
    func importFile(_ url: URL) {
        do { replace(try file.read(url)) }
        catch { fileError = error.localizedDescription }
    }
    func exportFile(_ url: URL) {
        do { try file.export(document, to: url); fileError = nil }
        catch { fileError = error.localizedDescription }
    }
    private func replace(_ value: ShortcutDocument) {
        do { try file.save(value, replacing: true); document = value; fileError = nil }
        catch { fileError = error.localizedDescription }
    }
}

extension Notification.Name {
    static let findEquations = Notification.Name("SlideMath.find")
    static let saveEquation = Notification.Name("SlideMath.saveEquation")
    static let newSourceFile = Notification.Name("SlideMath.newSourceFile")
}

struct ShortcutSettingsView: View {
    @ObservedObject var shortcuts: ShortcutPreferences
    @State private var recording: AppCommand?
    @State private var error: String?
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Keyboard Shortcuts").font(.title2.weight(.semibold))
                Spacer()
                Button("Restore Defaults") { recording = nil; shortcuts.reset(); error = nil }
            }.padding(24)
            SettingsFileControls(preferences: shortcuts, beforeAction: { recording = nil; error = nil })
                .padding(.horizontal, 24).padding(.bottom, 16)
            Divider()
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(AppCommand.allCases) { command in
                        HStack {
                            Text(command.title)
                            Spacer()
                            ShortcutRecorder(label: shortcuts.binding(command).label, active: recording == command,
                                             start: { recording = command; error = nil }, cancel: { recording = nil },
                                             recorded: { binding in
                                error = shortcuts.set(binding, for: command)
                                if error == nil { recording = nil }
                            }).frame(width: 126, height: 28)
                            Button { recording = nil; error = shortcuts.set(.unassigned, for: command) } label: { Image(systemName: "xmark.circle.fill").foregroundStyle(.tertiary) }
                                .buttonStyle(.plain).help("Clear shortcut").accessibilityLabel("Clear \(command.title) shortcut")
                        }.padding(.horizontal, 24).padding(.vertical, 7)
                    }
                }.padding(.vertical, 12)
            }
            Divider()
            HStack {
                Text(error ?? shortcuts.fileError ?? (recording == nil ? "Click a shortcut to change it." : "Press a shortcut. Esc to cancel."))
                    .font(.caption).foregroundStyle(error == nil && shortcuts.fileError == nil ? Color.secondary : Color.orange)
                Spacer()
            }.frame(minHeight: 20).padding(18)
        }.frame(width: 540, height: 640)
            .onDisappear { recording = nil }
    }
}

struct ShortcutRecorder: NSViewRepresentable {
    let label: String
    let active: Bool
    let start: () -> Void
    let cancel: () -> Void
    let recorded: (ShortcutBinding) -> Void
    func makeNSView(context: Context) -> RecordingButton { RecordingButton() }
    func updateNSView(_ button: RecordingButton, context: Context) {
        button.title = active ? "Type shortcut…" : label
        button.start = start; button.cancel = cancel; button.recorded = recorded
        button.recording = active
    }
    static func dismantleNSView(_ button: RecordingButton, coordinator: ()) { button.recording = false }
}

final class RecordingButton: NSButton {
    var start: (() -> Void)?
    var cancel: (() -> Void)?
    var recorded: ((ShortcutBinding) -> Void)?
    private var monitor: Any?
    var recording = false {
        didSet {
            guard recording != oldValue else { return }
            if recording {
                window?.makeFirstResponder(self)
                monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
                    guard let self, self.recording, event.window === self.window else { return event }
                    if event.keyCode == 53 { self.cancel?(); return nil }
                    if let binding = ShortcutBinding.from(event) { self.recorded?(binding) }
                    return nil
                }
            } else if let monitor { NSEvent.removeMonitor(monitor); self.monitor = nil }
        }
    }
    init() { super.init(frame: .zero); bezelStyle = .rounded; target = self; action = #selector(begin); font = .monospacedSystemFont(ofSize: 12, weight: .medium) }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    @objc private func begin() { start?() }
    override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); if window == nil { recording = false } }
}

struct LibraryCommands: Commands {
    @ObservedObject var store: LibraryStore
    @ObservedObject var preferences: WindowPreferences
    @ObservedObject var shortcuts: ShortcutPreferences
    @ObservedObject var pdf: PDFSession
    @ObservedObject var files: ProjectFiles
    private var noSheet: Bool { store.editor == nil && !store.showingNewProject }
    private var inLibrary: Bool { !store.showingProjects && !store.root.isEmpty && noSheet }
    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            item(.newProject, enabled: noSheet) { store.showingNewProject = true }
            item(.open, enabled: noSheet, action: store.openProject)
            item(.projects, enabled: noSheet, action: store.showProjects)
            Divider()
            item(.newEquation, enabled: inLibrary) {
                if store.editingProject { NotificationCenter.default.post(name: .newSourceFile, object: nil) }
                else { store.beginEdit(nil) }
            }
            item(.newFigure, enabled: inLibrary && !store.editingProject, action: store.newFigure)
        }
        CommandGroup(after: .textEditing) { item(.find, enabled: noSheet) {
            if inLibrary, store.editingProject, let path = files.currentPath { files.controller(path).find() }
            else { NotificationCenter.default.post(name: .findEquations, object: nil) }
        } }
        CommandGroup(replacing: .saveItem) { item(.saveEquation, enabled: store.editor != nil || (inLibrary && store.editingProject && files.current != nil)) {
            if store.editor != nil { NotificationCenter.default.post(name: .saveEquation, object: nil) }
            else { files.saveCurrent() }
        } }
        CommandMenu("Library") {
            item(.rescan, enabled: inLibrary && !store.busy, action: store.scan)
            item(.edit, enabled: inLibrary && !store.editingProject && store.selected != nil) { store.beginEdit(store.selected) }
            item(.duplicate, enabled: inLibrary && !store.editingProject && store.selected != nil) { store.beginEdit(store.selected, duplicate: true) }
            Divider()
            item(.copySVG, enabled: inLibrary && !store.editingProject && !store.selectedCards.isEmpty && store.selectedCards.allSatisfy(store.canExport), action: store.copySelection)
            item(.exportSVG, enabled: inLibrary && !store.editingProject && !store.selectedCards.isEmpty && store.selectedCards.allSatisfy(store.canExport), action: store.exportSelection)
        }
        CommandGroup(after: .sidebar) {
            item(.projectSource, enabled: inLibrary, action: store.toggleProjectEditor)
            item(.nextSourceTab, enabled: inLibrary && store.editingProject && !files.buffers.isEmpty) { files.cycleSourceTab(1) }
            item(.previousSourceTab, enabled: inLibrary && store.editingProject && !files.buffers.isEmpty) { files.cycleSourceTab(-1) }
            item(.splitSource, enabled: inLibrary && store.editingProject && !files.buffers.isEmpty) { files.splitSourcePane(.right) }
            item(.splitSourceDown, enabled: inLibrary && store.editingProject && !files.buffers.isEmpty) { files.splitSourcePane(.down) }
            item(.togglePDF, enabled: inLibrary, action: store.togglePDF)
            item(.inspector, enabled: inLibrary, action: store.toggleInspector)
            item(.rebuildPDF, enabled: inLibrary && store.showPDF && !pdf.building) { store.buildPDF(force: true) }
            Divider()
            item(.previousPage, enabled: inLibrary && store.showPDF && pdf.page > 1, action: pdf.previousPage)
            item(.nextPage, enabled: inLibrary && store.showPDF && pdf.page < pdf.pageCount, action: pdf.nextPage)
            item(.zoomIn, enabled: inLibrary && store.showPDF && pdf.document != nil, action: pdf.zoomIn)
            item(.zoomOut, enabled: inLibrary && store.showPDF && pdf.document != nil, action: pdf.zoomOut)
            item(.fitPDF, enabled: inLibrary && store.showPDF && pdf.document != nil, action: pdf.fit)
        }
        CommandGroup(after: .windowArrangement) {
            item(.fullScreen, enabled: noSheet, action: preferences.toggleFullScreen)
            Toggle(AppCommand.pin.title, isOn: $preferences.pinned).keyboardShortcut(shortcuts.binding(.pin).shortcut)
            Picker("Appearance", selection: $preferences.appearance) {
                Text("System").tag("system"); Text("Light").tag("light"); Text("Dark").tag("dark")
            }
            Button("Reset Opacity") { preferences.opacity = 1 }
        }
    }
    private func item(_ command: AppCommand, enabled: Bool = true, action: @escaping () -> Void) -> some View {
        Button(command == .newEquation && store.editingProject ? "New File…" : command == .copySVG ? "Copy \(store.exportFormat.title)" : command == .exportSVG ? "Export \(store.exportFormat.title)…" : command.title, action: action).keyboardShortcut(shortcuts.binding(command).shortcut).disabled(!enabled)
    }
}
