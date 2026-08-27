// swift-tools-version: 5.9
import PackageDescription

// Derive this package's directory from #file (the Package.swift path).
let libDir = String(#file.dropLast("/Package.swift".count))

let package = Package(
    name: "Rdict",
    platforms: [.macOS(.v14), .iOS(.v17)],
    products: [
        .library(name: "Rdict", targets: ["Rdict"]),
    ],
    targets: [
        .target(
            name: "CRdict",
            path: "Sources/CRdict",
            publicHeadersPath: "include",
            linkerSettings: [
                .unsafeFlags([
                    "-L\(libDir)/lib",
                    "-lrdict",
                    "-lstdc++",
                ]),
            ]
        ),
        .target(
            name: "Rdict",
            dependencies: ["CRdict"],
            path: "Sources/Rdict"
        ),
    ]
)
