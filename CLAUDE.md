# CLAUDE.md

Project context for Claude.

## What This Is

A portable JSON/TOML validator with relational constraints. Rust core with WASM and Python bindings.

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
cd crates/wasm && wasm-pack build --target web   # WASM
cd crates/python && maturin develop   # Python
```

## Where to Put Code

| What | Where |
|------|-------|
| Validation logic | `crates/core/src/validate.rs` |
| New JSON Schema keyword | `crates/core/src/schema.rs` |
| JSONPath extensions | `crates/core/src/jsonpath.rs` |
| JS bindings | `crates/wasm/src/lib.rs` |
| Python bindings | `crates/python/src/lib.rs` |
| Test fixtures | `tests/fixtures/valid/` or `tests/fixtures/invalid/` |

## Patterns

- All logic in `core`, bindings are thin wrappers
- Use `thiserror` for errors
- Inline tests with `#[cfg(test)]`
- JSON uses camelCase, Rust uses snake_case
- Error messages support `{{value}}`, `{{path}}`, `{{index}}` placeholders
