import SwiftUI

enum SourceSaveMode: String, CaseIterable, Identifiable {
    case afterDelay, manual
    var id: String { rawValue }
    var title: String { self == .afterDelay ? "After Delay" : "Manual" }
}

@MainActor final class EditorPreferences: ObservableObject {
    @Published var saveMode: SourceSaveMode { didSet { defaults.set(saveMode.rawValue, forKey: "sourceSaveMode") } }
    @Published var delay: Double { didSet { defaults.set(delay, forKey: "sourceSaveDelay") } }
    private let defaults: UserDefaults
    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        saveMode = defaults.string(forKey: "sourceSaveMode").flatMap(SourceSaveMode.init) ?? .afterDelay
        delay = max(0.3, defaults.object(forKey: "sourceSaveDelay") as? Double ?? 0.8)
    }
}

struct EditorSettingsView: View {
    @ObservedObject var preferences: EditorPreferences
    var body: some View {
        Form {
            Section("Source Files") {
                Picker("Auto Save", selection: $preferences.saveMode) {
                    ForEach(SourceSaveMode.allCases) { Text($0.title).tag($0) }
                }
                if preferences.saveMode == .afterDelay {
                    Picker("Delay", selection: $preferences.delay) {
                        Text("500 ms").tag(0.5); Text("800 ms").tag(0.8)
                        Text("1 second").tag(1.0); Text("2 seconds").tag(2.0)
                    }
                }
                Text("⌘S saves immediately in either mode.").font(.caption).foregroundStyle(.secondary)
            }
        }.formStyle(.grouped).frame(width: 540, height: 320)
    }
}
