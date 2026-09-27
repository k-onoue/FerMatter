import XCTest
import AppKit
@testable import SlideMath

final class SettingsFileTests: XCTestCase {
    private var directory: URL!
    private var defaults: UserDefaults!
    private var suite: String!
    override func setUpWithError() throws {
        suite = "SettingsTests-\(UUID())"
        defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        directory = FileManager.default.temporaryDirectory.appendingPathComponent(suite)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }
    override func tearDownWithError() throws {
        defaults.removePersistentDomain(forName: suite)
        try FileManager.default.removeItem(at: directory)
    }

    @MainActor func testLegacyShortcutsMigrateOnceAndFileTakesPrecedence() throws {
        let oldBinding = ShortcutBinding("k", shift: true)
        defaults.set(try JSONEncoder().encode(["togglePDF": oldBinding]), forKey: "keyboardShortcuts")
        let preferences = ShortcutPreferences(defaults: defaults, directory: directory)
        XCTAssertNil(preferences.fileError)
        XCTAssertEqual(preferences.binding(.togglePDF), oldBinding)
        let file = SettingsFile<ShortcutDocument>(directory: directory, name: "keybindings.json")
        XCTAssertEqual(try file.read(preferences.fileURL).bindings["togglePDF"], oldBinding)
        XCTAssertNil(preferences.set(ShortcutBinding("j", option: true), for: .togglePDF))
        XCTAssertEqual(ShortcutPreferences(defaults: defaults, directory: directory).binding(.togglePDF), ShortcutBinding("j", option: true))
        preferences.reset()
        XCTAssertEqual(ShortcutPreferences(defaults: defaults, directory: directory).binding(.togglePDF), AppCommand.togglePDF.defaultShortcut)
        XCTAssertNotNil(defaults.data(forKey: "keyboardShortcuts"))
    }

    @MainActor func testInvalidReloadImportAndRestartKeepLastValidShortcuts() throws {
        let preferences = ShortcutPreferences(defaults: defaults, directory: directory)
        let binding = ShortcutBinding("k", option: true)
        XCTAssertNil(preferences.set(binding, for: .togglePDF))
        let invalid = Data("{ invalid JSON".utf8)
        try invalid.write(to: preferences.fileURL)
        preferences.reload()
        XCTAssertNotNil(preferences.fileError)
        XCTAssertEqual(preferences.binding(.togglePDF), binding)
        XCTAssertNotNil(preferences.set(ShortcutBinding("j", option: true), for: .togglePDF))
        XCTAssertEqual(try Data(contentsOf: preferences.fileURL), invalid)
        let reopened = ShortcutPreferences(defaults: defaults, directory: directory)
        XCTAssertEqual(reopened.binding(.togglePDF), binding)
        XCTAssertNotNil(reopened.fileError)
        reopened.importFile(preferences.fileURL)
        XCTAssertEqual(reopened.binding(.togglePDF), binding)
        XCTAssertEqual(try Data(contentsOf: preferences.fileURL), invalid)
    }

    @MainActor func testShortcutImportValidatesWholeAssignmentSoKeysCanBeSwapped() throws {
        let preferences = ShortcutPreferences(defaults: defaults, directory: directory)
        let imported = directory.appendingPathComponent("swap.json")
        let document = ShortcutDocument(bindings: ["open": AppCommand.projects.defaultShortcut, "projects": AppCommand.open.defaultShortcut],
            extensions: ["community.example": .object(["enabled": .bool(false), "values": .array([.null, .number(2), .string("local")])])])
        try JSONEncoder().encode(document).write(to: imported)
        preferences.importFile(imported)
        XCTAssertNil(preferences.fileError)
        XCTAssertEqual(preferences.binding(.open), AppCommand.projects.defaultShortcut)
        XCTAssertEqual(preferences.binding(.projects), AppCommand.open.defaultShortcut)
        XCTAssertNil(preferences.set(ShortcutBinding("k", option: true), for: .togglePDF))
        let exported = directory.appendingPathComponent("export.json")
        preferences.exportFile(exported)
        let value = try SettingsFile<ShortcutDocument>(directory: directory, name: "keybindings.json").read(exported)
        XCTAssertEqual(value.extensions, document.extensions)
    }

    func testShortcutSchemaRejectsConflictsReservedKeysUnknownCommandsAndVersions() throws {
        try ShortcutDocument().validate()
        for bindings in [["open": ShortcutBinding("n")], ["open": ShortcutBinding("q")],
                         ["open": ShortcutBinding("x", command: false)], ["open": ShortcutBinding("ab")],
                         ["community.run": ShortcutBinding("k")]] {
            XCTAssertThrowsError(try ShortcutDocument(bindings: bindings).validate())
        }
        XCTAssertThrowsError(try ShortcutDocument(schemaVersion: 2).validate())
        XCTAssertThrowsError(try ShortcutDocument(kind: "mathlidelux.theme").validate())
        try ShortcutDocument(bindings: ["open": .unassigned, "projects": .unassigned]).validate()
    }

