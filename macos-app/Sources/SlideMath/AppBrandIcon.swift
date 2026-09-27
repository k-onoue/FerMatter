import AppKit
import SwiftUI

/// Use the approved artwork for in-app branding as well as the application bundle.
struct AppBrandIcon: View {
    var size: CGFloat = 32
    private static let image: NSImage = {
        if let url = Bundle.main.url(forResource: "AppMark", withExtension: "png"),
           let image = NSImage(contentsOf: url) { return image }
        return NSApp.applicationIconImage
    }()

    var body: some View {
        Image(nsImage: Self.image).resizable().interpolation(.high).scaledToFit()
            .frame(width: size, height: size).accessibilityHidden(true)
    }
}
