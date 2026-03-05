# graph-validator

A portable JSON/TOML validator with custom JSON Schema extensions for relational constraints.

Write your validation logic once in Rust, then use it everywhere:
- **JavaScript/TypeScript** — via WebAssembly
- **Python** — via native extension (PyO3)

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
cargo build -p graph-validator-core
cargo test -p graph-validator-core
```

### WASM (for JavaScript)
```bash
cd crates/wasm
wasm-pack build --target web
```

Output: `crates/wasm/pkg/`

### Python extension
```bash
cd crates/python
maturin develop        # Install to current venv
maturin build --release  # Build wheel
```

## Usage

### JavaScript

```javascript
import init, { validate_graph } from './pkg/graph_validator_wasm.js';

await init();

const schema = JSON.stringify({
  "x-references": [
    { "from": "organisations[*].members[*]", "to": ["users[*].name"] }
  ]
});

const data = `{
  "users": [{"name": "alice"}, {"name": "bob"}],
  "organisations": [{"name": "acme", "members": ["alice", "bob"]}]
}`;

const result = validate_graph(schema, data);
console.log(result.valid);  // true
console.log(result.errors); // []

// Errors include source locations (line/column)
for (const error of result.errors) {
  console.log(`${error.message} (line ${error.line}, col ${error.column})`);
}
```

### Python

```python
import graph_validator
import json

schema = json.dumps({
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
result = graph_validator.validate_graph(schema, data_json)
print(result.valid)   # True

# TOML validation (with line/column in errors)
data_toml = """
[[users]]
name = "alice"

[[organisations]]
name = "acme"
members = ["alice"]
"""
result = graph_validator.validate_graph_toml(schema, data_toml)
print(result.valid)   # True

# Errors include source locations
for error in result.errors:
    print(f"{error.message} (line {error.line}, col {error.column})")

# Or use dicts directly (no source locations)
result = graph_validator.validate_graph_dict(schema_dict, data_dict)
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
graph-validator/
├── crates/
│   ├── core/           # Core validation logic (pure Rust)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── jsonpath.rs     # JSONPath parser & evaluator
│   │       ├── schema.rs       # x- keyword parsing
│   │       ├── validate.rs     # Validation logic
│   │       ├── source_map.rs   # Shared source location types
│   │       ├── json.rs         # JSON source map builder
│   │       └── toml.rs         # TOML→JSON conversion with source map
│   ├── wasm/           # WASM bindings (wasm-bindgen)
│   └── python/         # Python bindings (PyO3)
└── tests/fixtures/     # Test cases
```

## License

MIT
