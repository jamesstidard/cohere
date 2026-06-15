# Cohere (Swift)

Swift bindings for [cohere](../../../README.md), a portable JSON/TOML validator
with relational JSON Schema extensions. The Rust core is exposed through a small
C ABI (the sibling `../ffi` crate) and wrapped in an idiomatic Swift API here.

> **The package manifest is `Package.swift` at the repo root, not in this
> directory.** SwiftPM only reads `Package.swift` from a repository's root, so
> that's where it has to live for the package to be installable from GitHub. This
> directory holds the Swift sources, tests, and build/release tooling that the
> root manifest points at.

## Layout

```
Package.swift                  # SwiftPM manifest (repo ROOT)
bindings/swift/
├── ffi/                       # Rust C FFI crate (cohere-swift)
└── Cohere/                    # this directory
    ├── build-xcframework.sh   #   Builds Cohere.xcframework from ../ffi
    ├── Cohere.xcframework     #   Compiled Rust (generated; git-ignored)
    ├── Sources/Cohere/        #   Swift wrapper: Schema, ValidationResult, ...
    └── Tests/CohereTests/
```

The package has two targets:

- **`CohereFFI`** — a `binaryTarget`. For local development it points at the
  locally-built `Cohere.xcframework`; for GitHub installs it points at a released
  `url:` + `checksum:`. It bundles the compiled `cohere-swift` static library
  plus the C header and module map; its Clang module is imported as `CCohere`.
- **`Cohere`** — the Swift wrapper that turns the C ABI into `Schema`,
  `ValidationResult`, and `ValidationError`.

## Local development

The XCFramework is a compiled artifact and is **not** checked in. Build it, then
set `COHERE_LOCAL_XCFRAMEWORK=1` so the root manifest links it instead of a
released artifact:

```bash
./build-xcframework.sh                  # release build (default)
PROFILE=debug ./build-xcframework.sh    # faster, for iterating

COHERE_LOCAL_XCFRAMEWORK=1 swift test   # run from the repo root
```

Why the env var rather than just detecting the file? SwiftPM caches manifest
evaluation, and the environment is part of that cache key while arbitrary
filesystem reads are not — so keying off the env var makes the local/released
choice track reliably. If you ever see a download/checksum error locally, make
sure the env var is set (and `swift package reset` to clear a stale cache).

`build-xcframework.sh` builds the static library for each installed Apple Rust
target and assembles slices for macOS, iOS device, and iOS simulator. Missing
targets are added with `rustup target add` automatically, or skipped with a
warning if that fails (e.g. offline). To control the platforms, pre-install the
triples you want:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin \
                  aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

## Installing from another project

The released XCFramework is downloaded and checksum-verified, so consumers need
no Rust toolchain:

```swift
.package(url: "https://github.com/jamesstidard/cohere", from: "0.1.0"),
// ...
.product(name: "Cohere", package: "cohere"),
```

For a local checkout, use `.package(path: "/path/to/cohere")` with
`COHERE_LOCAL_XCFRAMEWORK=1` set and the XCFramework built.

## API

```swift
import Cohere

let schema = try Schema(json: #"{"type": "object", "required": ["name"]}"#)

let result = try schema.validate(json: #"{"name": "alice"}"#)
result.valid          // Bool
result.errors         // [ValidationError] — message, path, value, line, column

try schema.validate(toml: #"name = "alice""#)
```

`Schema(json:)` and the `validate` methods throw `CohereError` when the schema or
the input document itself cannot be parsed. Schema *violations* are not thrown —
they appear as `result.errors` with `result.valid == false`.

## Releasing

The Swift package is released as part of the repo-wide *Release* workflow
(`.github/workflows/release.yml`), which publishes one GitHub Release per version
containing the Swift, Python, and WASM artifacts together. For Swift it builds
the XCFramework, computes the checksum, and commits it into the root
`Package.swift` at the tag — so the checksum always matches the uploaded zip.
See [Releasing](../../../README.md#releasing) in the top-level README.
