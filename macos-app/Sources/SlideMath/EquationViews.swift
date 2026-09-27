import SwiftUI
import AppKit
import UniformTypeIdentifiers

struct EquationImage: View {
    let card: Equation
    @State private var image: NSImage?
    var body: some View {
        Group {
            if let image {
                Image(nsImage: image).resizable().renderingMode(card.kind == "figure" ? .original : .template).interpolation(.high).scaledToFit()
                    .frame(maxHeight: card.kind == "inline" ? 42 : card.kind == "figure" ? 170 : 120).foregroundStyle(.primary)
                    .padding(card.kind == "figure" ? 8 : 0)
                    .background(card.kind == "figure" ? Color.white : .clear, in: RoundedRectangle(cornerRadius: 8))
            } else {
                Image(systemName: card.status == "failed" ? "exclamationmark.triangle" : "function")
                    .foregroundStyle(card.status == "failed" ? Color.orange : Color.secondary.opacity(0.4))
                    .help(card.status == "failed" ? "Render error" : "Pending")
            }
        }.task(id: card.pngPath) { image = card.pngPath.flatMap { NSImage(contentsOfFile: $0) } }
    }
}

struct EquationTile: View {
    @EnvironmentObject private var store: LibraryStore
    @State private var hovered = false
    let card: Equation
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(card.kind.capitalized).font(.system(size: 10)).foregroundStyle(.tertiary)
                Spacer()
                if card.origin == "custom" { Image(systemName: "pencil").font(.system(size: 9)).foregroundStyle(.blue.opacity(0.7)) }
                if card.status == "failed" { Image(systemName: "exclamationmark.circle.fill").font(.system(size: 10)).foregroundStyle(.orange) }
                if card.status == "pending" { Image(systemName: "circle.dotted").font(.system(size: 10)).foregroundStyle(.secondary) }
            }
            EquationImage(card: card).frame(maxWidth: .infinity).frame(height: store.compact ? 72 : 112).padding(.vertical, 14)
            Text(card.name).font(.system(size: 12, weight: .medium)).lineLimit(1)
            HStack(spacing: 5) {
                Text(store.sectionName(for: card) ?? card.location).lineLimit(1).truncationMode(.middle)
                Spacer(minLength: 0)
                if store.sort == "custom" {
                    Button { store.move(card, by: -1) } label: { Image(systemName: "arrow.up") }.help("Move Up")
                    Button { store.move(card, by: 1) } label: { Image(systemName: "arrow.down") }.help("Move Down")
                }
            }.font(.system(size: 10)).foregroundStyle(.tertiary).padding(.top, 6).buttonStyle(.plain)
        }.padding(17)
            .modifier(GlassSurface(radius: 16))
            .overlay(RoundedRectangle(cornerRadius: 16).strokeBorder(store.selectedIDs.contains(card.id) ? Color.accentColor.opacity(0.8) : Color.primary.opacity(hovered ? 0.18 : 0.07), lineWidth: store.selectedIDs.contains(card.id) ? 1.5 : 1))
            .contentShape(RoundedRectangle(cornerRadius: 16))
            .onHover { hovered = $0 }
            .gesture(TapGesture(count: 2).onEnded { store.selection = card.id; store.beginEdit(card) }
                .exclusively(before: TapGesture(count: 1).onEnded { store.selectCard(card.id, modifiers: NSEvent.modifierFlags) }))
            .onDrag { store.dragProvider(card) }
            .contextMenu {
                if store.selectedIDs.contains(card.id), store.selectedIDs.count > 1 {
                    Button("Copy \(store.selectedIDs.count) Images", action: store.copySelection)
                        .disabled(!store.selectedCards.allSatisfy(store.canExport))
                    Button("Export \(store.selectedIDs.count) Images…", action: store.exportSelection)
                        .disabled(!store.selectedCards.allSatisfy(store.canExport))
                    Menu("Assign Label to Selection") {
                        Button("Unlabeled") { store.assignSelected(to: nil) }
                        ForEach(store.sections) { section in Button(section.name) { store.assignSelected(to: section.id) } }
                    }
                    Divider()
                }
                Button("Edit") { store.beginEdit(card) }
                Button("Duplicate") { store.beginEdit(card, duplicate: true) }
                Menu("Assign Label") {
                    Button("Unlabeled") { store.assign(card, to: nil) }
                    ForEach(store.sections) { section in Button(section.name) { store.assign(card, to: section.id) } }
                }
                Divider()
                Button("Copy \(store.exportFormat.title)") { store.copyImage(card) }.disabled(!store.canExport(card))
                Button("Export \(store.exportFormat.title)…") { store.export(card) }.disabled(!store.canExport(card))
                if card.origin == "custom" { Divider(); Button("Delete Equation", role: .destructive) { store.delete(card) } }
            }
            .accessibilityElement(children: .combine).accessibilityLabel("\(card.name), \(card.kind), \(card.status)")
            .accessibilityAddTraits(.isButton).accessibilityValue(store.selectedIDs.contains(card.id) ? "Selected" : "")
            .accessibilityAction { store.selectCard(card.id, modifiers: NSEvent.modifierFlags) }
            .accessibilityAction(named: Text("Edit")) { store.beginEdit(card) }
    }
}

