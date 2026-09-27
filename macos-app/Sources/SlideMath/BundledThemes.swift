import Foundation

enum BundledThemes {
    static let filenames = [
        "atom-one", "ayu", "catppuccin", "everforest", "github", "gruvbox-material",
        "horizon", "min", "night-owl", "noctis", "rose-pine", "solarized",
        "tokyo-night", "vitesse", "vscode-modern"
    ]

    private static var resources: Bundle {
        // A packaged .app keeps resources under Contents/Resources; SwiftPM's
        // generated accessor otherwise looks beside the executable or in .build.
        if let url = Bundle.main.url(forResource: "Mathlidelux_SlideMath", withExtension: "bundle"),
           let bundle = Bundle(url: url) { return bundle }
        return .module
    }

    static func load() throws -> [ThemeDocument] {
        try [.standard] + filenames.map { name in
            guard let url = resources.url(forResource: name + ".theme", withExtension: "json", subdirectory: "Themes") else {
                throw SettingsFileError.invalid("Missing bundled theme: \(name).")
            }
            let document = try JSONDecoder().decode(ThemeDocument.self, from: Data(contentsOf: url))
            try document.validate()
            return document
        }
    }
}
