// swift-tools-version: 5.9
import PackageDescription

// Relative path to the Swift binding library (sibling directory).
let swiftLibDir = "../../swift"

let package = Package(
    name: "RdictApp",
    platforms: [.macOS(.v14), .iOS(.v17)],
    dependencies: [
        .package(path: swiftLibDir),
    ],
    targets: [
        .executableTarget(
            name: "RdictApp",
            dependencies: [
                .product(name: "Rdict", package: "swift"),
            ],
            path: "Sources/RdictApp"
        ),
    ]
)
