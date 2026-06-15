# Swift Example

A runnable Swift program demonstrating the [cohere Swift bindings](../../bindings/swift/Cohere).

## Prerequisites

Build the XCFramework that the Swift package links against (from the repo root):

```bash
./bindings/swift/Cohere/build-xcframework.sh
```

This needs a Rust toolchain and Xcode command-line tools. It is only required
once (and again whenever the Rust core changes).

## Run

This example depends on the local checkout, so set `COHERE_LOCAL_XCFRAMEWORK=1`
to link the framework you just built (rather than a released one):

```bash
cd examples/swift
COHERE_LOCAL_XCFRAMEWORK=1 swift run
```

## What it shows

1. **Users and organisations** — `x-uniqueAcross`, `x-references`, and
   `x-referencedBy` on valid data.
2. **Invalid data** — an unknown member reference and an orphaned user.
3. **Graph structure** — node/edge relationships.
4. **TOML validation** — errors carry source line/column.
5. **Branching on the result** — using `result.valid` in a condition.
6. **Parse errors** — malformed input throws `CohereError`, distinct from
   schema violations (which appear in `result.errors`).
