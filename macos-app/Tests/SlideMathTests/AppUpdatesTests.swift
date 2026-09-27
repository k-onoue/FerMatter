import XCTest
import CryptoKit
@testable import SlideMath

final class AppUpdatesTests: XCTestCase {
    func testSemanticVersionOrderingAndBuildMetadata() throws {
        let ordered = ["1.0.0-alpha", "1.0.0-alpha.1", "1.0.0-alpha.beta", "1.0.0-beta", "1.0.0-beta.2", "1.0.0-beta.11", "1.0.0-rc.1", "1.0.0", "1.9.0", "1.10.0", "2.0.0"]
        let versions = try ordered.map { try XCTUnwrap(ReleaseVersion($0)) }
        for (left, right) in zip(versions, versions.dropFirst()) { XCTAssertLessThan(left, right) }
        XCTAssertEqual(ReleaseVersion("v1.2.3+build.01"), ReleaseVersion("1.2.3+other"))
        XCTAssertLessThan(try XCTUnwrap(ReleaseVersion("1.0.0-999999999999999999999999")), try XCTUnwrap(ReleaseVersion("1.0.0-1000000000000000000000000")))
    }

    func testRejectsMalformedVersions() {
        for value in ["", "1", "1.2", "1.2.3.4", "01.2.3", "1.02.3", "1.2.03", "1.2.3-01", "1.2.3-alpha..2", "1.2.3+", "1.2.3\n", " 1.2.3", "1.2.3/evil"] {
            XCTAssertNil(ReleaseVersion(value), value)
        }
    }

    func testOnlyNewStableOfficialReleasesAreOffered() throws {
        let current = try XCTUnwrap(ReleaseVersion("0.10.0"))
        XCTAssertNotNil(AppUpdate(release: release(), currentVersion: current))
        for candidate in [release(tag: "v0.9.9"), release(tag: "v0.10.0"), release(tag: "v0.11.0-rc.1"),
                          release(draft: true), release(prerelease: true), release(url: "https://example.com/release")] {
            XCTAssertNil(AppUpdate(release: candidate, currentVersion: current))
        }
        XCTAssertNotNil(AppUpdate(release: release(tag: "v0.10.0"), currentVersion: try XCTUnwrap(ReleaseVersion("0.10.0-rc.1"))))
    }

    func testAssetSelectionPreservesPublishedNameAndRejectsPrototype() throws {
        let current = try XCTUnwrap(ReleaseVersion("0.10.0"))
        let versioned = asset(name: "Mathlidelux-0.11.0-macOS-arm64.dmg")
        let compatible = asset()
        let update = try XCTUnwrap(AppUpdate(release: release(assets: [versioned, compatible]), currentVersion: current))
        XCTAssertEqual(update.asset?.name, "Mathlidelux.dmg")
        let unavailable = release(assets: [asset(name: "Mathlidelux-prototype-unnotarized.dmg"), asset(name: "source.zip"), asset(url: "https://example.com/Mathlidelux.dmg")])
        XCTAssertNil(AppUpdate(release: unavailable, currentVersion: current)?.asset)
    }

    func testURLValidationAndRedirectBoundaries() {
        let official = asset().browserDownloadURL
        XCTAssertTrue(UpdateSource.isAssetURL(official, tag: "v0.11.0", name: "Mathlidelux.dmg"))
        for value in ["http://github.com/k-onoue/mathlidelux/releases/download/v0.11.0/Mathlidelux.dmg", "https://github.com/other/mathlidelux/releases/download/v0.11.0/Mathlidelux.dmg", official.absoluteString + "?redirect=evil", official.absoluteString + "#fragment", "https://user@github.com/k-onoue/mathlidelux/releases/download/v0.11.0/Mathlidelux.dmg"] {
            XCTAssertFalse(UpdateSource.isAssetURL(URL(string: value)!, tag: "v0.11.0", name: "Mathlidelux.dmg"), value)
        }
        let policy = UpdateDownloadRedirects(assetURL: official)
        XCTAssertTrue(policy.permits(official))
        XCTAssertTrue(policy.permits(URL(string: "https://release-assets.githubusercontent.com/github-production-release-asset/file?token=test")!))
        for value in ["http://release-assets.githubusercontent.com/file", "https://release-assets.githubusercontent.com.evil.test/file", "https://example.com/file", "https://github.com/other/repo/file", "https://user:password@release-assets.githubusercontent.com/file"] {
            XCTAssertFalse(policy.permits(URL(string: value)!), value)
        }
    }

