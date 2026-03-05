// Basic usage example for cohere WASM bindings
// This file shows the core API usage without the HTML wrapper

import init, { Schema } from '../../crates/wasm/pkg/cohere_wasm.js';

// Initialize WASM module
await init();

// Example 1: Users and Organizations
console.log('=== Example 1: Users and Organizations ===\n');

const schema1 = new Schema({
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
});

const data1 = JSON.stringify({
  users: [
    { name: "alice", age: 32 },
    { name: "bob", age: 28 }
  ],
  organisations: [
    { name: "acme", members: ["alice", "bob"] }
  ]
});

const result1 = schema1.validateJson(data1);
console.log('Valid:', result1.valid);
console.log('Errors:', result1.errors);
console.log('');

// Example 2: Invalid Data (demonstrates error messages)
console.log('=== Example 2: Invalid Data ===\n');

const schema2 = new Schema({
  "x-references": [
    {
      "from": "organisations[*].members[*]",
      "to": ["users[*].name"],
      "message": "Unknown member '{{value}}' at {{path}}"
    }
  ]
});

const data2 = JSON.stringify({
  users: [
    { name: "alice" }
  ],
  organisations: [
    { name: "acme", members: ["alice", "charlie"] }
  ]
});

const result2 = schema2.validateJson(data2);
console.log('Valid:', result2.valid);
console.log('Errors:', result2.errors);
console.log('');

// Example 3: Graph Nodes and Edges
console.log('=== Example 3: Graph Nodes and Edges ===\n');

const schema3 = new Schema({
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
      "min": 1
    }
  ]
});

const data3 = JSON.stringify({
  nodes: [
    { name: "start" },
    { name: "end" }
  ],
  edges: [
    { name: "connection", from: "start", to: "end" }
  ]
});

const result3 = schema3.validateJson(data3);
console.log('Valid:', result3.valid);
console.log('Errors:', result3.errors);
console.log('');

// Example 4: Error Location (line/column)
console.log('=== Example 4: Error Location (line/column) ===\n');

const schema4 = new Schema({
  "x-references": [
    {
      "from": "organisations[*].members[*]",
      "to": ["users[*].name"],
      "message": "Unknown member '{{value}}'"
    }
  ]
});

// Multi-line JSON string to demonstrate line/column in errors
const data4 = `{
  "users": [
    {"name": "alice"}
  ],
  "organisations": [
    {"name": "acme", "members": ["alice", "charlie"]}
  ]
}`;

const result4 = schema4.validateJson(data4);
console.log('Valid:', result4.valid);
for (const error of result4.errors) {
  const location = error.line ? ` (line ${error.line}, col ${error.column})` : '';
  console.log(`  • ${error.message}${location}`);
}

// Example 5: TOML Validation (with line/column)
console.log('\n=== Example 5: TOML Validation ===\n');

const schema5 = new Schema({
  "x-references": [
    {
      "from": "organisations[*].members[*]",
      "to": ["users[*].name"],
      "message": "Unknown member '{{value}}'"
    }
  ]
});

const data5 = `
[[users]]
name = "alice"

[[organisations]]
name = "acme"
members = ["alice", "charlie"]
`;

const result5 = schema5.validateToml(data5);
console.log('Valid:', result5.valid);
for (const error of result5.errors) {
  const location = error.line ? ` (line ${error.line}, col ${error.column})` : '';
  console.log(`  • ${error.message}${location}`);
}
