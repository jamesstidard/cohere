# cohere

A portable JSON/TOML validator with custom JSON Schema extensions for relational constraints.

Write your validation logic once in Rust, then use it everywhere:
- **JavaScript/TypeScript** — via WebAssembly
- **Python** — via native extension (PyO3)
- **Swift** — via C FFI, distributed as a Swift package

Validates both JSON and TOML data, with validation errors mapped back to source locations (line/column) for inline editor diagnostics.

## The Problem

JSON Schema validates structure and types, but can't express relationships between data:

```json
{
  "users": [
    { "name": "alice", "age": 32 },
    { "name": "bob", "age": 28 }
  ],
  "organisations": [
    { "name": "acme", "members": ["alice", "charlie"] }
  ]
}
```

Questions JSON Schema can't answer:
- Is `charlie` in `members` actually a valid user?
- Are all user names unique?
- Is every user a member of at least one organisation?

This library adds three custom keywords to handle these relational constraints.

## Custom Keywords

### `x-uniqueAcross`

Ensures values are unique across multiple JSONPaths.

```json
"x-uniqueAcross": [
  {
    "paths": ["users[*].name", "organisations[*].name"],
    "message": "Names must be unique. Duplicate: '{{value}}'"
  }
]
```

### `x-references`

Ensures values at `from` exist somewhere in `to` — like a foreign key.

```json
"x-references": [
  {
    "from": "organisations[*].members[*]",
    "to": ["users[*].name"],
    "message": "Unknown member '{{value}}'"
  }
]
```

### `x-referencedBy`

Ensures values at `target` are referenced by values at `from`.

```json
"x-referencedBy": [
  {
    "target": "users[*].name",
    "from": ["organisations[*].members[*]"],
    "min": 1,
    "message": "User '{{value}}' is not in any organisation"
  }
]
```

## Full Example

A simple JSON "database" with users and organisations:

**Schema:**
```json
{
  "x-uniqueAcross": [
    { "paths": ["users[*].name", "organisations[*].name"] }
  ],
  "x-references": [
    { "from": "organisations[*].members[*]", "to": ["users[*].name"] }
  ],
  "x-referencedBy": [
    { "target": "users[*].name", "from": ["organisations[*].members[*]"], "min": 1 }
  ]
}
```

**Valid data:**
```json
{
  "users": [
    { "name": "alice", "age": 32 },
    { "name": "bob", "age": 28 }
  ],
  "organisations": [
    { "name": "acme", "members": ["alice", "bob"] }
  ]
}
```

**Invalid data (will fail validation):**
```json
{
  "users": [
    { "name": "alice", "age": 32 }
  ],
  "organisations": [
    { "name": "acme", "members": ["alice", "charlie"] }
  ]
}
```
Error: `Unknown member 'charlie'`

---

## Another Example: Graph Data

The same keywords work for node/edge graph structures:

**Schema:**
```json
{
  "x-uniqueAcross": [
    { "paths": ["nodes[*].name", "edges[*].name"] }
  ],
  "x-references": [
    { "from": "edges[*].from", "to": ["nodes[*].name"] },
    { "from": "edges[*].to", "to": ["nodes[*].name"] }
  ],
  "x-referencedBy": [
    { "target": "nodes[*].name", "from": ["edges[*].from", "edges[*].to"], "min": 1 }
  ]
}
```

**Data:**
```json
{
  "nodes": [
    { "name": "start" },
    { "name": "end" }
  ],
  "edges": [
    { "name": "connection", "from": "start", "to": "end" }
  ]
}
```

---

## Prerequisites

**macOS (Homebrew):**
```bash
brew install wasm-pack maturin
```

**Cross-platform (Cargo):**
```bash
cargo install wasm-pack maturin
```

## Building

### Core library
```bash
cargo build -p cohere-core
cargo test -p cohere-core
```

### WASM (for JavaScript)
```bash
cd bindings/wasm
wasm-pack build --target bundler  # For bundlers (webpack, vite, esbuild)
wasm-pack build --target web      # For standalone use (requires init())
```

Output: `bindings/wasm/pkg/`

### Python extension
```bash
cd bindings/python
maturin develop        # Install to current venv
maturin build --release  # Build wheel
```

### Swift package
```bash
./bindings/swift/Cohere/build-xcframework.sh   # Builds the Cohere.xcframework (macOS + iOS slices)
COHERE_LOCAL_XCFRAMEWORK=1 swift test          # Build/test against it (manifest is at the repo root)
```

This compiles the `cohere-swift` C FFI crate for each installed Apple Rust
target and packages it as an XCFramework. The Swift package manifest lives at
the repo root (`Package.swift`) so the package is installable directly from
GitHub. Set `COHERE_LOCAL_XCFRAMEWORK=1` for local development so the manifest
links the locally-built framework instead of a released one. Missing Rust
targets are added automatically (or skipped with a warning if offline). See
[bindings/swift/Cohere/README.md](bindings/swift/Cohere/README.md) for details.

## Releasing

All three bindings ship together in a **single GitHub Release** per version — no
npm or PyPI account needed. Run the *Release* workflow
(`.github/workflows/release.yml`) with a version like `0.2.0`. It:

1. stamps the version across every manifest (`scripts/set-version.sh`),
2. builds the Swift XCFramework, the Python wheels (abi3, one per OS) + an sdist,
   and the WASM npm tarball,
3. commits the version bump + the Swift checksum, tags `v0.2.0`, and
4. publishes one `v0.2.0` release with every artifact attached.

Consumers install directly from the release assets:

