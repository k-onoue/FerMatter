import SwiftUI
import AppKit
import UniformTypeIdentifiers

@main
struct SlideMathApp: App {
    @StateObject private var store = LibraryStore()
    @StateObject private var preferences = WindowPreferences()
    @StateObject private var shortcuts = ShortcutPreferences()
    @StateObject private var templates = TemplateStore()
    var body: some Scene {
        WindowGroup(AppIdentity.name) {
            AppRootView()
                .environmentObject(store).environmentObject(preferences).environmentObject(templates).environmentObject(shortcuts)
                .environment(\.locale, Locale(identifier: "en"))
                .preferredColorScheme(preferences.colorScheme)
                .background(WindowConfiguration(preferences: preferences, shortcuts: shortcuts).ignoresSafeArea())
                .frame(minWidth: 1040, minHeight: 680)
                .onAppear { NSApp.setActivationPolicy(.regular); NSApp.activate(ignoringOtherApps: true) }
                .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in store.refreshCompletions(); if store.editingProject { store.files.refresh() } }
                .onReceive(NotificationCenter.default.publisher(for: NSApplication.didResignActiveNotification)) { _ in store.files.flushAutoSaves() }
                .onReceive(NotificationCenter.default.publisher(for: NSApplication.willTerminateNotification)) { _ in store.files.flushAutoSaves(); store.files.persistDrafts(); store.saveNow(); store.stop(); store.cancelPreview() }
        }
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1380, height: 880)
        .commands { LibraryCommands(store: store, preferences: preferences, shortcuts: shortcuts, pdf: store.pdf, files: store.files) }
        Settings {
            TabView {
                EditorSettingsView(preferences: store.files.editorPreferences).tabItem { Label("Editor", systemImage: "text.alignleft") }
                TemplateSettingsView(templates: templates).tabItem { Label("Templates", systemImage: "doc.on.doc") }
                ShortcutSettingsView(shortcuts: shortcuts).tabItem { Label("Shortcuts", systemImage: "keyboard") }
            }.frame(width: 780, height: 700)
                .environment(\.locale, Locale(identifier: "en"))
                .preferredColorScheme(preferences.colorScheme)
        }
    }
}

struct GlassSurface: ViewModifier {
    @AppStorage("opacity") private var opacity = 1.0
    var radius: CGFloat = 18
    @ViewBuilder func body(content: Content) -> some View {
        if #available(macOS 26.0, *) {
            content.background {
                RoundedRectangle(cornerRadius: radius).fill(.clear)
                    .glassEffect(.regular, in: RoundedRectangle(cornerRadius: radius))
                    .opacity(opacity)
            }
        } else { content.background { RoundedRectangle(cornerRadius: radius).fill(.regularMaterial).opacity(opacity) } }
    }
}

struct AppRootView: View {
    @EnvironmentObject private var store: LibraryStore
    @EnvironmentObject private var preferences: WindowPreferences
    @Environment(\.colorScheme) private var scheme
    var body: some View {
        Group {
            if store.showingProjects { ProjectsView() }
            else { LibraryView() }
        }
        .background {
            ZStack {
                Color(nsColor: .windowBackgroundColor).opacity(preferences.opacity)
                LinearGradient(colors: [.blue.opacity(scheme == .dark ? 0.055 : 0.025), .clear], startPoint: .topLeading, endPoint: .bottomTrailing)
                    .opacity(preferences.opacity)
            }.ignoresSafeArea()
        }
        .tint(.blue)
        .modifier(TransparentWindowSurface())
        .sheet(item: $store.editor) { EditorView(initial: $0).environmentObject(store) }
        .sheet(isPresented: $store.showingNewProject) { NewProjectView() }
        .alert(AppIdentity.name, isPresented: Binding(get: { store.alert != nil }, set: { if !$0 { store.alert = nil } })) {
            Button("OK") { store.alert = nil }
        } message: { Text(store.alert ?? "") }
    }
}

