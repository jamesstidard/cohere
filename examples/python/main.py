#!/usr/bin/env python3
"""
Basic usage examples for graph-validator Python bindings.

Run this after installing the package:
    cd crates/python
    maturin develop
    cd ../../examples/python
    python example.py
"""

import graph_validator
import json


def print_result(title, result):
    """Helper to print validation results."""
    print(f"\n{'=' * 60}")
    print(f"{title}")
    print('=' * 60)
    print(f"Valid: {result.valid}")
    if result.errors:
        print("Errors:")
        for error in result.errors:
            print(f"  • {error}")
    print()


def example1_json_strings():
    """Example using JSON strings (validate_graph)."""
    schema = '''
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
    '''

    data = '''
    {
      "users": [
        {"name": "alice", "age": 32},
        {"name": "bob", "age": 28}
      ],
      "organisations": [
        {"name": "acme", "members": ["alice", "bob"]}
      ]
    }
    '''

    result = graph_validator.validate_graph(schema, data)
    print_result("Example 1: Users and Organizations (JSON Strings)", result)


def example2_python_dicts():
    """Example using Python dicts (validate_graph_dict)."""
    schema = {
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

    data = {
        "users": [
            {"name": "alice", "age": 32},
            {"name": "orphan", "age": 25}
        ],
        "organisations": [
            {"name": "acme", "members": ["alice", "charlie"]}
        ]
    }

    result = graph_validator.validate_graph_dict(schema, data)
    print_result("Example 2: Invalid Data (Python Dicts)", result)


def example3_graph_structure():
    """Example with node/edge graph structure."""
    schema = {
        "x-uniqueAcross": [
            {"paths": ["nodes[*].name", "edges[*].name"]}
        ],
        "x-references": [
            {"from": "edges[*].from", "to": ["nodes[*].name"]},
            {"from": "edges[*].to", "to": ["nodes[*].name"]}
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

    data = {
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

    result = graph_validator.validate_graph_dict(schema, data)
    print_result("Example 3: Graph Nodes and Edges", result)


def example4_toml_validation():
    """Example using TOML data (validate_graph_toml)."""
    schema = json.dumps({
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
    })

    data_toml = """\
[[users]]
name = "alice"

[[users]]
name = "bob"

[[organisations]]
name = "acme"
members = ["alice", "charlie"]
"""

    result = graph_validator.validate_graph_toml(schema, data_toml)
    print(f"\n{'=' * 60}")
    print("Example 4: TOML Validation (with line/column)")
    print('=' * 60)
    print(f"Valid: {result.valid}")
    if result.errors:
        print("Errors:")
        for error in result.errors:
            location = ""
            if error.line is not None:
                location = f" (line {error.line}, col {error.column})"
            print(f"  • {error.message}{location}")
    print()


def example5_using_bool():
    """Example showing truthiness of ValidationResult."""
    schema = {
        "x-references": [
            {"from": "tags[*]", "to": ["valid_tags[*]"]}
        ]
    }

    valid_data = {
        "valid_tags": ["python", "rust", "wasm"],
        "tags": ["python", "rust"]
    }

    invalid_data = {
        "valid_tags": ["python", "rust", "wasm"],
        "tags": ["python", "javascript"]  # javascript is not in valid_tags
    }

    # ValidationResult can be used in boolean context
    result1 = graph_validator.validate_graph_dict(schema, valid_data)
    if result1:
        print("\n✓ Valid data passed validation")

    result2 = graph_validator.validate_graph_dict(schema, invalid_data)
    if not result2:
        print("✗ Invalid data failed validation")
        print(f"  Errors: {result2.errors}")


if __name__ == "__main__":
    print("Graph Validator - Python Examples")

    example1_json_strings()
    example2_python_dicts()
    example3_graph_structure()
    example4_toml_validation()
    example5_using_bool()

    print("\n" + "=" * 60)
    print("All examples completed!")
    print("=" * 60)
