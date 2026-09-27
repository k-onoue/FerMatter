import Foundation
import CryptoKit
import Darwin

struct UpdateDownloader {
    static let maximumSize: Int64 = 2_000_000_000

    func download(_ update: AppUpdate) async throws -> URL {
        guard let asset = update.asset else { throw UpdateError.invalidDownload }
        let policy = UpdateDownloadRedirects(assetURL: asset.browserDownloadURL)
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 30
        configuration.timeoutIntervalForResource = 15 * 60
        configuration.httpShouldSetCookies = false
        let session = URLSession(configuration: configuration, delegate: policy, delegateQueue: nil)
        defer { session.invalidateAndCancel() }
        let (temporaryURL, response) = try await session.download(from: asset.browserDownloadURL)
        defer { try? FileManager.default.removeItem(at: temporaryURL) }
        try Task.checkCancellation()
        guard let http = response as? HTTPURLResponse, http.statusCode == 200,
              let finalURL = http.url, policy.permits(finalURL) else { throw UpdateError.invalidDownload }
        try Self.validateFile(temporaryURL, asset: asset)
        try Task.checkCancellation()

        let manager = FileManager.default
        let downloads = try manager.url(for: .downloadsDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
        let directory = downloads.appendingPathComponent("Mathlidelux-\(update.version)-\(UUID().uuidString.prefix(8))", isDirectory: true)
        try manager.createDirectory(at: directory, withIntermediateDirectories: false)
        let destination = directory.appendingPathComponent(asset.name)
        do {
            try manager.moveItem(at: temporaryURL, to: destination)
            // URLSession downloads do not reliably add quarantine. Preserve Gatekeeper's normal
            // downloaded-file checks when Finder opens this disk image.
            try Self.quarantine(destination)
            try Task.checkCancellation()
            return destination
        } catch {
            try? manager.removeItem(at: directory)
            throw error
        }
    }

    static func validateFile(_ url: URL, asset: GitHubRelease.Asset) throws {
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        guard let size = attributes[.size] as? NSNumber, size.int64Value == asset.size,
              asset.size >= 512, asset.size <= maximumSize else { throw UpdateError.invalidDownload }
        let file = try FileHandle(forReadingFrom: url)
        defer { try? file.close() }
        // UDIF disk images end with a 512-byte trailer beginning with "koly".
        try file.seek(toOffset: UInt64(asset.size - 512))
        guard try file.read(upToCount: 4) == Data("koly".utf8) else { throw UpdateError.invalidDownload }
        if let digest = asset.digest {
            let prefix = "sha256:"
            guard digest.hasPrefix(prefix) else { throw UpdateError.invalidDownload }
            let expected = String(digest.dropFirst(prefix.count)).lowercased()
            guard expected.count == 64, expected.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }) else {
                throw UpdateError.invalidDownload
            }
            try file.seek(toOffset: 0)
            var hash = SHA256()
            while let chunk = try file.read(upToCount: 1_048_576), !chunk.isEmpty {
                try Task.checkCancellation()
                hash.update(data: chunk)
            }
            guard hash.finalize().map({ String(format: "%02x", $0) }).joined() == expected else { throw UpdateError.checksumMismatch }
        }
    }

    private static func quarantine(_ url: URL) throws {
        let value = "0083;\(String(Int(Date().timeIntervalSince1970), radix: 16));Mathlidelux;\(UUID().uuidString)"
        let result = value.withCString { bytes in
            url.withUnsafeFileSystemRepresentation { path in
                setxattr(path!, "com.apple.quarantine", bytes, value.utf8.count, 0, 0)
            }
        }
        guard result == 0 else { throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno)) }
    }
}

final class UpdateDownloadRedirects: NSObject, URLSessionDownloadDelegate {
    let assetURL: URL
    init(assetURL: URL) { self.assetURL = assetURL }

    func permits(_ url: URL) -> Bool {
        guard url.scheme == "https", url.user == nil, url.password == nil, url.port == nil, url.fragment == nil else { return false }
        return url == assetURL || url.host == "release-assets.githubusercontent.com"
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) {
        completionHandler(request.url.map(permits) == true ? request : nil)
    }

    func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {}

    func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask,
                    didWriteData bytesWritten: Int64, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) {
        if totalBytesWritten > UpdateDownloader.maximumSize { downloadTask.cancel() }
    }
}