struct TransparentWindowSurface: ViewModifier {
    @ViewBuilder func body(content: Content) -> some View {
        if #available(macOS 15.0, *) { content.containerBackground(.clear, for: .window) }
        else { content }
    }
}

struct ProjectsView: View {
    @EnvironmentObject private var store: LibraryStore
    @State private var query = ""
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                AppBrandIcon(size: 36)
                Text(AppIdentity.name).font(.system(size: 18, weight: .semibold))
                Spacer()
                WindowControls()
            }.padding(.horizontal, 32).padding(.vertical, 20)
            ScrollView {
                VStack(alignment: .leading, spacing: 26) {
                    HStack(alignment: .center) {
                        Text("Projects").font(.system(size: 32, weight: .semibold))
                        Spacer()
                        SearchField(placeholder: "Search projects", text: $query).frame(width: 200)
                        Button(action: store.openProject) { Label("Open Project", systemImage: "folder") }
                            .controlSize(.large)
                        Button { store.showingNewProject = true } label: { Label("New Project", systemImage: "plus") }
                            .buttonStyle(.borderedProminent).controlSize(.large)
                    }.padding(.top, 24)
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 280, maximum: 420), spacing: 22)], alignment: .leading, spacing: 22) {
                        ForEach(store.projects.filter { query.isEmpty || $0.name.localizedCaseInsensitiveContains(query) || $0.root.localizedCaseInsensitiveContains(query) }) { project in
                            Button { store.load(URL(fileURLWithPath: project.root), rescan: false) } label: {
                                ProjectTile(project: project)
                            }.buttonStyle(.plain).help(project.root)
                                .contextMenu { Button("Show in Finder") { NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: project.root) } }
                        }
                        if query.isEmpty {
                            Button { store.showingNewProject = true } label: {
                                VStack(spacing: 14) {
                                    Image(systemName: "plus").font(.system(size: 26, weight: .light))
                                    Text("New Project").font(.system(size: 13, weight: .medium))
                                }.foregroundStyle(.secondary).frame(maxWidth: .infinity).frame(height: 230)
                                    .background(.primary.opacity(0.025), in: RoundedRectangle(cornerRadius: 20))
                                    .overlay(RoundedRectangle(cornerRadius: 20).strokeBorder(.primary.opacity(0.12), style: StrokeStyle(lineWidth: 1, dash: [5, 5])))
                            }.buttonStyle(.plain)
                        }
                    }
                    if !query.isEmpty && !store.projects.contains(where: { $0.name.localizedCaseInsensitiveContains(query) || $0.root.localizedCaseInsensitiveContains(query) }) {
                        ContentUnavailableView.search(text: query)
                    }
                }.padding(.horizontal, 48).padding(.bottom, 40).frame(maxWidth: 1450)
            }
        }
    }
}

struct ProjectTile: View {
    @EnvironmentObject private var preferences: WindowPreferences
    let project: ProjectSummary
    @State private var hovered = false
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ZStack {
                RoundedRectangle(cornerRadius: 14).fill(.blue.opacity(0.045))
                if let equation = project.preview { EquationImage(card: equation).padding(22) }
                else { Image(systemName: "folder").font(.system(size: 32, weight: .light)).foregroundStyle(.secondary) }
            }.frame(height: 130)
            HStack {
                Text(project.name).font(.system(size: 16, weight: .semibold)).lineLimit(1)
                Spacer()
                Image(systemName: project.available ? "arrow.up.right" : "exclamationmark.triangle").foregroundStyle(.secondary)
            }.padding(.top, 16)
            HStack {
                Text(project.format.displayName + " · " + (project.figureCount > 0 ? "\(project.count) items" : equationCountLabel(project.count)))
                Spacer()
                Text(project.updatedLabel)
            }.font(.system(size: 11)).foregroundStyle(.secondary).padding(.top, 7)
        }.padding(20).frame(height: 230)
            .background(Color(nsColor: .textBackgroundColor).opacity(preferences.opacity), in: RoundedRectangle(cornerRadius: 20))
            .overlay(RoundedRectangle(cornerRadius: 20).strokeBorder(hovered ? Color.accentColor.opacity(0.5) : Color.primary.opacity(0.07)))
            .onHover { hovered = $0 }
    }
}

