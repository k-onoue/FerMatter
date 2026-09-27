import AppKit
import SwiftUI

@MainActor
final class AppUpdates: ObservableObject {
    enum Status: Equatable {
        case idle, checking, available, upToDate, noRelease, downloading, downloaded(URL), failed(String)
    }
    @Published private(set) var status: Status = .idle
    @Published private(set) var update: AppUpdate?
    @Published private(set) var lastChecked: Date?
    @Published var showsBanner = false
    @Published var automaticallyChecks: Bool {
        didSet { defaults.set(automaticallyChecks, forKey: Self.automaticKey) }
    }
    let installedVersion: String?
    private let defaults: UserDefaults
    private let fetch: () async throws -> GitHubRelease?
    private let now: () -> Date
    private let download: (AppUpdate) async throws -> URL
    private var dismissedVersion: ReleaseVersion?
    private var downloadTask: Task<Void, Never>?
    private static let automaticKey = "updates.automaticallyChecks"
    private static let checkedKey = "updates.lastChecked"
    static let checkInterval: TimeInterval = 24 * 60 * 60

    init(defaults: UserDefaults = .standard,
         installedVersion: String? = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String,
         now: @escaping () -> Date = Date.init,
         fetch: @escaping () async throws -> GitHubRelease? = { try await UpdateClient().latestRelease() },
         download: @escaping (AppUpdate) async throws -> URL = { try await UpdateDownloader().download($0) }) {
        self.defaults = defaults
        self.installedVersion = installedVersion
        self.now = now
        self.fetch = fetch
        self.download = download
        automaticallyChecks = defaults.object(forKey: Self.automaticKey) as? Bool ?? true
        lastChecked = defaults.object(forKey: Self.checkedKey) as? Date
    }

    var isBusy: Bool { status == .checking || status == .downloading }
    var message: String {
        switch status {
        case .idle: return "Check for new versions of Mathlidelux."
        case .checking: return "Checking for updates…"
        case .available:
            let available = "Mathlidelux \(update?.version.description ?? "") is available."
            return update?.asset == nil ? available + " A compatible disk image is not available yet." : available
        case .upToDate: return "You’re up to date."
        case .noRelease: return "No published release is available yet."
        case .downloading: return "Downloading Mathlidelux \(update?.version.description ?? "")…"
        case .downloaded: return "Download ready. Open the disk image to install the update."
        case .failed(let message): return message
        }
    }

    func check(manually: Bool = true) async {
        guard !isBusy else { return }
        if !manually {
            guard automaticallyChecks, installedVersion != nil,
                  lastChecked.map({ now().timeIntervalSince($0) >= Self.checkInterval || now() < $0 }) ?? true else { return }
        }
        if manually { showsBanner = true }
        guard let installedVersion, let version = ReleaseVersion(installedVersion) else {
            status = .failed(UpdateError.invalidVersion.localizedDescription)
            return
        }
        status = .checking
        lastChecked = now()
        defaults.set(lastChecked, forKey: Self.checkedKey)
        do {
            let release = try await fetch()
            try Task.checkCancellation()
            update = release.flatMap { AppUpdate(release: $0, currentVersion: version) }
            if let update {
                status = .available
                if manually || dismissedVersion != update.version { showsBanner = true }
            } else { status = release == nil ? .noRelease : .upToDate }
        } catch is CancellationError { status = update == nil ? .idle : .available }
        catch { status = .failed(error.localizedDescription) }
    }

    func acceptUpdate() {
        guard !isBusy, let update, update.asset != nil else { return }
        status = .downloading
        showsBanner = true
        downloadTask = Task {
            defer { downloadTask = nil }
            do {
                let file = try await download(update)
                try Task.checkCancellation()
                status = .downloaded(file)
            } catch is CancellationError { status = .available }
            catch let error as URLError where error.code == .cancelled { status = .available }
            catch { status = .failed(error.localizedDescription) }
        }
    }

    func cancelDownload() { downloadTask?.cancel() }

    func dismiss() {
        dismissedVersion = update?.version
        showsBanner = false
    }
}

struct UpdateCommands: Commands {
    @ObservedObject var updates: AppUpdates
    var body: some Commands {
        CommandGroup(after: .appInfo) {
            Button("Check for Updates…") { Task { await updates.check() } }
                .disabled(updates.isBusy)
        }
    }
}

struct UpdateBanner: View {
    @ObservedObject var updates: AppUpdates
    var body: some View {
        if updates.showsBanner {
            HStack(spacing: 12) {
                if updates.isBusy { ProgressView().controlSize(.small) }
                else { Image(systemName: "arrow.down.circle").foregroundStyle(.blue) }
                Text(updates.message).font(.callout).textSelection(.enabled)
                Spacer(minLength: 10)
                UpdateActions(updates: updates)
                if !updates.isBusy {
                    Button { updates.dismiss() } label: { Image(systemName: "xmark") }
                        .buttonStyle(.plain).help("Dismiss update message").accessibilityLabel("Dismiss update message")
                }
            }
            .padding(.horizontal, 20).padding(.vertical, 12)
            .background(.regularMaterial)
            .overlay(alignment: .bottom) { Divider() }
        }
    }
}

struct UpdateActions: View {
    @ObservedObject var updates: AppUpdates
    var body: some View {
        if updates.status == .downloading {
            Button("Cancel", action: updates.cancelDownload)
        } else if case .downloaded(let file) = updates.status {
            Button("Open Disk Image") {
                if !NSWorkspace.shared.open(file) { NSWorkspace.shared.activateFileViewerSelecting([file]) }
            }.buttonStyle(.borderedProminent)
            Button("Show in Finder") { NSWorkspace.shared.activateFileViewerSelecting([file]) }
        } else if let update = updates.update, !updates.isBusy {
            Link("View Release", destination: update.releaseURL)
            if update.asset != nil {
                Button("Download Update", action: updates.acceptUpdate).buttonStyle(.borderedProminent)
            }
        }
    }
}

struct UpdateSettingsView: View {
    @ObservedObject var updates: AppUpdates
    var body: some View {
        Form {
            Section {
                LabeledContent("Installed version", value: updates.installedVersion ?? "Development build")
                Toggle("Automatically check for updates", isOn: $updates.automaticallyChecks)
                Text("Checks GitHub for a stable release at most once a day when the app opens or becomes active.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section("Software Update") {
                HStack {
                    if updates.isBusy { ProgressView().controlSize(.small) }
                    Text(updates.message).textSelection(.enabled)
                }
                if let lastChecked = updates.lastChecked {
                    LabeledContent("Last checked") { Text(lastChecked, format: .dateTime.month().day().hour().minute()) }
                        .foregroundStyle(.secondary)
                }
                HStack {
                    Button("Check for Updates…") { Task { await updates.check() } }.disabled(updates.isBusy)
                    UpdateActions(updates: updates)
                }
            }
            Section {
                Text("Updates download to your Downloads folder. Open the disk image, then quit Mathlidelux and copy the new app to Applications.")
                    .font(.caption).foregroundStyle(.secondary)
                Link("All Releases on GitHub", destination: UpdateSource.releasesURL)
            }
        }.formStyle(.grouped).padding()
    }
}
