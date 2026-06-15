# CLAUDE.md

Project context for Claude.

## What This Is

A portable JSON/TOML validator with relational constraints. Rust core with WASM, Python, and Swift bindings.

**Use case:** JSON "databases" where entities reference each other:

```json
{
  "users": [{ "name": "alice" }],
  "organisations": [{ "name": "acme", "members": ["alice"] }]
}
```

Extends JSON Schema with three custom keywords:
- `x-uniqueAcross` — uniqueness across multiple JSONPaths
- `x-references` — foreign key validation
- `x-referencedBy` — ensure values are referenced

## Build

```bash
cargo test -p cohere-core   # Core tests
cd bindings/wasm && wasm-pack build --target web   # WASM
cd bindings/python && maturin develop   # Python
./bindings/swift/Cohere/build-xcframework.sh && COHERE_LOCAL_XCFRAMEWORK=1 swift test   # Swift (manifest at repo root)
```

## Where to Put Code

| What | Where |
|------|-------|
| Validation logic | `crates/core/src/validate.rs` |
| New JSON Schema keyword | `crates/core/src/schema.rs` |
| JSONPath extensions | `crates/core/src/jsonpath.rs` |
| JS bindings | `bindings/wasm/src/lib.rs` |
| Python bindings | `bindings/python/src/lib.rs` |
| Swift C FFI (Rust side) | `bindings/swift/ffi/src/lib.rs` + `bindings/swift/ffi/include/cohere.h` |
| Swift wrapper (Swift side) | `bindings/swift/Cohere/Sources/Cohere/Cohere.swift` |
| Swift package manifest | `Package.swift` (repo root — required for GitHub install) |
| Release tooling (all bindings) | `.github/workflows/release.yml` + `scripts/set-version.sh` |
| Test fixtures | `tests/fixtures/valid/` or `tests/fixtures/invalid/` |

## Patterns

- All logic in `core`, bindings are thin wrappers
- Use `thiserror` for errors
- Inline tests with `#[cfg(test)]`
- JSON uses camelCase, Rust uses snake_case
- Error messages support `{{value}}`, `{{path}}`, `{{index}}` placeholders