struct SearchField: View {
    var placeholder = "Search equations"
    @Binding var text: String
    @FocusState private var focused: Bool
    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass").foregroundStyle(.secondary)
            TextField(placeholder, text: $text).textFieldStyle(.plain).focused($focused)
                .onReceive(NotificationCenter.default.publisher(for: .findEquations)) { _ in focused = true }
            if !text.isEmpty {
                Button { text = "" } label: { Image(systemName: "xmark.circle.fill").foregroundStyle(.tertiary) }.buttonStyle(.plain).help("Clear search")
            }
        }.font(.system(size: 12)).padding(.horizontal, 12).padding(.vertical, 9)
            .background(.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 10))
    }
}

struct SectionDraft: Identifiable {
    let id = UUID()
    var section: EquationSection?
}

struct LibraryView: View {
    @EnvironmentObject private var shortcuts: ShortcutPreferences
    @EnvironmentObject private var store: LibraryStore
    @EnvironmentObject private var preferences: WindowPreferences
    @State private var sectionEditor: SectionDraft?
    @State private var deletingSection: EquationSection?
    @State private var showProjectSettings = false
    var body: some View {
        RememberedSplitView(widths: store.paneWidths, pane: store.showPDF ? .pdf : store.showInspector ? .inspector : nil) {
            HSplitView {
                sidebar.frame(minWidth: 205, idealWidth: 220, maxWidth: 320)
                VStack(spacing: 0) {
                    toolbar.padding(18)
                    ZStack {
                        libraryContent.opacity(store.editingProject ? 0 : 1)
                            .allowsHitTesting(!store.editingProject).accessibilityHidden(store.editingProject)
                        ProjectSourcePane(files: store.files, active: store.editingProject)
                            .opacity(store.editingProject ? 1 : 0)
                            .allowsHitTesting(store.editingProject).accessibilityHidden(!store.editingProject)
                    }
                }.frame(minWidth: 480)
            }
        } trailing: {
            if store.showPDF { PDFPane(session: store.pdf) }
            else { InspectorView() }
        }
        .background(SourceShortcutRouting(files: store.files, shortcuts: shortcuts, active: store.editingProject))
        .sheet(item: $sectionEditor) { draft in SectionNameSheet(section: draft.section).environmentObject(store) }
        .confirmationDialog("Delete label?", isPresented: Binding(get: { deletingSection != nil }, set: { if !$0 { deletingSection = nil } })) {
            Button("Delete Label", role: .destructive) { if let section = deletingSection { store.deleteSection(section) }; deletingSection = nil }
        } message: { Text("Equations will stay in All equations.") }
    }