struct InspectorView: View {
    @Environment(\.appTheme) private var theme
    @EnvironmentObject private var store: LibraryStore
    @EnvironmentObject private var preferences: WindowPreferences
    var body: some View {
        ScrollView {
            if let card = store.selected {
                VStack(alignment: .leading, spacing: 20) {
                    HStack {
                        Text(card.origin.capitalized).font(.system(size: 11, weight: .medium)).foregroundStyle(.secondary)
                        Spacer()
                        Button { store.showInspector = false } label: { Image(systemName: "xmark").font(.system(size: 10)) }.buttonStyle(.plain).foregroundStyle(.tertiary).help("Close Inspector")
                    }
                    Text(card.name).font(.system(size: 20, weight: .semibold)).textSelection(.enabled)
                    EquationImage(card: card).frame(maxWidth: .infinity).frame(height: 135).padding(14)
                        .background(Color(nsColor: theme.color(.surfaceBackground)).opacity(preferences.opacity), in: RoundedRectangle(cornerRadius: 14))
                        .onDrag { store.dragProvider(card) }
                    HStack(spacing: 10) {
                        Button { store.beginEdit(card) } label: { Image(systemName: "pencil") }.help(card.kind == "figure" ? "Edit Figure" : "Edit Equation").accessibilityLabel(card.kind == "figure" ? "Edit Figure" : "Edit Equation")
                        Button { store.beginEdit(card, duplicate: true) } label: { Image(systemName: "square.on.square") }.help("Duplicate").accessibilityLabel("Duplicate")
                        Spacer()
                        Button { store.copyImage(card) } label: { Image(systemName: "doc.on.clipboard") }.disabled(!store.canExport(card)).help("Copy \(store.exportFormat.title)").accessibilityLabel("Copy \(store.exportFormat.title)")
                        Button { store.export(card) } label: { Image(systemName: "square.and.arrow.up") }.disabled(!store.canExport(card)).help("Export \(store.exportFormat.title)").accessibilityLabel("Export \(store.exportFormat.title)")
                    }.controlSize(.large)
                    ExportFormatPicker()
                    LocalExportSizePicker(value: Binding(get: { card.exportFontSize }, set: { store.setExportFontSize($0, for: card) }), isFigure: card.kind == "figure")
                    Divider()
                    if let path = store.outlineTitle(for: card) {
                        Text(path).font(.system(size: 11)).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                    }
                    LabeledContent("Label") {
                        Picker("Label", selection: Binding(get: { store.sectionAssignments[card.id] ?? "" }, set: { store.assign(card, to: $0.isEmpty ? nil : $0) })) {
                            Text("Unlabeled").tag("")
                            ForEach(store.sections) { Text($0.name).tag($0.id) }
                        }.labelsHidden()
                    }.font(.system(size: 11))
                    VStack(alignment: .leading, spacing: 10) {
                        Text(store.format.displayName).font(.system(size: 11, weight: .medium)).foregroundStyle(.secondary)
                        Text(card.tex).font(.system(size: 11, design: .monospaced)).textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading).padding(14)
                            .background(.primary.opacity(0.035), in: RoundedRectangle(cornerRadius: 12))
                    }
                    if let context = card.context {
                        DisclosureGroup("Source") {
                            VStack(alignment: .leading, spacing: 10) {
                                Text(card.location).foregroundStyle(.secondary).textSelection(.enabled)
                                ForEach(Array(context.enumerated()), id: \.offset) { index, line in
                                    HStack(alignment: .top, spacing: 10) {
                                        Text("\((card.contextStart ?? 1) + index)").foregroundStyle(.tertiary).frame(width: 28, alignment: .trailing)
                                        Text(line).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
                                    }.font(.system(size: 10, design: .monospaced))
                                }
                            }.padding(.top, 10)
                        }.font(.system(size: 11)).foregroundStyle(.secondary)
                    }
                    if card.status == "failed" {
                        DisclosureGroup("Render log") { Text(card.error ?? "Unknown error").font(.system(size: 10, design: .monospaced)).textSelection(.enabled) }.foregroundStyle(.orange)
                    }
                    if card.origin == "custom" {
                        Button("Delete Equation", role: .destructive) { store.delete(card) }.buttonStyle(.plain).font(.system(size: 11)).foregroundStyle(.secondary)
                    }
                }.padding(24)
            } else {
                VStack(spacing: 12) {
                    Image(systemName: "cursorarrow.click").font(.system(size: 24, weight: .light))
                    Text("Select an item").font(.system(size: 12))
                }.foregroundStyle(.tertiary).frame(maxWidth: .infinity).padding(.top, 100)
            }
        }.background(.background.opacity(0.65 * preferences.opacity))
    }
}

