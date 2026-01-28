// Basic usage example for graph-validator WASM bindings
// This file shows the core API usage without the HTML wrapper

import init, { validate_graph } from '../../crates/wasm/pkg/graph_validator_wasm.js';

// Initialize WASM module
await init();

// Example 1: Users and Organizations
console.log('=== Example 1: Users and Organizations ===\n');

const schema1 = JSON.stringify({
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

const result1 = validate_graph(schema1, data1);
console.log('Valid:', result1.valid);
console.log('Errors:', result1.errors);
console.log('');

// Example 2: Invalid Data (demonstrates error messages)
console.log('=== Example 2: Invalid Data ===\n');

const schema2 = JSON.stringify({
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

const result2 = validate_graph(schema2, data2);
console.log('Valid:', result2.valid);
console.log('Errors:', result2.errors);
console.log('');

// Example 3: Graph Nodes and Edges
console.log('=== Example 3: Graph Nodes and Edges ===\n');

const schema3 = JSON.stringify({
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

const result3 = validate_graph(schema3, data3);
console.log('Valid:', result3.valid);
console.log('Errors:', result3.errors);
