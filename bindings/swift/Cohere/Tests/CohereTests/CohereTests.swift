import XCTest
@testable import Cohere

final class CohereTests: XCTestCase {
    private let schemaJSON = """
    {
        "type": "object",
        "properties": { "name": { "type": "string" } },
        "required": ["name"]
    }
    """

    func testValidJSON() throws {
        let schema = try Schema(json: schemaJSON)
        let result = try schema.validate(json: #"{"name": "alice"}"#)
        XCTAssertTrue(result.valid)
        XCTAssertTrue(result.errors.isEmpty)
    }

    func testInvalidJSONReportsErrors() throws {
        let schema = try Schema(json: schemaJSON)
        let result = try schema.validate(json: "{}")
        XCTAssertFalse(result.valid)
        XCTAssertFalse(result.errors.isEmpty)
    }

    func testValidTOML() throws {
        let schema = try Schema(json: schemaJSON)
        let result = try schema.validate(toml: #"name = "alice""#)
        XCTAssertTrue(result.valid)
    }

    func testMalformedJSONThrows() throws {
        let schema = try Schema(json: schemaJSON)
        XCTAssertThrowsError(try schema.validate(json: "{not json")) { error in
            guard case CohereError.invalidData = error else {
                return XCTFail("expected invalidData, got \(error)")
            }
        }
    }

    func testInvalidSchemaThrows() {
        XCTAssertThrowsError(try Schema(json: "{not json")) { error in
            guard case CohereError.invalidSchema = error else {
                return XCTFail("expected invalidSchema, got \(error)")
            }
        }
    }

    func testRelationalKeywords() throws {
        let relationalSchema = """
        {
            "type": "object",
            "x-references": [
                { "from": "organisations[*].members[*]", "to": ["users[*].name"] }
            ]
        }
        """
        let schema = try Schema(json: relationalSchema)

        let good = try schema.validate(json: #"""
        {"users": [{"name": "alice"}], "organisations": [{"name": "acme", "members": ["alice"]}]}
        """#)
        XCTAssertTrue(good.valid)

        let bad = try schema.validate(json: #"""
        {"users": [{"name": "alice"}], "organisations": [{"name": "acme", "members": ["bob"]}]}
        """#)
        XCTAssertFalse(bad.valid)
    }
}
