import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct NewProjectView: View {
    @EnvironmentObject private var store: LibraryStore
    @EnvironmentObject private var templates: TemplateStore
    @Environment(\.dismiss) private var dismiss
    @State private var name = "Untitled"
    @State private var templateID = ""
    @State private var text = ""
    @State private var mode: ProjectInputMode = .automatic
    @State private var directory: URL?
    @State private var error: String?
    @State private var managingTemplates = false
    @StateObject private var sourceController = SourceEditorController()
    private var template: ProjectTemplate { templates.templates.first { $0.id == templateID } ?? templates.defaultTemplate }
    private var location: URL { directory ?? AppIdentity.newProjectDirectory }
    private var interpretation: String {
        (try? ProjectComposer.prepare(text, template: template, mode: mode).interpretation) ?? ""
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("New Project").font(.title2.weight(.semibold))
            HStack(spacing: 16) {
                TextField("Project name", text: $name).textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Project name")
                Picker("Template", selection: $templateID) {
                    ForEach(templates.templates) { Text($0.name).tag($0.id) }
                }.frame(width: 255)
                Button { managingTemplates = true } label: { Image(systemName: "slider.horizontal.3") }
                    .help("Manage templates").accessibilityLabel("Manage templates")
            }
            VStack(spacing: 0) {
                HStack {
                    Button("Paste", systemImage: "doc.on.clipboard") {
                        guard let value = NSPasteboard.general.string(forType: .string) else { error = "The clipboard has no text."; return }
                        text = value; error = nil
                    }
                    Button("Load File…", systemImage: "doc") { loadFile() }
                    Spacer()
                    Picker("Input", selection: $mode) {
                        ForEach(ProjectInputMode.allCases) { Text($0.title).tag($0) }
                    }.pickerStyle(.segmented).labelsHidden().frame(width: 270)
                }.padding(12)
                Divider()
                ZStack(alignment: .topLeading) {
                    SourceTextEditor(text: $text, label: "Project source", format: template.format, controller: sourceController)
                        .modifier(GlassSurface(radius: 10))
                    if text.isEmpty {
                        Text(template.format == .latex ? "x^2 + y^2 = r^2\n\\int_0^1 x^2\\,dx = \\frac{1}{3}" : "x^2 + y^2 = r^2\nintegral_0^1 x^2 dif x = 1/3")
                            .font(.system(size: 13, design: .monospaced)).foregroundStyle(.secondary)
                            .padding(.horizontal, 13).padding(.vertical, 10).allowsHitTesting(false)
                    }
                }.frame(minHeight: 270)
                Divider()
                HStack {
                    Text(interpretation).font(.caption).foregroundStyle(.secondary)
                    Spacer()
                    Text(template.format.displayName).font(.caption.weight(.medium)).foregroundStyle(.secondary)
                }.padding(.horizontal, 12).frame(height: 30)
            }.background(Color(nsColor: .textBackgroundColor), in: RoundedRectangle(cornerRadius: 12))
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.primary.opacity(0.1)))
            HStack {
                Image(systemName: "folder").foregroundStyle(.secondary)
                Text((location.path as NSString).abbreviatingWithTildeInPath).lineLimit(1).truncationMode(.middle)
                    .font(.caption).foregroundStyle(.secondary).help(location.path)
                Button("Change…") { chooseDirectory() }.controlSize(.small)
                Spacer()
            }
            HStack {
                if let error { Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled) }
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Button("Create Project") { create() }.buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
                    .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }.padding(26).frame(width: 760, height: 580)
            .onAppear { templateID = templates.defaultTemplate.id }
            .onChange(of: templateID) { _, _ in error = nil }
            .sheet(isPresented: $managingTemplates) {
                VStack(spacing: 0) {
                    TemplateSettingsView(templates: templates)
                    Divider()
                    HStack { Spacer(); Button("Done") { managingTemplates = false }.keyboardShortcut(.cancelAction) }.padding(18)
                }.frame(width: 720, height: 560)
            }
    }

    private func loadFile() {
        let panel = NSOpenPanel()
        panel.title = "Load Source"; panel.prompt = "Load"
        panel.allowedContentTypes = [.plainText, UTType(filenameExtension: "tex") ?? .plainText, UTType(filenameExtension: "typ") ?? .plainText]
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            text = try String(contentsOf: url, encoding: .utf8)
            if name == "Untitled" { name = url.deletingPathExtension().lastPathComponent }
            if ["tex", "typ"].contains(url.pathExtension.lowercased()) {
                let format: ProjectFormat = url.pathExtension.lowercased() == "typ" ? .typst : .latex
                if template.format != format { templateID = ProjectTemplate.builtIns.first { $0.format == format }!.id }
            }
            error = nil
        } catch { self.error = error.localizedDescription }
    }

    private func chooseDirectory() {
        let panel = NSOpenPanel()
        panel.title = "Project Location"; panel.prompt = "Choose"
        panel.canChooseDirectories = true; panel.canChooseFiles = false; panel.canCreateDirectories = true
        panel.directoryURL = FileManager.default.fileExists(atPath: location.path) ? location : location.deletingLastPathComponent()
        if panel.runModal() == .OK { directory = panel.url }
    }

    private func create() {
        do {
            let main = try ProjectComposer.create(name: name, text: text, template: template, mode: mode, directory: location)
            dismiss()
            store.load(main)
        } catch { self.error = error.localizedDescription }
    }
}