    private var libraryContent: some View {
        VStack(spacing: 0) {
            if store.filtered.isEmpty {
                Spacer()
                ContentUnavailableView {
                    Label(store.query.isEmpty ? "No equations" : "No results", systemImage: store.query.isEmpty ? "square.stack.3d.up" : "magnifyingglass")
                } actions: {
                    if store.query.isEmpty { Button("New Equation") { store.beginEdit(nil) } }
                }
                Spacer()
            } else {
                EquationCanvas()
            }
            HStack(spacing: 8) {
                Text(store.countLabel(store.filtered))
                if store.selectedIDs.count > 1 {
                    Text("\(store.selectedIDs.count) selected").foregroundStyle(.primary)
                    Button("Export…", action: store.exportSelection).buttonStyle(.plain)
                        .disabled(!store.selectedCards.allSatisfy(store.canExport))
                }
                if store.busy {
                    ProgressView().controlSize(.mini)
                    Text(store.status)
                    Button("Stop", action: store.stop).buttonStyle(.plain)
                } else if !store.status.isEmpty { Text(store.status) }
                Spacer()
                if store.failedCount > 0 { Label("\(store.failedCount) failed", systemImage: "exclamationmark.circle").foregroundStyle(.orange) }
                HStack(spacing: 7) {
                    ExportFormatPicker().frame(width: 82)
                    Image(systemName: "textformat.size").help("Export Font Size")
                    Picker("Export Font Size", selection: $store.exportFontSize) {
                        Text("Original").tag(0.0)
                        ForEach([10, 12, 14, 16, 18, 20, 24, 28, 32, 36, 40, 48, 60, 72, 96], id: \.self) { Text("\($0) pt").tag(Double($0)) }
                    }.labelsHidden().frame(width: 92).onChange(of: store.exportFontSize) { _, _ in store.saveNow(); store.prepareFontMetrics() }
                    if store.fontMetricsBusy { ProgressView().controlSize(.mini) }
                }
            }.font(.system(size: 11)).foregroundStyle(.secondary).padding(.horizontal, 24).padding(.vertical, 12)
        }
    }

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 22) {
            Button(action: store.showProjects) {
                Label("Projects", systemImage: "chevron.left").font(.system(size: 12, weight: .medium)).foregroundStyle(.secondary)
            }.buttonStyle(.plain).padding(.top, 12)
            HStack(spacing: 10) {
                Image(systemName: "folder.fill").foregroundStyle(.blue).font(.system(size: 18))
                Text(store.projectName).font(.system(size: 19, weight: .semibold)).lineLimit(1)
            }.padding(.bottom, 5)
            Picker("Project mode", selection: Binding(get: { store.editingProject }, set: { if $0 != store.editingProject { store.toggleProjectEditor() } })) {
                Text("Library").tag(false)
                Text("Source").tag(true)
            }.pickerStyle(.segmented).labelsHidden()
            if store.editingProject {
                SourceSidebar(files: store.files)
            } else {
                VStack(spacing: 5) {
                    sectionRow(store.allItemsTitle, icon: "square.grid.2x2", id: "all", count: store.cards.count)
                }
                ScrollView {
                    VStack(alignment: .leading, spacing: 20) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Outline").font(.system(size: 11, weight: .medium)).foregroundStyle(.secondary).padding(.horizontal, 10).padding(.bottom, 6)
                            if store.cards.contains(where: { ($0.outlinePath ?? []).isEmpty }) {
                                sectionRow("No heading", icon: "text.alignleft", id: "outline:none", count: store.cards.filter { ($0.outlinePath ?? []).isEmpty }.count)
                            }
                            ForEach(store.visibleOutline) { heading in OutlineRow(heading: heading) }
                        }
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text("Labels").font(.system(size: 11, weight: .medium)).foregroundStyle(.secondary)
                                Spacer()
                                Button { sectionEditor = SectionDraft() } label: { Image(systemName: "plus") }.buttonStyle(.plain).help("New Label").accessibilityLabel("New Label")
                            }.padding(.horizontal, 10).padding(.bottom, 6)
                            sectionRow("Unlabeled", icon: "tag", id: "unsectioned", count: store.cards.filter { store.sectionAssignments[$0.id] == nil }.count)
                            ForEach(store.sections) { section in
                                sectionRow(section.name, icon: "tag", id: section.id, count: store.sectionAssignments.values.filter { $0 == section.id }.count)
                                    .contextMenu {
                                        Button("Rename…") { sectionEditor = SectionDraft(section: section) }
                                        Button("Delete Label…", role: .destructive) { deletingSection = section }
                                    }
                            }
                        }
                    }
                }
            }
            Spacer(minLength: 0)
            Button { showProjectSettings.toggle() } label: {
                HStack { Label("Project Settings", systemImage: "gearshape"); Spacer() }.font(.system(size: 11)).foregroundStyle(.secondary)
            }.buttonStyle(.plain).padding(10)
                .popover(isPresented: $showProjectSettings) { ProjectSettingsView().environmentObject(store) }
        }.padding(.horizontal, 16).padding(.bottom, 14)
            .background { Rectangle().fill(.regularMaterial).opacity(preferences.opacity) }
    }

    private func sectionRow(_ title: String, icon: String, id: String, count: Int) -> some View {
        Button { store.selectSection(id) } label: {
            HStack(spacing: 9) {
                Image(systemName: icon).frame(width: 17)
                Text(title).lineLimit(1)
                Spacer(minLength: 3)
                Text("\(count)").font(.system(size: 10)).monospacedDigit().foregroundStyle(.secondary)
            }.font(.system(size: 12, weight: store.sectionFilter == id ? .medium : .regular))
                .padding(.horizontal, 10).padding(.vertical, 10)
                .background(store.sectionFilter == id ? Color.accentColor.opacity(0.12) : .clear, in: RoundedRectangle(cornerRadius: 10))
                .foregroundStyle(store.sectionFilter == id ? Color.accentColor : Color.primary)
        }.buttonStyle(.plain)
    }

    private var toolbar: some View {
        VStack(spacing: 16) {
            HStack(spacing: 10) {
                Text(store.editingProject ? "Source" : "Library").font(.system(size: 23, weight: .semibold)).lineLimit(1)
                Spacer(minLength: 8)
                WindowControls()
                Button(action: store.togglePDF) { Image(systemName: "doc.text").frame(width: 28, height: 28) }
                    .buttonStyle(.plain).foregroundStyle(store.showPDF ? Color.accentColor : Color.secondary).help("Toggle PDF").accessibilityLabel("Toggle PDF")
                Button(action: store.toggleInspector) { Image(systemName: "sidebar.right").frame(width: 28, height: 28) }
                    .buttonStyle(.plain).foregroundStyle(.secondary).help("Toggle Inspector").accessibilityLabel("Toggle Inspector")
                Menu {
                    Button("New Equation") { store.beginEdit(nil) }
                    Button("New Figure", action: store.newFigure)
                } label: { Image(systemName: "plus").fontWeight(.medium).frame(width: 23, height: 23) }
                    .menuStyle(.borderlessButton).menuIndicator(.hidden).frame(width: 28).help("New Item").accessibilityLabel("New Item")
            }
            if !store.editingProject {
            HStack(spacing: 12) {
                Picker("Equation type", selection: $store.kind) {
                    Text("All").tag("all"); Text("Inline").tag("inline"); Text("Block").tag("block"); Text("Figures").tag("figure")
                }.pickerStyle(.segmented).labelsHidden().frame(width: 220)
                Spacer(minLength: 0)
                SearchField(placeholder: store.hasFigures ? "Search library" : "Search equations", text: $store.query).frame(minWidth: 100, maxWidth: 240)
                Menu {
                    Picker("Origin", selection: $store.origin) { Text("All origins").tag("all"); Text("Source").tag("source"); Text("Custom").tag("custom") }
                    Picker("Sort", selection: $store.sort) { Text("Source order").tag("source"); Text("Custom order").tag("custom") }
                    Divider()
                    Toggle("Compact Cards", isOn: $store.compact)
                } label: { Image(systemName: "line.3.horizontal.decrease") }
                .menuStyle(.borderlessButton).frame(width: 23).help("Filter & Sort").accessibilityLabel("Filter & Sort")
            }
            }
        }.padding(18).modifier(GlassSurface())
    }
}

