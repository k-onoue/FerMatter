import AppKit
import AVFoundation
import ImageIO

/// Read-only fallback for native full-screen Spaces, where WindowServer may
/// provide a black backdrop. Never choose a bundled/default or unrelated image.
enum WallpaperSelection: Equatable {
    case image(URL)
    case aerial(String)
    case unavailable
}

struct DesktopWallpaper {
    let home: URL
    static let storePath = "Library/Application Support/com.apple.wallpaper/Store/Index.plist"

    func selection(displayID: String, workspaceURL: URL?) -> WallpaperSelection {
        let data = try? Data(contentsOf: home.appendingPathComponent(Self.storePath))
        let store = data.flatMap { try? PropertyListSerialization.propertyList(from: $0, format: nil) } as? [String: Any]
        return Self.selection(store: store ?? [:], displayID: displayID, workspaceURL: workspaceURL)
    }

    static func selection(store: [String: Any], displayID: String, workspaceURL: URL?) -> WallpaperSelection {
        // A populated global setting is the "Show on all Spaces" selection.
        // SystemDefault is deliberately ignored: it need not be in active use.
        if let choice = choice(in: store["AllSpacesAndDisplays"]) { return choice }
        if let displays = store["Displays"] as? [String: Any],
           let entry = displays.first(where: { $0.key.caseInsensitiveCompare(displayID) == .orderedSame })?.value,
           let choice = choice(in: entry) { return choice }
        // The public API correctly resolves ordinary per-Space image files.
        // For modern providers it can return DefaultDesktop.heic instead.
        if let url = workspaceURL, url.isFileURL, url.lastPathComponent != "DefaultDesktop.heic" {
            return .image(url)
        }
        // There is no public API for the current Space UUID. Only use per-Space
        // provider settings when they agree; never guess from dictionary order.
        if let spaces = store["Spaces"] as? [String: Any], !spaces.isEmpty {
            let choices = spaces.values.map { space -> WallpaperSelection? in
                guard let space = space as? [String: Any] else { return nil }
                if let displays = space["Displays"] as? [String: Any],
                   let entry = displays.first(where: { $0.key.caseInsensitiveCompare(displayID) == .orderedSame })?.value {
                    return choice(in: entry)
                }
                return choice(in: space["Default"]) ?? choice(in: space)
            }
            if let first = choices.first ?? nil, choices.allSatisfy({ $0 == first }) { return first }
        }
        return .unavailable
    }

    private static func choice(in value: Any?) -> WallpaperSelection? {
        guard let entry = value as? [String: Any] else { return nil }
        let mode = entry["Type"] as? String
        let desktop = (mode == "linked" ? entry["Linked"] : entry["Desktop"]) as? [String: Any]
        guard let content = desktop?["Content"] as? [String: Any],
              let choices = content["Choices"] as? [[String: Any]], !choices.isEmpty else { return nil }
        guard choices.count == 1,
              content["Shuffle"] == nil || content["Shuffle"] as? String == "$null" else { return .unavailable }
        let choice = choices[0]
        let configuration = (choice["Configuration"] as? Data)
            .flatMap { try? PropertyListSerialization.propertyList(from: $0, format: nil) } as? [String: Any] ?? [:]
        switch choice["Provider"] as? String {
        case "com.apple.wallpaper.choice.aerials":
            guard let id = configuration["assetID"] as? String, UUID(uuidString: id) != nil else { return .unavailable }
            return .aerial(id)
        case "com.apple.wallpaper.choice.image":
            if let url = fileURL(configuration["url"]) { return .image(url) }
            if let files = choice["Files"] as? [Any], files.count == 1, let url = fileURL(files[0]) { return .image(url) }
            return .unavailable
        default: return .unavailable
        }
    }

    private static func fileURL(_ value: Any?) -> URL? {
        let string = value as? String ?? (value as? [String: Any])?["relative"] as? String
        guard let string, let url = URL(string: string), url.isFileURL else { return nil }
        return url
    }

    func file(for selection: WallpaperSelection) -> URL? {
        switch selection {
        case .image(let url): return url
        case .aerial(let id):
            guard UUID(uuidString: id) != nil else { return nil }
            let paths = [home.appendingPathComponent("Library/Application Support/com.apple.wallpaper/aerials/videos/\(id).mov"),
                         URL(fileURLWithPath: "/Library/Application Support/com.apple.idleassetsd/Customer/4KSDR240FPS/\(id).mov")]
            return paths.first { FileManager.default.isReadableFile(atPath: $0.path) }
        case .unavailable: return nil
        }
    }
}

actor WallpaperImageLoader {
    private var cachedURL: URL?
    private var cachedDate: Date?
    private var cachedImage: CGImage?

    func image(home: URL, displayID: String, workspaceURL: URL?) async -> CGImage? {
        let resolver = DesktopWallpaper(home: home)
        let selection = resolver.selection(displayID: displayID, workspaceURL: workspaceURL)
        guard let url = resolver.file(for: selection) else {
            cachedURL = nil; cachedImage = nil; return nil
        }
        let modified = try? url.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate
        if url == cachedURL, modified == cachedDate, let cachedImage { return cachedImage }
        let image: CGImage?
        switch selection {
        case .aerial:
            let generator = AVAssetImageGenerator(asset: AVURLAsset(url: url))
            generator.appliesPreferredTrackTransform = true
            generator.maximumSize = CGSize(width: 4096, height: 4096)
            image = try? await generator.image(at: .zero).image
        case .image:
            let source = CGImageSourceCreateWithURL(url as CFURL, nil)
            image = source.flatMap { CGImageSourceCreateThumbnailAtIndex($0, 0, [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceThumbnailMaxPixelSize: 4096
            ] as CFDictionary) }
        case .unavailable: image = nil
        }
        // Failed reads may recover after macOS finishes downloading the asset.
        cachedURL = url; cachedDate = modified; cachedImage = image
        return image
    }
}
