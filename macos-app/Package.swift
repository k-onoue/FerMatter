// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "Mathlidelux",
    platforms: [.macOS(.v14)],
    products: [.executable(name: "Mathlidelux", targets: ["SlideMath"])],
    targets: [
        .executableTarget(name: "SlideMath", resources: [.copy("Themes")]),
        .testTarget(name: "SlideMathTests", dependencies: ["SlideMath"], path: "Tests/SlideMathTests")
    ]
)
