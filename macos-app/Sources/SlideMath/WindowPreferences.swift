import AppKit
import SwiftUI

@MainActor
final class WindowPreferences: ObservableObject {
    @Published var appearance: String { didSet { defaults.set(appearance, forKey: "appearance") } }
    @Published var pinned: Bool { didSet { defaults.set(pinned, forKey: "pinned") } }
    @Published var opacity: Double { didSet { defaults.set(opacity, forKey: "opacity") } }
    private let defaults: UserDefaults
    private weak var primaryWindow: NSWindow?
    private var enteringFullScreen = false
    var fullScreenShortcut = AppCommand.fullScreen.defaultShortcut
    private var fullScreenMonitor: Any?

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        appearance = defaults.string(forKey: "appearance") ?? "system"
        pinned = defaults.bool(forKey: "pinned")
        let savedOpacity = defaults.object(forKey: "opacity") as? Double ?? 1
        opacity = min(1, max(0.45, savedOpacity))
    }

    var colorScheme: ColorScheme? { appearance == "dark" ? .dark : appearance == "light" ? .light : nil }

    func apply(to window: NSWindow) {
        primaryWindow = window
        if fullScreenMonitor == nil {
            // The system's disabled full-screen menu item can consume the same
            // shortcut while a window floats. Route our binding before menus.
            fullScreenMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
                guard let self, let window = self.primaryWindow, NSApp.isActive,
                      (event.window === window || NSApp.keyWindow === window),
                      window.attachedSheet == nil, !self.fullScreenShortcut.key.isEmpty,
                      ShortcutBinding.from(event) == self.fullScreenShortcut else { return event }
                self.toggleFullScreen(); return nil
            }
        }
        if window.styleMask.contains(.fullScreen) { enteringFullScreen = false }
        window.level = pinned && !enteringFullScreen && !window.styleMask.contains(.fullScreen) ? .floating : .normal
        window.collectionBehavior.subtract([.transient, .stationary, .fullScreenAuxiliary, .fullScreenNone])
        window.collectionBehavior.formUnion([.managed, .fullScreenPrimary])
        // Keep text, math, and controls opaque; only view backgrounds use opacity.
        window.alphaValue = 1
        window.isOpaque = false
        window.backgroundColor = .clear
        // Extend the shared backdrop beneath the native title bar. Content still
        // respects the title-bar safe area, leaving traffic lights and dragging intact.
        window.styleMask.insert(.fullSizeContentView)
        window.titlebarAppearsTransparent = true
        window.titlebarSeparatorStyle = .none
        window.appearance = appearance == "system" ? nil : NSAppearance(named: appearance == "dark" ? .darkAqua : .aqua)
    }

    func toggleFullScreen() {
        guard let window = primaryWindow, window.attachedSheet == nil, !enteringFullScreen else { return }
        enteringFullScreen = !window.styleMask.contains(.fullScreen)
        window.level = .normal
        // Give AppKit a run-loop turn to recognize the new window level before
        // it checks whether this previously floating window may enter a Space.
        DispatchQueue.main.async { [weak window] in window?.toggleFullScreen(nil) }
        // Restore pinning if the system rejects the transition.
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self, weak window] in
            guard let self, let window else { return }
            self.enteringFullScreen = false; self.apply(to: window)
        }
    }
    deinit { if let fullScreenMonitor { NSEvent.removeMonitor(fullScreenMonitor) } }
}

struct WindowConfiguration: NSViewRepresentable {
    @ObservedObject var preferences: WindowPreferences
    @ObservedObject var shortcuts: ShortcutPreferences
    func makeNSView(context: Context) -> WindowProbe { WindowProbe() }
    func updateNSView(_ view: WindowProbe, context: Context) {
        preferences.fullScreenShortcut = shortcuts.binding(.fullScreen)
        view.configure = { window in preferences.apply(to: window) }
        if let window = view.window { preferences.apply(to: window) }
    }
}

