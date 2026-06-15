// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "cohere-example",
    platforms: [
        .macOS(.v11),
    ],
    dependencies: [
        // The Cohere package (manifest at the repo root). Build its XCFramework
        // first: ../../bindings/swift/Cohere/build-xcframework.sh
        .package(path: "../.."),
    ],
    targets: [
        .executableTarget(
            name: "cohere-example",
            dependencies: [
                .product(name: "Cohere", package: "cohere"),
            ]
        ),
    ]
)
