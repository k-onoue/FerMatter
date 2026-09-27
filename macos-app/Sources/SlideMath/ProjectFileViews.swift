import SwiftUI
import AppKit

struct ProjectFileTree: View {
    @ObservedObject var files: ProjectFiles
    @State private var creation: FileCreation?
    private struct FileCreation: Identifiable {
        let id = UUID()
        let parent: String
        let directory: Bool
    }
    var body: some View {
        VStack(spacing: 8) {
            HStack {
                Text("Files").font(.system(size: 11, weight: .medium)).foregroundStyle(.secondary)
                Spacer()
                Button(action: files.refresh) { Image(systemName: "arrow.clockwise") }.help("Refresh Files").accessibilityLabel("Refresh Files")
                Menu {
                    Button("New File…") { creation = FileCreation(parent: files.destinationFolder, directory: false) }
                    Button("New Folder…") { creation = FileCreation(parent: files.destinationFolder, directory: true) }
                } label: { Image(systemName: "plus") }.menuStyle(.borderlessButton).menuIndicator(.hidden).frame(width: 20).accessibilityLabel("New File or Folder")
            }.buttonStyle(.plain).padding(.horizontal, 8)
            NativeProjectFileTree(files: files) { parent, directory in
                creation = FileCreation(parent: parent, directory: directory)
            }

        }
        .sheet(item: $creation) { draft in
            FileNameSheet(directory: draft.directory) { name in
                try files.create(name: name, parent: draft.parent, directory: draft.directory)
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: .newSourceFile)) { _ in
            creation = FileCreation(parent: files.destinationFolder, directory: false)
        }
    }
}

private struct FileNameSheet: View {
    let directory: Bool
    let create: (String) throws -> String
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var error: String?
    @FocusState private var focused: Bool
    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text(directory ? "New Folder" : "New File").font(.title2.weight(.semibold))
            TextField(directory ? "Folder name" : "File name", text: $name).textFieldStyle(.roundedBorder).focused($focused).onSubmit(save)
            if let error { Text(error).font(.caption).foregroundStyle(.red) }
            HStack { Spacer(); Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction); Button("Create", action: save).keyboardShortcut(.defaultAction).disabled(name.isEmpty) }
        }.padding(24).frame(width: 370).onAppear { focused = true }
    }
    private func save() { do { _ = try create(name); dismiss() } catch { self.error = error.localizedDescription } }
}

struct ProjectSourcePane: View {
    @ObservedObject var files: ProjectFiles
    let active: Bool
    var body: some View {
        VStack(spacing: 0) {
            if let error = files.error {
                HStack(alignment: .top) {
                    Image(systemName: "exclamationmark.triangle").foregroundStyle(.orange)
                    Text(error).font(.callout).textSelection(.enabled)
                    Spacer()
                    Button { files.error = nil } label: { Image(systemName: "xmark") }.buttonStyle(.plain)
                }.padding(12).background(.orange.opacity(0.08)).padding(.horizontal, 16)
            }
            SourcePaneBranch(files: files, layout: files.sourceLayout, active: active)
        }
        .alert("Save changes before closing?", isPresented: Binding(get: { files.pendingTabClose != nil }, set: { if !$0 { files.pendingTabClose = nil } }), presenting: files.pendingTabClose) { request in
            Button("Save") { files.confirmCloseTab(request, saveChanges: true) }
            Button("Don’t Save", role: .destructive) { files.confirmCloseTab(request, saveChanges: false) }
            Button("Cancel", role: .cancel) { files.pendingTabClose = nil }
        } message: { request in Text(request.path) }
    }
}

private struct SourcePaneBranch: View {
    @ObservedObject var files: ProjectFiles
    let layout: SourcePaneLayout
    let active: Bool
    var body: some View {
        switch layout {
        case .pane(let id): SourceEditorGroup(files: files, pane: id, active: active).frame(minWidth: 220, minHeight: 180)
        case .split(let axis, let first, let second):
            if axis == .right {
                HSplitView {
                    AnyView(SourcePaneBranch(files: files, layout: first, active: active))
                    AnyView(SourcePaneBranch(files: files, layout: second, active: active))
                }
            } else {
                VSplitView {
                    AnyView(SourcePaneBranch(files: files, layout: first, active: active))
                    AnyView(SourcePaneBranch(files: files, layout: second, active: active))
                }
            }
        }
    }
}

