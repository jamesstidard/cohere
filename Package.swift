// swift-tools-version: 5.9
import PackageDescription
import Foundation

// This manifest lives at the repo root so the package is installable directly
// from GitHub (SwiftPM only reads Package.swift from a repository's root).
//
// The compiled Rust core is shipped as an XCFramework. For local development we
// link a locally-built one; consumers installing from GitHub get the
// checksummed release artifact. The two lines below are rewritten on each
// release by the Release workflow (.github/workflows/release.yml).
let releaseVersion = "0.1.0"
let releaseChecksum = "0000000000000000000000000000000000000000000000000000000000000000"

let swiftSources = "bindings/swift/Cohere/Sources/Cohere"
let swiftTests = "bindings/swift/Cohere/Tests/CohereTests"
let localXcframework = "bindings/swift/Cohere/Cohere.xcframework"

// Set COHERE_LOCAL_XCFRAMEWORK=1 to link a locally-built XCFramework (developer
// workflow — build it first with bindings/swift/Cohere/build-xcframework.sh).
// Without it — e.g. installed from GitHub — the released, checksummed artifact
// is used. We key off the environment rather than the file's presence because
// the environment is part of SwiftPM's manifest cache key, so the choice tracks
// reliably; a cached "does this file exist" result would not.
let useLocalXcframework =
    ProcessInfo.processInfo.environment["COHERE_LOCAL_XCFRAMEWORK"] != nil

let ffiTarget: Target = useLocalXcframework
    ? .binaryTarget(name: "CohereFFI", path: localXcframework)
    : .binaryTarget(
        name: "CohereFFI",
        url: "https://github.com/jamesstidard/cohere/releases/download/v\(releaseVersion)/Cohere.xcframework.zip",
        checksum: releaseChecksum
    )

let package = Package(
    name: "Cohere",
    platforms: [
        .macOS(.v11),
        .iOS(.v13),
    ],
    products: [
        .library(name: "Cohere", targets: ["Cohere"]),
    ],
    targets: [
        ffiTarget,
        .target(
            name: "Cohere",
            dependencies: ["CohereFFI"],
            path: swiftSources
        ),
        .testTarget(
            name: "CohereTests",
            dependencies: ["Cohere"],
            path: swiftTests
        ),
    ]
)
