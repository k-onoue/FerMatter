import Foundation

enum AppIdentity {
    static let name = "Mathlidelux"

    // Keep the original location: saved projects contain absolute paths into it.
    // The public app name can change without moving sources, drafts or templates.
    static var applicationSupport: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Slide Math", isDirectory: true)
    }
}