    @MainActor func testExternalEditsAreNotOverwrittenByRecorderUntilReload() throws {
        let preferences = ShortcutPreferences(defaults: defaults, directory: directory)
        let document = ShortcutDocument(bindings: ["togglePDF": ShortcutBinding("k", option: true)])
        let external = try JSONEncoder().encode(document)
        try external.write(to: preferences.fileURL)
        XCTAssertNotNil(preferences.set(.unassigned, for: .open))
        XCTAssertEqual(try Data(contentsOf: preferences.fileURL), external)
        preferences.reload()
        XCTAssertNil(preferences.fileError)
        XCTAssertNil(preferences.set(.unassigned, for: .open))
        XCTAssertEqual(preferences.binding(.togglePDF), document.binding(.togglePDF))
    }

    @MainActor func testThemeImportReloadExportAndRecoveryPreserveIDAndExtensions() throws {
        let preferences = ThemePreferences(directory: directory)
        var custom = ThemeDocument.standard
        custom.id = "community.example.ocean"; custom.name = "Ocean"
        custom.light["accent"] = "#007766"; custom.dark["command"] = "#99CCEE"
        custom.extensions = ["community.example": .object(["license": .string("MIT")])]
        let imported = directory.appendingPathComponent("ocean.json")
        try JSONEncoder().encode(custom).write(to: imported)
        preferences.importFile(imported)
        XCTAssertNil(preferences.fileError); XCTAssertEqual(preferences.document, custom)
        XCTAssertEqual(ThemePreferences(directory: directory).document, custom)
        let exported = directory.appendingPathComponent("export.json")
        preferences.exportFile(exported)
        XCTAssertEqual(try JSONDecoder().decode(ThemeDocument.self, from: Data(contentsOf: exported)), custom)
        var invalid = custom; invalid.dark["text"] = "#00000000"
        let invalidBytes = try JSONEncoder().encode(invalid)
        try invalidBytes.write(to: preferences.fileURL)
        preferences.reload()
        XCTAssertNotNil(preferences.fileError); XCTAssertEqual(preferences.document, custom)
        XCTAssertEqual(ThemePreferences(directory: directory).document, custom)
        preferences.importFile(preferences.fileURL)
        XCTAssertNotNil(preferences.fileError); XCTAssertEqual(preferences.document, custom)
        XCTAssertEqual(try Data(contentsOf: preferences.fileURL), invalidBytes)
        preferences.reset()
        XCTAssertEqual(ThemePreferences(directory: directory).document, .standard)
    }

    func testThemeSchemaRejectsInvalidIDsColorsTokensAndVersions() throws {
        try ThemeDocument.standard.validate()
        var document = ThemeDocument.standard; document.schemaVersion = 2
        XCTAssertThrowsError(try document.validate())
        document = .standard; document.id = "../unsafe"
        XCTAssertThrowsError(try document.validate())
        document = .standard; document.name = "  "
        XCTAssertThrowsError(try document.validate())
        for color in ["red", "#abc", "#12345678", "#GGGGGG", "javascript:run()"] {
            document = .standard; document.light["text"] = color
            XCTAssertThrowsError(try document.validate())
        }
        document = .standard; document.dark["script"] = "system"
        XCTAssertThrowsError(try document.validate())
    }

    @MainActor func testThemeColorsReachSyntaxAndStayOpaqueInBothAppearances() throws {
        _ = NSApplication.shared
        var theme = ThemeDocument.standard
        theme.light["command"] = "#123456"; theme.dark["command"] = "#AABBCC"
        theme.light["text"] = "#334455"
        let view = AssistedTextView(); view.theme = theme; view.string = "\\alpha x"; view.highlight()
        let command = try XCTUnwrap(view.textStorage?.attribute(.foregroundColor, at: 0, effectiveRange: nil) as? NSColor)
        for (appearance, expected) in [(NSAppearance.Name.aqua, 18.0 / 255), (.darkAqua, 170.0 / 255)] {
            try XCTUnwrap(NSAppearance(named: appearance)).performAsCurrentDrawingAppearance {
                let color = command.usingColorSpace(.sRGB)!
                XCTAssertEqual(color.redComponent, expected, accuracy: 0.001)
                XCTAssertEqual(color.alphaComponent, 1)
            }
        }
        let text = try XCTUnwrap(view.typingAttributes[.foregroundColor] as? NSColor)
        NSAppearance(named: .aqua)!.performAsCurrentDrawingAppearance {
            XCTAssertEqual(text.usingColorSpace(.sRGB)!.redComponent, 51.0 / 255, accuracy: 0.001)
        }
    }

    @MainActor func testOversizedOrFailedWritesDoNotReplaceActiveSettings() throws {
        let preferences = ThemePreferences(directory: directory)
        let invalid = directory.appendingPathComponent("large.json")
        try Data(repeating: 32, count: SettingsFile<ThemeDocument>.maxBytes + 1).write(to: invalid)
        let original = try Data(contentsOf: preferences.fileURL)
        preferences.importFile(invalid)
        XCTAssertNotNil(preferences.fileError)
        XCTAssertEqual(try Data(contentsOf: preferences.fileURL), original)
        let blocked = directory.appendingPathComponent("blocked")
        try Data().write(to: blocked)
        let unwritable = ThemePreferences(directory: blocked)
        XCTAssertNotNil(unwritable.fileError)
        var custom = ThemeDocument.standard; custom.name = "Unsaved"
        try JSONEncoder().encode(custom).write(to: invalid)
        unwritable.importFile(invalid)
        XCTAssertEqual(unwritable.document, .standard)
        XCTAssertNotNil(unwritable.fileError)
    }
}