final class WindowProbe: NSView {
    var configure: ((NSWindow) -> Void)?
    private var wallpaper: NSImage?
    private let loader = WallpaperImageLoader()
    private var refreshTask: Task<Void, Never>?
    private var refreshTimer: Timer?
    private var revision = 0
    private var observers: [NSObjectProtocol] = []
    private var workspaceObservers: [NSObjectProtocol] = []
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        observers.forEach(NotificationCenter.default.removeObserver); observers = []
        workspaceObservers.forEach(NSWorkspace.shared.notificationCenter.removeObserver); workspaceObservers = []
        refreshTask?.cancel(); refreshTimer?.invalidate(); wallpaper = nil
        guard let window else { return }
        configure?(window)
        refreshBackdrop()
        // Observe wallpaper changes even when the app stays in the same Space.
        refreshTimer = Timer.scheduledTimer(withTimeInterval: 10, repeats: true) { [weak self] _ in self?.refreshBackdrop() }
        for name in [NSWindow.willEnterFullScreenNotification, NSWindow.didEnterFullScreenNotification,
                     NSWindow.didExitFullScreenNotification,
                     NSWindow.didChangeScreenNotification, NSWindow.didChangeBackingPropertiesNotification] {
            observers.append(NotificationCenter.default.addObserver(forName: name, object: window, queue: .main) { [weak self, weak window] note in
                guard let self, let window else { return }
                self.configure?(window)
                if name == NSWindow.didExitFullScreenNotification {
                    // Reapply after AppKit has finished restoring the window's
                    // level and style mask at the end of the transition.
                    DispatchQueue.main.async { [weak self, weak window] in
                        if let window { self?.configure?(window) }
                    }
                }
                self.refreshBackdrop()
            })
        }
        observers.append(NotificationCenter.default.addObserver(forName: NSApplication.didChangeScreenParametersNotification, object: nil, queue: .main) { [weak self] _ in
            // The built-in panel can disappear or reappear when the lid changes.
            // Resolve the display UUID again after AppKit reassigns the screen.
            DispatchQueue.main.async { self?.refreshBackdrop() }
        })
        for name in [NSWorkspace.activeSpaceDidChangeNotification, NSWorkspace.didWakeNotification, NSWorkspace.screensDidWakeNotification] {
            workspaceObservers.append(NSWorkspace.shared.notificationCenter.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                self?.refreshBackdrop()
            })
        }
    }
    func refreshBackdrop() {
        revision += 1
        refreshTask?.cancel()
        guard let window, window.styleMask.contains(.fullScreen), let screen = window.screen,
              let number = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber,
              let uuid = CGDisplayCreateUUIDFromDisplayID(number.uint32Value)?.takeRetainedValue() else {
            wallpaper = nil; needsDisplay = true; return
        }
        let displayID = CFUUIDCreateString(nil, uuid) as String
        let workspaceURL = NSWorkspace.shared.desktopImageURL(for: screen)
        let request = revision
        let home = FileManager.default.homeDirectoryForCurrentUser
        refreshTask = Task { [weak self, loader] in
            let image = await loader.image(home: home, displayID: displayID, workspaceURL: workspaceURL)
            guard !Task.isCancelled, let self, self.revision == request else { return }
            self.wallpaper = image.map { NSImage(cgImage: $0, size: .zero) }
            self.needsDisplay = true
        }
    }
    override func draw(_ dirtyRect: NSRect) {
        guard let wallpaper, wallpaper.size.width > 0, wallpaper.size.height > 0 else { return }
        let scale = max(bounds.width / wallpaper.size.width, bounds.height / wallpaper.size.height)
        let size = NSSize(width: wallpaper.size.width * scale, height: wallpaper.size.height * scale)
        let rect = NSRect(x: bounds.midX - size.width / 2, y: bounds.midY - size.height / 2, width: size.width, height: size.height)
        NSGraphicsContext.saveGraphicsState()
        bounds.clip()
        wallpaper.draw(in: rect, from: .zero, operation: .sourceOver, fraction: 1)
        NSGraphicsContext.restoreGraphicsState()
    }
    deinit {
        refreshTask?.cancel(); refreshTimer?.invalidate()
        observers.forEach(NotificationCenter.default.removeObserver)
        workspaceObservers.forEach(NSWorkspace.shared.notificationCenter.removeObserver)
    }
}

struct WindowControls: View {
    @EnvironmentObject private var preferences: WindowPreferences
    @State private var showAppearance = false
    var body: some View {
        HStack(spacing: 6) {
            Button { preferences.pinned.toggle() } label: {
                Image(systemName: preferences.pinned ? "pin.fill" : "pin")
                    .foregroundStyle(preferences.pinned ? Color.accentColor : Color.secondary)
                    .frame(width: 28, height: 28)
            }.buttonStyle(.plain).help("Always on Top")
                .accessibilityLabel("Always on Top").accessibilityValue(preferences.pinned ? "On" : "Off")
            Button { showAppearance.toggle() } label: {
                Image(systemName: "circle.lefthalf.filled").frame(width: 28, height: 28)
            }.buttonStyle(.plain).foregroundStyle(.secondary).help("Appearance & Window")
                .accessibilityLabel("Appearance & Window")
                .popover(isPresented: $showAppearance, arrowEdge: .bottom) {
                    VStack(alignment: .leading, spacing: 20) {
                        Text("Appearance").font(.headline)
                        Picker("Appearance", selection: $preferences.appearance) {
                            Text("System").tag("system")
                            Text("Light").tag("light")
                            Text("Dark").tag("dark")
                        }.pickerStyle(.segmented).labelsHidden()
                        Divider()
                        Toggle("Always on Top", isOn: $preferences.pinned).toggleStyle(.switch)
                        VStack(spacing: 10) {
                            HStack { Text("Background"); Spacer(); Text("\(Int(preferences.opacity * 100))%").monospacedDigit().foregroundStyle(.secondary) }
                            Slider(value: $preferences.opacity, in: 0.45...1, step: 0.05) { Text("Background opacity") }.labelsHidden()
                            HStack { Text("45%"); Spacer(); Button("Reset") { preferences.opacity = 1 }.buttonStyle(.plain) }.font(.caption).foregroundStyle(.secondary)
                        }
                    }.padding(22).frame(width: 280)
                }
        }
    }
}
