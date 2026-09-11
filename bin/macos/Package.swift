// swift-tools-version: 6.0
import Foundation
import PackageDescription

let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
let rustProfile = ProcessInfo.processInfo.environment["PITPLS_RUST_PROFILE"] ?? "debug"
let library = root.appendingPathComponent("../../target/\(rustProfile)/libmacos_bindings.a").standardized.path

let package = Package(
    name: "Pitpls",
    platforms: [.macOS(.v14)],
    products: [.executable(name: "Pitpls", targets: ["Pitpls"])],
    targets: [
        .systemLibrary(name: "PitCoreFFI"),
        .target(name: "PitCore", dependencies: ["PitCoreFFI"], linkerSettings: [
            .unsafeFlags([library]),
            .linkedFramework("Security"),
            .linkedFramework("SystemConfiguration"),
            .linkedFramework("CoreFoundation"),
            .linkedLibrary("iconv"),
            .linkedLibrary("c++"),
        ]),
        .executableTarget(name: "Pitpls", dependencies: ["PitCore"]),
    ]
)