struct OutlineRow: View {
    @EnvironmentObject private var store: LibraryStore
    let heading: DocumentHeading
    private var selected: Bool { store.sectionFilter == "outline:" + heading.id }
    private var hasChildren: Bool { store.outline.contains { $0.path.dropLast().last == heading.id } }
    var body: some View {
        HStack(spacing: 4) {
            if hasChildren {
                Button { store.toggleOutline(heading) } label: {
                    Image(systemName: store.collapsedOutline.contains(heading.id) ? "chevron.right" : "chevron.down")
                        .font(.system(size: 9, weight: .semibold)).frame(width: 14, height: 30)
                }.buttonStyle(.plain).accessibilityLabel((store.collapsedOutline.contains(heading.id) ? "Expand " : "Collapse ") + heading.title)
            } else { Color.clear.frame(width: 14, height: 1) }
            Button { store.selectSection("outline:" + heading.id) } label: {
                HStack(spacing: 5) {
                    Text(heading.title).lineLimit(2).multilineTextAlignment(.leading)
                    Spacer(minLength: 2)
                    Text("\(store.outlineCount(heading))").font(.system(size: 10)).monospacedDigit().foregroundStyle(.secondary)
                }.font(.system(size: 12, weight: selected ? .medium : .regular))
                    .padding(.vertical, 9).contentShape(Rectangle())
            }.buttonStyle(.plain).help(heading.title + " · " + heading.file + ":\(heading.line)")
        }
        .padding(.leading, 6 + CGFloat(max(0, heading.path.count - 1)) * 12).padding(.trailing, 10)
        .foregroundStyle(selected ? Color.accentColor : Color.primary)
        .background(selected ? Color.accentColor.opacity(0.12) : .clear, in: RoundedRectangle(cornerRadius: 9))
    }
}