struct TemplateSettingsView: View {
    @ObservedObject var templates: TemplateStore
    @State private var editing: ProjectTemplate?
    @State private var deleting: ProjectTemplate?
    @State private var error: String?
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Templates").font(.title2.weight(.semibold))
                Spacer()
                Menu {
                    ForEach(ProjectTemplate.builtIns) { template in
                        Button(template.name) { editing = template.customized() }
                    }
                } label: { Label("New Template", systemImage: "plus") }
            }.padding(24)
            Divider()
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(templates.templates) { template in
                        HStack(spacing: 16) {
                            Image(systemName: "doc.text")
                                .font(.title3).foregroundStyle(.blue).frame(width: 30)
                            VStack(alignment: .leading, spacing: 4) {
                                Text(template.name).font(.headline)
                                Text(template.isBuiltIn ? "Built-in" : template.format.displayName).font(.caption).foregroundStyle(.secondary)
                            }
                            Spacer()
                            if templates.defaultTemplate.id == template.id {
                                Text("Default").font(.caption).foregroundStyle(.secondary)
                            }
                            Button(template.isBuiltIn ? "Customize…" : "Edit…") {
                                editing = template.isBuiltIn ? template.customized() : template
                            }
                            Menu {
                                Button("Set as Default") { perform { try templates.setDefault(template) } }
                                Button("Duplicate") { editing = template.customized() }
                                if !template.isBuiltIn {
                                    Divider()
                                    Button("Delete", role: .destructive) { deleting = template }
                                }
                            } label: { Image(systemName: "ellipsis") }.menuStyle(.borderlessButton).frame(width: 24)
                                .accessibilityLabel("Options for \(template.name)")
                        }.padding(.horizontal, 24).padding(.vertical, 16)
                            .contentShape(Rectangle()).onTapGesture(count: 2) { editing = template.isBuiltIn ? template.customized() : template }
                        Divider().padding(.leading, 70)
                    }
                }
            }
            if let message = error ?? templates.error {
                Text(message).font(.caption).foregroundStyle(.red).padding(18)
            }
        }.sheet(item: $editing) { template in
            TemplateEditorView(templates: templates, initial: template)
        }.alert("Delete Template?", isPresented: Binding(get: { deleting != nil }, set: { if !$0 { deleting = nil } })) {
            Button("Cancel", role: .cancel) { deleting = nil }
            Button("Delete", role: .destructive) { if let deleting { perform { try templates.delete(deleting) } }; deleting = nil }
        } message: { Text("Existing projects keep their source files.") }
    }
    private func perform(_ action: () throws -> Void) {
        do { try action(); error = nil } catch { self.error = error.localizedDescription }
    }
}

struct TemplateEditorView: View {
    @ObservedObject var templates: TemplateStore
    let initial: ProjectTemplate
    @Environment(\.dismiss) private var dismiss
    @State private var draft: ProjectTemplate
    @State private var error: String?
    init(templates: TemplateStore, initial: ProjectTemplate) {
        self.templates = templates; self.initial = initial; _draft = State(initialValue: initial)
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                Text("Edit Template").font(.title2.weight(.semibold))
                Spacer()
                Text(draft.format.displayName).foregroundStyle(.secondary)
            }
            HStack {
                TextField("Template name", text: $draft.name).textFieldStyle(.roundedBorder).accessibilityLabel("Template name")
                if draft.format == .latex {
                    Picker("Compiler", selection: $draft.compiler) {
                        Text("pdfLaTeX").tag("pdflatex"); Text("XeLaTeX").tag("xelatex"); Text("LuaLaTeX").tag("lualatex")
                    }.frame(width: 240)
                }
            }
            AssistedSourceEditor(text: $draft.source, label: "Template source", format: draft.format)
                .overlay(RoundedRectangle(cornerRadius: 8).strokeBorder(.primary.opacity(0.12)))
            Text("{{content}} marks the pasted content.").font(.caption).foregroundStyle(.secondary)
            HStack {
                if let error { Text(error).font(.caption).foregroundStyle(.red) }
                Spacer()
                Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction)
                Button("Save Template") {
                    do { try templates.save(draft); dismiss() } catch { self.error = error.localizedDescription }
                }.buttonStyle(.borderedProminent).keyboardShortcut(.defaultAction)
            }
        }.padding(26).frame(width: 700, height: 550)
    }
}