struct EditorView: View {
    @Environment(\.appTheme) private var theme
    @EnvironmentObject private var store: LibraryStore
    @Environment(\.dismiss) private var dismiss
    @State var draft: Equation
    @State private var discard = false
    let original: Equation
    init(initial: Equation) { _draft = State(initialValue: initial); original = initial }
    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            HStack {
                Text(draft.kind == "figure" ? "Edit Figure" : "Edit Equation").font(.system(size: 22, weight: .semibold))
                Spacer()
                Picker("Type", selection: $draft.kind) { Text("Inline").tag("inline"); Text("Block").tag("block"); Text("Figure").tag("figure") }.pickerStyle(.segmented).labelsHidden().frame(width: 225)
            }
            TextField(draft.kind == "figure" ? "Figure name" : "Equation name", text: $draft.name).textFieldStyle(.plain).font(.system(size: 15, weight: .medium)).padding(12)
                .background(.primary.opacity(0.035), in: RoundedRectangle(cornerRadius: 10))
            HSplitView {
                VStack(alignment: .leading, spacing: 10) {
                    AssistedSourceEditor(text: $draft.tex, label: store.format.displayName, format: store.format, context: draft.kind == "figure" ? .figure : .math, projectCompletions: store.projectCompletions)
                }.frame(minWidth: 320, minHeight: 250).padding(.trailing, 10)
                VStack(spacing: 10) {
                    HStack {
                        Text("Preview").font(.system(size: 11, weight: .medium)); Spacer()
                        ExportFormatPicker().frame(width: 95)
                        if store.previewBusy { ProgressView().controlSize(.small) }
                    }.foregroundStyle(.secondary)
                    LocalExportSizePicker(value: $draft.exportFontSize, isFigure: draft.kind == "figure")
                    VStack {
                        if let card = store.preview, card.tex == draft.tex, card.kind == draft.kind {
                            EquationImage(card: card).frame(maxWidth: .infinity, maxHeight: .infinity)
                                .contentShape(Rectangle())
                                .onDrag {
                                    var image = card; image.name = draft.name; image.exportFontSize = draft.exportFontSize
                                    return store.dragProvider(image)
                                }
                                .help("Drag \(store.exportFormat.title)")
                            if let error = card.error { ScrollView { Text(error).font(.system(size: 10, design: .monospaced)).textSelection(.enabled) }.frame(maxHeight: 105) }
                        } else {
                            Spacer(); Image(systemName: "function").font(.system(size: 30, weight: .light)).foregroundStyle(.tertiary); Spacer()
                        }
                    }.padding(18).frame(maxWidth: .infinity).background(Color(nsColor: theme.color(.surfaceBackground)), in: RoundedRectangle(cornerRadius: 12))
                }.frame(minWidth: 280).padding(.leading, 10)
            }
            HStack {
                Spacer()
                Button("Cancel") { if draft != original { discard = true } else { close() } }.keyboardShortcut(.cancelAction)
                Button("Save") { store.saveDraft(draft) }.buttonStyle(.borderedProminent).keyboardShortcut(.return, modifiers: .command)
                    .disabled(draft.tex.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || store.previewBusy)
            }
        }.padding(28).frame(width: 820, height: 490)
            .task(id: draft.tex + "\u{0}" + draft.kind) {
                do { try await Task.sleep(nanoseconds: 700_000_000); store.renderPreview(draft) } catch {}
            }
            .interactiveDismissDisabled(draft != original)
            .onDisappear { store.cancelPreview() }
            .onReceive(NotificationCenter.default.publisher(for: .saveEquation)) { _ in
                if !draft.tex.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && !store.previewBusy { store.saveDraft(draft) }
            }
            .confirmationDialog("Discard changes?", isPresented: $discard) {
                Button("Discard", role: .destructive, action: close); Button("Keep Editing", role: .cancel) {}
            }
    }
    private func close() { store.cancelPreview(); dismiss() }
}

struct ExportFormatPicker: View {
    @EnvironmentObject private var store: LibraryStore
    var body: some View {
        Picker("Export Format", selection: $store.exportFormat) {
            ForEach(ImageExportFormat.allCases) { Text($0.title).tag($0) }
        }.labelsHidden().help("Export format")
            .onChange(of: store.exportFormat) { _, _ in store.saveNow(); store.prepareExportResources() }
    }
}

struct LocalExportSizePicker: View {
    @EnvironmentObject private var store: LibraryStore
    @Binding var value: Double?
    var isFigure: Bool
    private let sizes = [10, 12, 14, 16, 18, 20, 24, 28, 32, 36, 40, 48, 60, 72, 96]
    var body: some View {
        HStack {
            Text("Export Font Size").font(.system(size: 11)).foregroundStyle(.secondary)
            Spacer(minLength: 8)
            Picker("Item Export Font Size", selection: $value) {
                Text(isFigure ? "Default · Original" : store.exportFontSize == 0 ? "Default · Original" : "Default · \(Int(store.exportFontSize)) pt").tag(Double?.none)
                Text("Original").tag(Double?.some(0))
                ForEach(sizes, id: \.self) { Text("\($0) pt").tag(Double?.some(Double($0))) }
            }.labelsHidden().frame(width: 145)
                .help(isFigure ? "Scale this figure, including its labels, for export" : "Font size for this equation’s exported image")
        }
    }
}
