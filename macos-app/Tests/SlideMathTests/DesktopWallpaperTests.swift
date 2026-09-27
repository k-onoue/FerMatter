import XCTest
@testable import SlideMath

final class DesktopWallpaperTests: XCTestCase {
    private let first = "52ACB9B8-75FC-4516-BC60-4550CFF3B661"
    private let second = "8002C4C8-C611-4894-A068-3D3A3C03472A"
    private let defaultURL = URL(fileURLWithPath: "/System/Library/CoreServices/DefaultDesktop.heic")

    private func entry(_ provider: String = "aerials", config: [String: Any], linked: Bool = true) throws -> [String: Any] {
        let choice: [String: Any] = ["Provider": "com.apple.wallpaper.choice.\(provider)",
                                   "Configuration": try PropertyListSerialization.data(fromPropertyList: config, format: .binary, options: 0)]
        return ["Type": linked ? "linked" : "desktop", linked ? "Linked" : "Desktop": ["Content": ["Choices": [choice], "Shuffle": "$null"]]]
    }

    func testActualAerialOverridesMisleadingPublicDefault() throws {
        let store = ["AllSpacesAndDisplays": try entry(config: ["assetID": first]),
                     "SystemDefault": try entry(config: ["assetID": second])]
        XCTAssertEqual(DesktopWallpaper.selection(store: store, displayID: "external", workspaceURL: defaultURL), .aerial(first))
    }

    func testExternalDisplayIsSelectedByIdentityNotOrderOrMainScreen() throws {
        let displays = ["INTERNAL": try entry(config: ["assetID": first]), "EXTERNAL": try entry(config: ["assetID": second])]
        let store: [String: Any] = ["AllSpacesAndDisplays": "$null", "Displays": displays]
        XCTAssertEqual(DesktopWallpaper.selection(store: store, displayID: "external", workspaceURL: defaultURL), .aerial(second))
        XCTAssertEqual(DesktopWallpaper.selection(store: store, displayID: "internal", workspaceURL: defaultURL), .aerial(first))
        XCTAssertEqual(DesktopWallpaper.selection(store: store, displayID: "disconnected", workspaceURL: defaultURL), .unavailable)
    }

    func testStaticPhotoAndSeparateScreensaver() throws {
        let url = URL(fileURLWithPath: "/tmp/WallpaperFixtures/My Photo.png")
        var desktop = try entry("image", config: ["type": "imageFile", "url": ["relative": url.absoluteString]], linked: false)
        desktop["Idle"] = try entry(config: ["assetID": first])["Linked"]
        XCTAssertEqual(DesktopWallpaper.selection(store: ["AllSpacesAndDisplays": desktop], displayID: "display", workspaceURL: defaultURL), .image(url))
    }

    func testGlobalSelectionTakesPrecedenceOverDormantDisplayEntries() throws {
        let store: [String: Any] = ["AllSpacesAndDisplays": try entry(config: ["assetID": first]),
                                   "Displays": ["display": try entry(config: ["assetID": second])]]
        XCTAssertEqual(DesktopWallpaper.selection(store: store, displayID: "display", workspaceURL: nil), .aerial(first))
    }

    func testAmbiguousSpacesAndUnknownProvidersNeverSubstituteDefault() throws {
        let spaces = ["one": try entry(config: ["assetID": first]), "two": try entry(config: ["assetID": second])]
        XCTAssertEqual(DesktopWallpaper.selection(store: ["Spaces": spaces], displayID: "display", workspaceURL: defaultURL), .unavailable)
        XCTAssertEqual(DesktopWallpaper.selection(store: ["SystemDefault": try entry(config: ["assetID": first])], displayID: "display", workspaceURL: defaultURL), .unavailable)
        let unknown = try entry("new-provider", config: [:])
        XCTAssertEqual(DesktopWallpaper.selection(store: ["AllSpacesAndDisplays": unknown], displayID: "display", workspaceURL: defaultURL), .unavailable)
        XCTAssertEqual(DesktopWallpaper.selection(store: [:], displayID: "display", workspaceURL: defaultURL), .unavailable)
    }

    func testPerSpacePhotoUsesPublicAPIAndUnsafeAssetIDsAreRejected() throws {
        let photo = URL(fileURLWithPath: "/tmp/WallpaperFixtures/space.png")
        XCTAssertEqual(DesktopWallpaper.selection(store: [:], displayID: "display", workspaceURL: photo), .image(photo))
        let store = ["AllSpacesAndDisplays": try entry(config: ["assetID": "../../unrelated.mov"])]
        XCTAssertEqual(DesktopWallpaper.selection(store: store, displayID: "display", workspaceURL: defaultURL), .unavailable)
        XCTAssertNil(DesktopWallpaper(home: FileManager.default.temporaryDirectory).file(for: .aerial("../../unrelated")))
    }

    func testStoredAerialResolvesOnlyItsExactLocalAsset() throws {
        let home = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: home) }
        let store = home.appendingPathComponent(DesktopWallpaper.storePath)
        let asset = home.appendingPathComponent("Library/Application Support/com.apple.wallpaper/aerials/videos/\(first).mov")
        try FileManager.default.createDirectory(at: store.deletingLastPathComponent(), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: asset.deletingLastPathComponent(), withIntermediateDirectories: true)
        let data = try PropertyListSerialization.data(fromPropertyList: ["AllSpacesAndDisplays": entry(config: ["assetID": first])], format: .binary, options: 0)
        try data.write(to: store); try Data().write(to: asset)
        let resolver = DesktopWallpaper(home: home)
        XCTAssertEqual(resolver.file(for: resolver.selection(displayID: "display", workspaceURL: defaultURL)), asset)
        XCTAssertNil(resolver.file(for: .aerial("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE")))
        XCTAssertEqual(try Data(contentsOf: store), data)
    }
}