struct ProjectSettingsView: View {
    @EnvironmentObject var store: LibraryStore
    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Project Settings").font(.headline)
            LabeledContent("Main file") {
                Picker("Main file", selection: Binding(get: { store.main }, set: { store.main = $0; store.scan() })) {
                    ForEach(store.candidates, id: \.self) { Text($0).tag($0) }
                }.labelsHidden().disabled(store.busy)
            }
            LabeledContent("Compiler") {
                if store.format == .typst { Text("Typst") }
                else {
                    Picker("Compiler", selection: $store.engine) {
                        Text("Project default").tag("project"); Text("pdfLaTeX").tag("pdflatex"); Text("XeLaTeX").tag("xelatex"); Text("LuaLaTeX").tag("lualatex")
                    }.labelsHidden().disabled(store.busy).onChange(of: store.engine) { _, _ in store.saveNow(); store.pdf.invalidate(); if store.showPDF { store.buildPDF() } }
                }
            }
            Divider()
            HStack {
                Button("Rescan", systemImage: "arrow.clockwise", action: store.scan)
                Button("Render All", systemImage: "play", action: store.renderAll).disabled(!store.canRender)
            }.disabled(store.busy)
            Button("Show in Finder") { NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: store.root) }.buttonStyle(.link)
            if !store.warnings.isEmpty {
                DisclosureGroup("\(store.warnings.count) warnings") {
                    ScrollView { Text(store.warnings.joined(separator: "\n")).font(.caption).textSelection(.enabled) }.frame(maxHeight: 160)
                }.foregroundStyle(.orange)
            }
        }.font(.system(size: 12)).padding(24).frame(width: 330)
    }
}

struct SectionNameSheet: View {
    @EnvironmentObject var store: LibraryStore
    @Environment(\.dismiss) private var dismiss
    let section: EquationSection?
    @State private var name = ""
    @FocusState private var focused: Bool
    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text(section == nil ? "New Label" : "Rename Label").font(.title2.weight(.semibold))
            TextField("Label name", text: $name).textFieldStyle(.roundedBorder).focused($focused).onSubmit(save)
            HStack { Spacer(); Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction); Button(section == nil ? "Create" : "Save", action: save).keyboardShortcut(.defaultAction).disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) }
        }.padding(26).frame(width: 340).onAppear { name = section?.name ?? ""; focused = true }
    }
    private func save() {
        guard !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        if let section { store.renameSection(section, to: name) } else { store.addSection(name) }
        dismiss()
    }
}
