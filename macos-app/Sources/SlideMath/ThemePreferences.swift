import AppKit
import SwiftUI

enum ThemeToken: String, CaseIterable {
    case accent, windowBackground, surfaceBackground, text, command, delimiter, number, comment
    var systemColor: NSColor {
        switch self {
        case .accent: .systemBlue
        case .windowBackground: .windowBackgroundColor
        case .surfaceBackground: .textBackgroundColor
        case .text: .textColor
        case .command: .systemPurple
        case .delimiter: .systemTeal
        case .number: .systemOrange
        case .comment: .secondaryLabelColor
        }
    }
}

struct ThemeDocument: SettingsDocument, Equatable {
    var schemaVersion = 1
    var kind = "mathlidelux.theme"
    var id: String
    var name: String
    var light: [String: String]
    var dark: [String: String]
    var extensions: [String: SettingsValue]? = nil

    static let standard = ThemeDocument(id: "org.mathlidelux.default", name: "Default",
        light: Dictionary(uniqueKeysWithValues: ThemeToken.allCases.map { ($0.rawValue, "system") }),
        dark: Dictionary(uniqueKeysWithValues: ThemeToken.allCases.map { ($0.rawValue, "system") }))

    func validate() throws {
        guard schemaVersion == 1, kind == "mathlidelux.theme" else { throw SettingsFileError.invalid("Unsupported theme format or version.") }
        guard id.range(of: #"^[a-z0-9]+(?:[.-][a-z0-9]+)+$"#, options: .regularExpression) != nil, id.count <= 120 else {
            throw SettingsFileError.invalid("Use a stable theme ID such as community.author.theme.")
        }
        guard !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, name.count <= 80 else {
            throw SettingsFileError.invalid("Theme names must contain 1–80 characters.")
        }
        for (variant, colors) in [("light", light), ("dark", dark)] {
            for (token, value) in colors {
                guard ThemeToken(rawValue: token) != nil else { throw SettingsFileError.invalid("Unknown theme color: \(token).") }
                guard value == "system" || Self.hexColor(value) != nil else {
                    throw SettingsFileError.invalid("\(variant).\(token) must be system or #RRGGBB.")
                }
            }
        }
    }

    func color(_ token: ThemeToken) -> NSColor {
        let light = light[token.rawValue].flatMap(Self.hexColor), dark = dark[token.rawValue].flatMap(Self.hexColor)
        guard light != nil || dark != nil else { return token.systemColor }
        return NSColor(name: nil) { appearance in
            (appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua ? dark : light) ?? token.systemColor
        }
    }

    private static func hexColor(_ value: String) -> NSColor? {
        guard value.range(of: #"^#[0-9A-Fa-f]{6}$"#, options: .regularExpression) != nil,
              let rgb = UInt32(value.dropFirst(), radix: 16) else { return nil }
        return NSColor(srgbRed: CGFloat((rgb >> 16) & 255) / 255, green: CGFloat((rgb >> 8) & 255) / 255,
                       blue: CGFloat(rgb & 255) / 255, alpha: 1)
    }
}

private struct AppThemeKey: EnvironmentKey { static let defaultValue = ThemeDocument.standard }
extension EnvironmentValues {
    var appTheme: ThemeDocument {
        get { self[AppThemeKey.self] }
        set { self[AppThemeKey.self] = newValue }
    }
}

@MainActor final class ThemePreferences: FileSettings {
    @Published private(set) var document: ThemeDocument
    @Published private(set) var fileError: String?
    private let file: SettingsFile<ThemeDocument>
    var fileURL: URL { file.url }
    init(directory: URL = AppIdentity.settingsDirectory) {
        file = SettingsFile(directory: directory, name: "theme.json")
        (document, fileError) = file.initial(.standard)
    }
    func reload() {
        do { document = try file.load(); fileError = nil }
        catch { fileError = error.localizedDescription }
    }
    func reset() { replace(.standard) }
    func importFile(_ url: URL) {
        do { replace(try file.read(url)) }
        catch { fileError = error.localizedDescription }
    }
    func exportFile(_ url: URL) {
        do { try file.export(document, to: url); fileError = nil }
        catch { fileError = error.localizedDescription }
    }
    private func replace(_ value: ThemeDocument) {
        do { try file.save(value, replacing: true); document = value; fileError = nil }
        catch { fileError = error.localizedDescription }
    }
}

struct ThemeSettingsView: View {
    @ObservedObject var themes: ThemePreferences
    @ObservedObject var window: WindowPreferences
    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            HStack {
                Text("Appearance").font(.title2.weight(.semibold))
                Spacer()
                Button("Restore Defaults", action: themes.reset)
            }
            Picker("Appearance", selection: $window.appearance) {
                Text("System").tag("system"); Text("Light").tag("light"); Text("Dark").tag("dark")
            }.pickerStyle(.segmented)
            Divider()
            HStack {
                Text(themes.document.name).font(.headline)
                Spacer()
                ForEach([ThemeToken.accent, .command, .delimiter, .number, .comment], id: \.rawValue) { token in
                    Circle().fill(Color(nsColor: themes.document.color(token))).frame(width: 18, height: 18).help(token.rawValue)
                }
            }
            Text("Edit the theme file, then Reload to apply.").font(.caption).foregroundStyle(.secondary)
            SettingsFileControls(preferences: themes)
            if let error = themes.fileError { Text(error).font(.caption).foregroundStyle(.orange).textSelection(.enabled) }
            Spacer()
        }.padding(24).frame(width: 640, height: 340)
    }
}
