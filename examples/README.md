# Cohere Examples

This directory contains working examples demonstrating how to use cohere in JavaScript, Python, and Swift.

## Prerequisites

### For JavaScript Examples

1. Build the WASM package:
   ```bash
   cd bindings/wasm
   wasm-pack build --target web
   cd ../..
   ```

### For Python Examples

1. Install the Python package:
   ```bash
   cd bindings/python
   maturin develop
   cd ../..
   ```

### For Swift Examples

1. Build the XCFramework the Swift package links against (run from the repo root):
   ```bash
   ./bindings/swift/Cohere/build-xcframework.sh
   ```

## JavaScript Examples

### Interactive Browser Example

Open [`javascript/index.html`](javascript/index.html) in a web browser:

```bash
# macOS
open examples/javascript/index.html

# Linux
xdg-open examples/javascript/index.html

# Windows
start examples/javascript/index.html
```

**Note:** You may need to serve the files via HTTP due to CORS restrictions. You can use a simple HTTP server:

```bash
# Using Python
python3 -m http.server 8000

# Using Node.js (if you have http-server installed)
npx http-server .

# Then open: http://localhost:8000/examples/javascript/index.html
```

The interactive example demonstrates:
- ✓ Valid users and organizations
- ✓ Valid graph structure with nodes and edges
- ✗ Invalid data showing error messages

### Standalone JavaScript Example

Run the standalone example with Node.js or Deno:

```bash
# With Node.js (requires --experimental-modules for ES modules)
node examples/javascript/example.js

# With Deno
deno run examples/javascript/example.js
```

## Python Examples

Run the Python example:

```bash
python examples/python/example.py
# or
python3 examples/python/example.py
```

The example demonstrates:
1. **JSON strings** - Using `validate_graph()` with JSON string inputs
2. **Python dicts** - Using `validate_graph_dict()` with native Python dictionaries
3. **Graph structure** - Validating node/edge relationships
4. **Boolean context** - Using `ValidationResult` in if statements

## Swift Examples

Run the Swift example with Swift Package Manager:

```bash
cd examples/swift
swift run
```

The example demonstrates:
1. **Users and organisations** - `x-uniqueAcross`, `x-references`, `x-referencedBy`
2. **Invalid data** - unknown reference and orphaned entity errors
3. **Graph structure** - validating node/edge relationships
4. **TOML validation** - errors carry source line/column
5. **Branching on the result** - using `result.valid` in a condition
6. **Parse errors** - malformed input throws `CohereError`, distinct from
   schema violations

See [`swift/README.md`](swift/README.md) for details.

## What the Examples Show

All examples demonstrate the three custom JSON Schema keywords:

### `x-uniqueAcross`
Ensures values are unique across multiple JSONPaths.

```python
{
  "x-uniqueAcross": [
    {
      "paths": ["users[*].name", "organisations[*].name"],
      "message": "Names must be unique. Duplicate: '{{value}}'"
    }
  ]
}
```

### `x-references`
Validates foreign key relationships - ensures values at `from` exist in `to`.

```python
{
  "x-references": [
    {
      "from": "organisations[*].members[*]",
      "to": ["users[*].name"],
      "message": "Unknown member '{{value}}'"
    }
  ]
}
```

### `x-referencedBy`
Ensures target values are referenced by other values (minimum/maximum times).

```python
{
  "x-referencedBy": [
    {
      "target": "users[*].name",
      "from": ["organisations[*].members[*]"],
      "min": 1,
      "message": "User '{{value}}' is not in any organisation"
    }
  ]
}
```

## Error Messages

All custom keywords support message templates with placeholders:

- `{{value}}` - The offending value
- `{{path}}` - Full JSONPath to the value (e.g., `organisations[0].members[1]`)
- `{{index}}` - Array index

## Next Steps

- Read the main [README](../README.md) for more details
- Explore the [test fixtures](../tests/fixtures/) for more examples
- Check out the [source code](../crates/core/src/) to understand the implementation