    func testDiskImageVerificationChecksSizeFormatAndDigest() throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: url) }
        var data = Data(repeating: 0, count: 1024)
        data.replaceSubrange(512..<516, with: Data("koly".utf8))
        try data.write(to: url)
        let digest = "sha256:" + SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        XCTAssertNoThrow(try UpdateDownloader.validateFile(url, asset: asset(size: 1024, digest: digest)))
        XCTAssertNoThrow(try UpdateDownloader.validateFile(url, asset: asset(size: 1024)))
        XCTAssertThrowsError(try UpdateDownloader.validateFile(url, asset: asset(size: 1000)))
        XCTAssertThrowsError(try UpdateDownloader.validateFile(url, asset: asset(size: 1024, digest: "sha256:" + String(repeating: "0", count: 64))))
        XCTAssertThrowsError(try UpdateDownloader.validateFile(url, asset: asset(size: 1024, digest: "sha1:invalid")))
        try Data(repeating: 0, count: 1024).write(to: url)
        XCTAssertThrowsError(try UpdateDownloader.validateFile(url, asset: asset(size: 1024)))
    }

    @MainActor
    func testAutomaticCheckThrottleOptOutAndManualOverride() async {
        let defaults = isolatedDefaults()
        var calls = 0
        var now = Date(timeIntervalSince1970: 1_800_000_000)
        let candidate = release()
        let updates = AppUpdates(defaults: defaults, installedVersion: "0.10.0", now: { now }, fetch: { calls += 1; return candidate })
        await updates.check(manually: false)
        XCTAssertEqual(calls, 1)
        XCTAssertEqual(updates.status, .available)
        XCTAssertTrue(updates.showsBanner)
        updates.dismiss()
        await updates.check(manually: false)
        XCTAssertEqual(calls, 1)
        now.addTimeInterval(AppUpdates.checkInterval)
        await updates.check(manually: false)
        XCTAssertEqual(calls, 2)
        XCTAssertFalse(updates.showsBanner)
        updates.automaticallyChecks = false
        now.addTimeInterval(AppUpdates.checkInterval)
        await updates.check(manually: false)
        XCTAssertEqual(calls, 2)
        await updates.check()
        XCTAssertEqual(calls, 3)
        XCTAssertTrue(updates.showsBanner)
        let restored = AppUpdates(defaults: defaults, installedVersion: "0.10.0")
        XCTAssertFalse(restored.automaticallyChecks)
        XCTAssertEqual(restored.lastChecked, now)
    }

    @MainActor
    func testMissingReleaseAndOfflineAreQuietUntilManualCheck() async {
        let noRelease = AppUpdates(defaults: isolatedDefaults(), installedVersion: "0.10.0", fetch: { nil })
        await noRelease.check(manually: false)
        XCTAssertEqual(noRelease.status, .noRelease)
        XCTAssertFalse(noRelease.showsBanner)
        await noRelease.check()
        XCTAssertTrue(noRelease.showsBanner)
        let offline = AppUpdates(defaults: isolatedDefaults(), installedVersion: "0.10.0", fetch: { throw URLError(.notConnectedToInternet) })
        await offline.check(manually: false)
        guard case .failed = offline.status else { return XCTFail("Expected offline state") }
        XCTAssertFalse(offline.showsBanner)
        await offline.check()
        XCTAssertTrue(offline.showsBanner)
    }

    @MainActor
    func testDevelopmentBuildDoesNotCheckAutomatically() async {
        var calls = 0
        let updates = AppUpdates(defaults: isolatedDefaults(), installedVersion: nil, fetch: { calls += 1; return nil })
        await updates.check(manually: false)
        XCTAssertEqual(updates.status, .idle)
        XCTAssertEqual(calls, 0)
        await updates.check()
        XCTAssertTrue(updates.showsBanner)
        XCTAssertEqual(updates.status, .failed(UpdateError.invalidVersion.localizedDescription))
    }

    @MainActor
    func testAcceptanceCancellationFailureAndReadyStates() async throws {
        let candidate = release(assets: [asset()])
        let ready = URL(fileURLWithPath: "/tmp/Mathlidelux.dmg")
        var downloads = 0
        let updates = AppUpdates(defaults: isolatedDefaults(), installedVersion: "0.10.0", fetch: { candidate }, download: { _ in downloads += 1; return ready })
        await updates.check()
        XCTAssertEqual(downloads, 0)
        updates.acceptUpdate()
        XCTAssertEqual(updates.status, .downloading)
        updates.acceptUpdate()
        try await settle(updates)
        XCTAssertEqual(downloads, 1)
        XCTAssertEqual(updates.status, .downloaded(ready))
        let cancelled = AppUpdates(defaults: isolatedDefaults(), installedVersion: "0.10.0", fetch: { candidate }, download: { _ in
            try await Task.sleep(nanoseconds: 5_000_000_000)
            return ready
        })
        await cancelled.check()
        cancelled.acceptUpdate()
        cancelled.cancelDownload()
        try await settle(cancelled)
        XCTAssertEqual(cancelled.status, .available)
        let failed = AppUpdates(defaults: isolatedDefaults(), installedVersion: "0.10.0", fetch: { candidate }, download: { _ in throw UpdateError.checksumMismatch })
        await failed.check()
        failed.acceptUpdate()
        try await settle(failed)
        XCTAssertEqual(failed.status, .failed(UpdateError.checksumMismatch.localizedDescription))
        XCTAssertNotNil(failed.update)
    }

    func testGitHubClientHandlesReleaseMissingRateLimitAndMalformedResponses() async throws {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [UpdateStubProtocol.self]
        let session = URLSession(configuration: configuration)
        defer { session.invalidateAndCancel() }
        let client = UpdateClient(session: session)
        UpdateStubProtocol.response = { request in
            XCTAssertEqual(request.url, UpdateSource.latestURL)
            XCTAssertEqual(request.value(forHTTPHeaderField: "Accept"), "application/vnd.github+json")
            XCTAssertEqual(request.value(forHTTPHeaderField: "X-GitHub-Api-Version"), "2022-11-28")
            return (200, Data(#"{"tag_name":"v0.11.0","html_url":"https://github.com/k-onoue/mathlidelux/releases/tag/v0.11.0","draft":false,"prerelease":false,"assets":[]}"#.utf8))
        }
        let release = try await client.latestRelease()
        XCTAssertEqual(release?.tagName, "v0.11.0")
        UpdateStubProtocol.response = { _ in (404, Data()) }
        let missing = try await client.latestRelease()
        XCTAssertNil(missing)
        for (status, data) in [(403, Data()), (429, Data()), (500, Data()), (200, Data("invalid json".utf8))] {
            UpdateStubProtocol.response = { _ in (status, data) }
            do { _ = try await client.latestRelease(); XCTFail("Expected failure for HTTP \(status)") }
            catch { XCTAssertTrue(error is UpdateError) }
        }
    }

    private func asset(name: String = "Mathlidelux.dmg", url: String? = nil, size: Int64 = 1024, digest: String? = nil) -> GitHubRelease.Asset {
        .init(name: name, state: "uploaded", size: size, browserDownloadURL: URL(string: url ?? "https://github.com/k-onoue/mathlidelux/releases/download/v0.11.0/\(name)")!, digest: digest)
    }
    private func release(tag: String = "v0.11.0", draft: Bool = false, prerelease: Bool = false, url: String? = nil, assets: [GitHubRelease.Asset] = []) -> GitHubRelease {
        .init(tagName: tag, htmlURL: URL(string: url ?? "https://github.com/k-onoue/mathlidelux/releases/tag/\(tag)")!, draft: draft, prerelease: prerelease, assets: assets)
    }
    private func isolatedDefaults() -> UserDefaults {
        let name = "MathlideluxUpdateTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: name)!
        addTeardownBlock { defaults.removePersistentDomain(forName: name) }
        return defaults
    }
    @MainActor private func settle(_ updates: AppUpdates) async throws {
        for _ in 0..<500 where updates.isBusy { try await Task.sleep(nanoseconds: 1_000_000) }
        XCTAssertFalse(updates.isBusy)
    }
}

private final class UpdateStubProtocol: URLProtocol {
    static var response: ((URLRequest) -> (Int, Data))?
    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func startLoading() {
        let (status, data) = Self.response!(request)
        client?.urlProtocol(self, didReceive: HTTPURLResponse(url: request.url!, statusCode: status, httpVersion: "HTTP/1.1", headerFields: nil)!, cacheStoragePolicy: .notAllowed)
        client?.urlProtocol(self, didLoad: data)
        client?.urlProtocolDidFinishLoading(self)
    }
    override func stopLoading() {}
}