```bash
# Swift  — SwiftPM resolves the XCFramework via Package.swift (url + checksum)
.package(url: "https://github.com/jamesstidard/cohere", from: "0.2.0")

# Python — pip install the wheel for your platform (no PyPI)
pip install https://github.com/jamesstidard/cohere/releases/download/v0.2.0/cohere-0.2.0-cp38-abi3-macosx_11_0_arm64.whl

# JS/WASM — npm install the tarball (no npm registry)
npm install https://github.com/jamesstidard/cohere/releases/download/v0.2.0/cohere-wasm-0.2.0.tgz
```

## Usage

### JavaScript (with bundler)

When using a bundler (webpack, vite, esbuild), WASM is loaded automatically:

```javascript
import { Schema } from 'cohere-wasm';

const schema = new Schema({
  "x-references": [
    { "from": "organisations[*].members[*]", "to": ["users[*].name"] }
  ]
});

// JSON validation
const data = `{
  "users": [{"name": "alice"}, {"name": "bob"}],
  "organisations": [{"name": "acme", "members": ["alice", "bob"]}]
}`;

const result = schema.validateJson(data);
console.log(result.valid);  // true
console.log(result.errors); // []

// TOML validation
const toml = `
[[users]]
name = "alice"

[[users]]
name = "bob"

[[organisations]]
name = "acme"
members = ["alice", "bob"]
`;

const tomlResult = schema.validateToml(toml);
console.log(tomlResult.valid);  // true

// Errors include source locations (line/column)
for (const error of result.errors) {
  console.log(`${error.message} (line ${error.line}, col ${error.column})`);
}
```

### JavaScript (without bundler)

When using `wasm-pack build --target web`, you need to call `init()` first to load the WASM module:

```javascript
import init, { Schema } from './pkg/cohere_wasm.js';

await init();

const schema = new Schema({...});
const result = schema.validateJson(data);
```

### Python

```python
import cohere

schema = cohere.Schema({
    "x-references": [
        {"from": "organisations[*].members[*]", "to": ["users[*].name"]}
    ]
})

# JSON validation (with line/column in errors)
data_json = """
{
  "users": [
    {"name": "alice"}
  ],
  "organisations": [
    {"name": "acme", "members": ["alice"]}
  ]
}
"""
result = schema.validate_json(data_json)
print(result.valid)   # True

# TOML validation (with line/column in errors)
data_toml = """
[[users]]
name = "alice"

[[organisations]]
name = "acme"
members = ["alice"]
"""
result = schema.validate_toml(data_toml)
print(result.valid)   # True

# Errors include source locations
for error in result.errors:
    print(f"{error.message} (line {error.line}, col {error.column})")
```

### Swift

Add it to another project's `Package.swift` (the manifest is at the repo root,
so a plain GitHub URL works). Consumers need no Rust toolchain — the released
XCFramework is downloaded and checksum-verified:

```swift
dependencies: [
    .package(url: "https://github.com/jamesstidard/cohere", from: "0.1.0"),
],
targets: [
    .target(name: "YourTarget", dependencies: [
        .product(name: "Cohere", package: "cohere"),
    ]),
]
```

> Versioned installs require a published release (see *Releasing the Swift
> package* below). For a local checkout, use `.package(path: "/path/to/cohere")`
> with `COHERE_LOCAL_XCFRAMEWORK=1` set and the XCFramework built.

```swift
import Cohere

let schema = try Schema(json: #"""
{
  "x-references": [
    { "from": "organisations[*].members[*]", "to": ["users[*].name"] }
  ]
}
"""#)

// JSON validation (with line/column in errors)
let result = try schema.validate(json: #"""
{
  "users": [{"name": "alice"}],
  "organisations": [{"name": "acme", "members": ["alice"]}]
}
"""#)
print(result.valid)  // true

// TOML validation
let tomlResult = try schema.validate(toml: #"""
[[users]]
name = "alice"

[[organisations]]
name = "acme"
members = ["alice"]
"""#)
print(tomlResult.valid)  // true

// Errors include source locations
for error in result.errors {
    print("\(error.message) (line \(error.line ?? 0), col \(error.column ?? 0))")
}
```

## Error Message Placeholders

Custom messages support these placeholders:

| Placeholder | Description |
|-------------|-------------|
| `{{value}}` | The offending value |
| `{{path}}` | Full JSONPath to the value (e.g., `organisations[0].members[1]`) |
| `{{index}}` | Array index |

## Supported JSONPath Syntax

| Syntax | Example | Description |
|--------|---------|-------------|
| `field` | `users` | Object field access |
| `[*]` | `users[*]` | All array elements |
| `[n]` | `users[0]` | Specific array index |
| Chained | `organisations[*].members[*]` | Nested paths |

## Project Structure

```
cohere/
├── crates/
│   └── core/           # Core validation logic (pure Rust)
│       └── src/
│           ├── lib.rs
│           ├── jsonpath.rs     # JSONPath parser & evaluator
│           ├── schema.rs       # x- keyword parsing
│           ├── validate.rs     # Validation logic
│           ├── source_map.rs   # Shared source location types
│           ├── json.rs         # JSON source map builder
│           └── toml.rs         # TOML→JSON conversion with source map
├── bindings/           # Language bindings (one folder per language)
│   ├── wasm/           # WASM bindings (wasm-bindgen)
│   ├── python/         # Python bindings (PyO3)
│   └── swift/          # Swift binding
│       ├── ffi/        #   Rust C FFI crate (staticlib + C header)
│       └── Cohere/     #   SwiftPM package (wrapper, XCFramework build)
├── examples/           # Runnable examples (javascript, python, swift)
└── tests/fixtures/     # Test cases
```

## License

MIT
