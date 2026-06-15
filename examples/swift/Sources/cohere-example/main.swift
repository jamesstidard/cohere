// Basic usage examples for the cohere Swift bindings.
//
// Build the XCFramework first, then run:
//     ../../bindings/swift/Cohere/build-xcframework.sh
//     COHERE_LOCAL_XCFRAMEWORK=1 swift run

import Cohere

func printResult(_ title: String, _ result: ValidationResult) {
    print("\n" + String(repeating: "=", count: 60))
    print(title)
    print(String(repeating: "=", count: 60))
    print("Valid: \(result.valid)")
    if !result.errors.isEmpty {
        print("Errors:")
        for error in result.errors {
            var location = ""
            if let line = error.line {
                location = " (line \(line), col \(error.column ?? 0))"
            }
            print("  • \(error.message)\(location)")
        }
    }
}

// Example 1: Users and organisations (all valid)
func example1UsersAndOrganisations() throws {
    let schema = try Schema(json: #"""
    {
      "x-uniqueAcross": [
        {
          "paths": ["users[*].name", "organisations[*].name"],
          "message": "Names must be unique. Duplicate: '{{value}}'"
        }
      ],
      "x-references": [
        {
          "from": "organisations[*].members[*]",
          "to": ["users[*].name"],
          "message": "Unknown member '{{value}}'"
        }
      ],
      "x-referencedBy": [
        {
          "target": "users[*].name",
          "from": ["organisations[*].members[*]"],
          "min": 1,
          "message": "User '{{value}}' is not in any organisation"
        }
      ]
    }
    """#)

    let data = #"""
    {
      "users": [
        {"name": "alice", "age": 32},
        {"name": "bob", "age": 28}
      ],
      "organisations": [
        {"name": "acme", "members": ["alice", "bob"]}
      ]
    }
    """#

    let result = try schema.validate(json: data)
    printResult("Example 1: Users and Organisations (JSON)", result)
}

// Example 2: Invalid data — unknown member and an orphaned user
func example2InvalidData() throws {
    let schema = try Schema(json: #"""
    {
      "x-references": [
        {
          "from": "organisations[*].members[*]",
          "to": ["users[*].name"],
          "message": "Unknown member '{{value}}' at {{path}}"
        }
      ],
      "x-referencedBy": [
        {
          "target": "users[*].name",
          "from": ["organisations[*].members[*]"],
          "min": 1,
          "message": "User '{{value}}' is not in any organisation"
        }
      ]
    }
    """#)

    let data = #"""
    {
      "users": [
        {"name": "alice", "age": 32},
        {"name": "orphan", "age": 25}
      ],
      "organisations": [
        {"name": "acme", "members": ["alice", "charlie"]}
      ]
    }
    """#

    let result = try schema.validate(json: data)
    printResult("Example 2: Invalid Data", result)
}

// Example 3: Graph nodes and edges
func example3GraphStructure() throws {
    let schema = try Schema(json: #"""
    {
      "x-uniqueAcross": [
        { "paths": ["nodes[*].name", "edges[*].name"] }
      ],
      "x-references": [
        { "from": "edges[*].from", "to": ["nodes[*].name"] },
        { "from": "edges[*].to", "to": ["nodes[*].name"] }
      ],
      "x-referencedBy": [
        {
          "target": "nodes[*].name",
          "from": ["edges[*].from", "edges[*].to"],
          "min": 1,
          "message": "Node '{{value}}' is not connected to any edges"
        }
      ]
    }
    """#)

    let data = #"""
    {
      "nodes": [
        {"name": "start"},
        {"name": "middle"},
        {"name": "end"}
      ],
      "edges": [
        {"name": "edge1", "from": "start", "to": "middle"},
        {"name": "edge2", "from": "middle", "to": "end"}
      ]
    }
    """#

    let result = try schema.validate(json: data)
    printResult("Example 3: Graph Nodes and Edges", result)
}

// Example 4: TOML validation (errors carry line/column)
func example4TomlValidation() throws {
    let schema = try Schema(json: #"""
    {
      "x-uniqueAcross": [
        {
          "paths": ["users[*].name"],
          "message": "Duplicate user name '{{value}}'"
        }
      ],
      "x-references": [
        {
          "from": "organisations[*].members[*]",
          "to": ["users[*].name"],
          "message": "Unknown member '{{value}}'"
        }
      ]
    }
    """#)

    let data = #"""
    [[users]]
    name = "alice"

    [[users]]
    name = "bob"

    [[organisations]]
    name = "acme"
    members = ["alice", "charlie"]
    """#

    let result = try schema.validate(toml: data)
    printResult("Example 4: TOML Validation (with line/column)", result)
}

// Example 5: Using the result in a condition
func example5UsingResult() throws {
    let schema = try Schema(json: #"""
    {
      "x-references": [
        { "from": "tags[*]", "to": ["valid_tags[*]"] }
      ]
    }
    """#)

    let validData = #"{"valid_tags": ["swift", "rust", "wasm"], "tags": ["swift", "rust"]}"#
    let invalidData = #"{"valid_tags": ["swift", "rust", "wasm"], "tags": ["swift", "javascript"]}"#

    print("\n" + String(repeating: "=", count: 60))
    print("Example 5: Branching on the Result")
    print(String(repeating: "=", count: 60))

    if try schema.validate(json: validData).valid {
        print("✓ Valid data passed validation")
    }

    let result = try schema.validate(json: invalidData)
    if !result.valid {
        print("✗ Invalid data failed validation")
        for error in result.errors {
            print("  • \(error.message)")
        }
    }
}

// Schema/document parse errors are thrown (distinct from schema violations).
func example6ParseError() {
    print("\n" + String(repeating: "=", count: 60))
    print("Example 6: Parse Errors Are Thrown")
    print(String(repeating: "=", count: 60))
    do {
        let schema = try Schema(json: #"{"x-references": []}"#)
        _ = try schema.validate(json: "{not valid json")
    } catch let error as CohereError {
        print("Caught CohereError: \(error)")
    } catch {
        print("Caught: \(error)")
    }
}

print("Cohere - Swift Examples")

do {
    try example1UsersAndOrganisations()
    try example2InvalidData()
    try example3GraphStructure()
    try example4TomlValidation()
    try example5UsingResult()
    example6ParseError()
} catch {
    print("Unexpected error: \(error)")
}

print("\n" + String(repeating: "=", count: 60))
print("All examples completed!")
print(String(repeating: "=", count: 60))
