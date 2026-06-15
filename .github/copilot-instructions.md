# Copilot Instructions

Context for AI coding assistants (GitHub Copilot, Cursor, Claude, etc.).

## Project Overview

`cohere` extends JSON Schema with relational constraints for JSON "databases" — documents where entities reference each other by name.

**Example use case:**
```json
{
  "users": [{ "name": "alice" }, { "name": "bob" }],
  "organisations": [{ "name": "acme", "members": ["alice", "bob"] }]
}
```

Validate that:
- All names are unique
- `members` only contains valid user names
- Every user belongs to at least one organisation

## Architecture

```
crates/
└── core/           # Pure Rust validation logic (no platform dependencies)
bindings/
├── wasm/           # wasm-bindgen bindings (thin wrapper around core)
├── python/         # PyO3 bindings (thin wrapper around core)
└── swift/          # Swift binding
    ├── ffi/        #   Rust C FFI crate (thin wrapper around core)
    └── Cohere/     #   SwiftPM package (idiomatic Swift wrapper + XCFramework)
```

**Key principle:** All logic lives in `core`. The binding crates are thin wrappers for serialization and type conversion only.

## Core Crate Structure

- `lib.rs` — Public API exports
- `jsonpath.rs` — JSONPath parser and evaluator (supports `field`, `[*]`, `[n]`)
- `schema.rs` — Parses JSON Schema with custom `x-` keywords into typed Rust structs
- `validate.rs` — Runs validation rules against JSON data

## Custom Keywords

Three custom JSON Schema keywords (prefixed with `x-` per convention):

### `x-uniqueAcross`
```json
{ "paths": ["users[*].name", "organisations[*].name"], "message": "..." }
```
Ensures collected values are unique across all specified paths.

### `x-references`
```json
{ "from": "organisations[*].members[*]", "to": ["users[*].name"], "message": "..." }
```
Ensures every value at `from` exists in at least one `to` path. Like a foreign key.

### `x-referencedBy`
```json
{ "target": "users[*].name", "from": ["organisations[*].members[*]"], "min": 1, "message": "..." }
```
Ensures every value at `target` is referenced at least `min` times by values at `from` paths.

## Conventions

- **Error handling:** Use `thiserror` for error types in core
- **Serialization:** Use `serde` with `#[serde(rename_all = "camelCase")]` for JSON compatibility
- **JSONPath:** Custom minimal implementation — no external crate. Supports `field`, `[*]`, `[n]`, and chaining.
- **Testing:** Unit tests inline in each module via `#[cfg(test)]`
- **Naming:** Rust uses snake_case internally; JSON uses camelCase

## Build Commands

```bash
# Core
cargo build -p cohere-core
cargo test -p cohere-core

# WASM
cd bindings/wasm && wasm-pack build --target web

# Python
cd bindings/python && maturin develop

# Swift
./bindings/swift/Cohere/build-xcframework.sh
```

## When Adding Features

1. **Add to core first** — implement the logic in `crates/core/`
2. **Add tests** — inline `#[cfg(test)]` module
3. **Update wasm bindings** — if new public API, expose in `bindings/wasm/src/lib.rs`
4. **Update python bindings** — if new public API, expose in `bindings/python/src/lib.rs` (and Swift in `bindings/swift/ffi/src/lib.rs`)
5. **Add fixture tests** — JSON test cases in `tests/fixtures/valid/` and `tests/fixtures/invalid/`

## Common Tasks

### Add a new validation keyword

1. Define the raw/parsed structs in `schema.rs`
2. Add parsing logic in `Schema::parse_*` method
3. Add to `ValidationRule` enum
4. Implement validation function in `validate.rs`
5. Wire into `validate()` function
6. Add tests

### Extend JSONPath syntax

1. Add new `Segment` variant in `jsonpath.rs`
2. Update `parse()` to handle new syntax
3. Update `evaluate()` to process new segment type
4. Add tests

## Error Message Placeholders

When implementing custom error messages, support these placeholders:

- `{{value}}` — the offending value
- `{{path}}` — full JSONPath to the value
- `{{index}}` — array index (when applicable)

## Dependencies

Core dependencies (keep minimal):
- `serde` + `serde_json` — JSON handling
- `thiserror` — error types

WASM-specific:
- `wasm-bindgen` — JS bindings
- `serde-wasm-bindgen` — JS value conversion

Python-specific:
- `pyo3` — Python bindings