private struct SourceEditorGroup: View {
    @ObservedObject var files: ProjectFiles
    let pane: Int
    let active: Bool
    @State private var reloading = false
    private var path: String? { files.path(in: pane) }
    private var current: SourceFileBuffer? { files.buffers.first { $0.path == path } }
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                ScrollView(.horizontal) {
                    HStack(spacing: 2) {
                        ForEach(files.tabs(in: pane)) { buffer in
                            SourceFileTab(files: files, buffer: buffer, pane: pane, selected: path == buffer.path)
                        }
                    }
                }
                Menu {
                    Button("Split Right", systemImage: "rectangle.split.2x1") { files.splitSourcePane(.right, pane: pane) }
                    Button("Split Down", systemImage: "rectangle.split.1x2") { files.splitSourcePane(.down, pane: pane) }
                    if let current {
                        Divider()
                        Button("Revert File…") { files.activeSourcePane = pane; reloading = current.dirty; if !current.dirty { files.reload(current.path) } }
                    }
                    if files.splitSource {
                        Divider()
                        Button("Close Pane", systemImage: "xmark") { files.closeSourcePane(pane) }
                    }
                } label: { Image(systemName: "rectangle.split.2x2") }
                    .menuStyle(.borderlessButton).menuIndicator(.hidden).frame(width: 24)
                    .help("Split Editor").accessibilityLabel("Split Editor Pane \(pane + 1)")
            }.padding(.horizontal, 12)
            if let current {
                if files.conflicts.contains(current.path) {
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.triangle").foregroundStyle(.orange)
                        Text("Changed on disk").font(.system(size: 11))
                        Spacer()
                        Button("Reload…") { files.activeSourcePane = pane; reloading = true }.controlSize(.small)
                        Button("Save Copy…") { files.activeSourcePane = pane; files.saveCopy() }.controlSize(.small)
                    }.padding(.horizontal, 12).padding(.vertical, 8)
                }
                ZStack {
                    ForEach(files.tabs(in: pane)) { buffer in
                        ProjectFileEditor(files: files, path: buffer.path, pane: pane,
                            visible: active && path == buffer.path, focused: active && path == buffer.path && files.activeSourcePane == pane)
                            .opacity(path == buffer.path ? 1 : 0)
                            .allowsHitTesting(path == buffer.path).accessibilityHidden(path != buffer.path)
                    }
                }.padding(.horizontal, 12).padding(.top, 6).padding(.bottom, 12)
            } else { ContentUnavailableView("Select a file", systemImage: "doc.text") }
        }
        .overlay { NativeSourceDropZone(files: files, pane: pane) }
        .overlay(alignment: .top) { if files.splitSource && files.activeSourcePane == pane { Rectangle().fill(Color.accentColor).frame(height: 2).allowsHitTesting(false) } }
        .confirmationDialog("Reload this file and discard its unsaved edits?", isPresented: $reloading) {
            Button("Reload", role: .destructive) { if let path { files.reload(path) } }
        }
    }
}

private struct SourceFileTab: View {
    @ObservedObject var files: ProjectFiles
    let buffer: SourceFileBuffer
    let pane: Int
    let selected: Bool
    var body: some View {
        NativeSourceTab(files: files, buffer: buffer, pane: pane)
            .frame(width: max(90, min(240, ((buffer.path as NSString).lastPathComponent as NSString).size(withAttributes: [.font: NSFont.systemFont(ofSize: 12)]).width + 40)), height: 34)
            .background(selected ? Color.accentColor.opacity(0.12) : .clear, in: RoundedRectangle(cornerRadius: 7))
            .help(buffer.path)
        .contextMenu {
            Button("Close Tab") { files.requestCloseTab(buffer.path, pane: pane) }
            Divider()
            Button("Split Right") { files.splitSourcePane(.right, pane: pane, path: buffer.path) }
            Button("Split Down") { files.splitSourcePane(.down, pane: pane, path: buffer.path) }
        }
    }
}

private struct ProjectFileEditor: View {
    @ObservedObject var files: ProjectFiles
    let path: String
    let pane: Int
    let visible: Bool
    let focused: Bool
    private var format: ProjectFormat { path.hasSuffix(".typ") ? .typst : files.format }
    var body: some View {
        SourceTextEditor(text: Binding(get: { [weak files, path] in files?.buffers.first { $0.path == path }?.text ?? "" }, set: { [weak files, path] in files?.update(path, text: $0) }),
                label: "Source: " + path + (files.splitSource ? " (Pane \(pane + 1))" : ""), format: format, context: .document,
                controller: files.controller(path, pane: pane), projectCompletions: files.completions, isVisible: visible,
                onFocus: { [weak files, pane] in
                    guard let files else { return }
                    if files.activeSourcePane != pane { files.activeSourcePane = pane }
                    files.wantsEditorFocus = true
                })
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .modifier(GlassSurface(radius: 8))
            .onChange(of: focused) { _, focused in if focused { focus() } }
            .onAppear { if focused { focus() } }
    }
    private func focus() {
        DispatchQueue.main.async {
            guard files.wantsEditorFocus, files.activeSourcePane == pane, files.currentPath == path else { return }
            if let view = files.controller(path, pane: pane).view { view.window?.makeFirstResponder(view) }
        }
    }
}
