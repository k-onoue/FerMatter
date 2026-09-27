import Foundation

/// SemVer precedence (build metadata is intentionally ignored). Release tags may start with v.
struct ReleaseVersion: Comparable, CustomStringConvertible {
    private let core: [String]
    let prerelease: [String]
    var description: String { core.joined(separator: ".") + (prerelease.isEmpty ? "" : "-" + prerelease.joined(separator: ".")) }

    init?(_ value: String) {
        let pattern = #"^v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"#
        guard let regex = try? NSRegularExpression(pattern: pattern),
              let match = regex.firstMatch(in: value, range: NSRange(value.startIndex..., in: value)),
              match.range.length == value.utf16.count else { return nil }
        func group(_ index: Int) -> String {
            guard let range = Range(match.range(at: index), in: value) else { return "" }
            return String(value[range])
        }
        core = (1...3).map(group)
        let suffix = group(4)
        prerelease = suffix.isEmpty ? [] : suffix.components(separatedBy: ".")
        guard !prerelease.contains(where: { Self.numeric($0) && $0.count > 1 && $0.first == "0" }) else { return nil }
    }

    static func < (lhs: Self, rhs: Self) -> Bool {
        for (left, right) in zip(lhs.core, rhs.core) where left != right { return numericLess(left, right) }
        if lhs.prerelease.isEmpty || rhs.prerelease.isEmpty { return !lhs.prerelease.isEmpty && rhs.prerelease.isEmpty }
        for (left, right) in zip(lhs.prerelease, rhs.prerelease) where left != right {
            let ln = numeric(left), rn = numeric(right)
            if ln && rn { return numericLess(left, right) }
            if ln != rn { return ln }
            return left < right
        }
        return lhs.prerelease.count < rhs.prerelease.count
    }

    private static func numeric(_ value: String) -> Bool { value.utf8.allSatisfy { (48...57).contains($0) } }
    private static func numericLess(_ lhs: String, _ rhs: String) -> Bool { lhs.count == rhs.count ? lhs < rhs : lhs.count < rhs.count }
}

struct GitHubRelease: Decodable {
    struct Asset: Decodable {
        let name: String
        let state: String
        let size: Int64
        let browserDownloadURL: URL
        let digest: String?
        enum CodingKeys: String, CodingKey {
            case name, state, size, digest
            case browserDownloadURL = "browser_download_url"
        }
    }
    let tagName: String
    let htmlURL: URL
    let draft: Bool
    let prerelease: Bool
    let assets: [Asset]
    enum CodingKeys: String, CodingKey {
        case draft, prerelease, assets
        case tagName = "tag_name"
        case htmlURL = "html_url"
    }
}

struct AppUpdate {
    let version: ReleaseVersion
    let releaseURL: URL
    let asset: GitHubRelease.Asset?

    init?(release: GitHubRelease, currentVersion: ReleaseVersion) {
        guard !release.draft, !release.prerelease,
              let version = ReleaseVersion(release.tagName), version.prerelease.isEmpty, version > currentVersion,
              UpdateSource.isReleaseURL(release.htmlURL, tag: release.tagName) else { return nil }
        self.version = version
        releaseURL = release.htmlURL
        // Current distributions target Apple Silicon. Keep asset naming explicit so a source archive
        // or an unrelated platform's package can never be accepted as an application update.
        asset = ["Mathlidelux.dmg", "Mathlidelux-\(version)-macOS-arm64.dmg"].compactMap { name in
            release.assets.first {
                $0.name == name && $0.state == "uploaded" && $0.size > 0 && $0.size <= UpdateDownloader.maximumSize &&
                UpdateSource.isAssetURL($0.browserDownloadURL, tag: release.tagName, name: $0.name)
            }
        }.first
    }
}

enum UpdateSource {
    static let repositoryPath = "/k-onoue/mathlidelux"
    static let latestURL = URL(string: "https://api.github.com/repos\(repositoryPath)/releases/latest")!
    static let releasesURL = URL(string: "https://github.com\(repositoryPath)/releases")!

    static func isReleaseURL(_ url: URL, tag: String) -> Bool {
        isOfficial(url) && url.path == "\(repositoryPath)/releases/tag/\(tag)"
    }
    static func isAssetURL(_ url: URL, tag: String, name: String) -> Bool {
        isOfficial(url) && !name.contains("/") && !name.contains("\\") &&
        url.path == "\(repositoryPath)/releases/download/\(tag)/\(name)"
    }
    private static func isOfficial(_ url: URL) -> Bool {
        url.scheme == "https" && url.host == "github.com" && url.user == nil && url.password == nil &&
        url.port == nil && url.query == nil && url.fragment == nil
    }
}

enum UpdateError: LocalizedError {
    case invalidVersion, invalidResponse, rateLimited, server(Int), invalidDownload, checksumMismatch
    var errorDescription: String? {
        switch self {
        case .invalidVersion: return "This development build has no app version. Run the packaged app to check for updates."
        case .invalidResponse: return "The update service returned an unexpected response. Try again later."
        case .rateLimited: return "GitHub is limiting update checks. Try again later."
        case .server(let status): return "The update service is unavailable (HTTP \(status)). Try again later."
        case .invalidDownload: return "The update download could not be verified. Try again later."
        case .checksumMismatch: return "The downloaded file did not match the release checksum. Please try again."
        }
    }
}

struct UpdateClient {
    var session: URLSession = .shared
    func latestRelease() async throws -> GitHubRelease? {
        var request = URLRequest(url: UpdateSource.latestURL, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: 20)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        request.setValue("2022-11-28", forHTTPHeaderField: "X-GitHub-Api-Version")
        request.setValue("Mathlidelux-Update-Check", forHTTPHeaderField: "User-Agent")
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse, http.url == UpdateSource.latestURL else { throw UpdateError.invalidResponse }
        switch http.statusCode {
        case 404: return nil // The repository may not have published a stable release yet.
        case 403, 429: throw UpdateError.rateLimited
        case 200: break
        default: throw UpdateError.server(http.statusCode)
        }
        guard data.count <= 2_000_000 else { throw UpdateError.invalidResponse }
        do { return try JSONDecoder().decode(GitHubRelease.self, from: data) }
        catch { throw UpdateError.invalidResponse }
    }
}
