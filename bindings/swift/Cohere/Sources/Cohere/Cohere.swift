import CCohere
import Foundation

/// A single schema violation.
public struct ValidationError: Codable, Equatable, Sendable {
    /// Human-readable description of the violation.
    public let message: String
    /// JSONPath to the offending value, when known.
    public let path: String?
    /// String form of the offending value, when known.
    public let value: String?
    /// 1-based line in the source document, when known.
    public let line: Int?
    /// 1-based column in the source document, when known.
    public let column: Int?
}

/// The outcome of validating a document against a ``Schema``.
public struct ValidationResult: Codable, Equatable, Sendable {
    /// `true` when the document satisfied the schema.
    public let valid: Bool
    /// The violations found, empty when ``valid`` is `true`.
    public let errors: [ValidationError]
}

/// Errors thrown when a schema or document cannot be processed.
///
/// These are distinct from schema *violations*, which are reported in a
/// successful ``ValidationResult`` with `valid == false`.
public enum CohereError: Error, Equatable {
    /// The schema string could not be parsed or compiled.
    case invalidSchema(String)
    /// The data string could not be parsed (malformed JSON/TOML).
    case invalidData(String)
    /// The result string could not be decoded (should not happen in practice).
    case decodingFailed(String)
}

/// A compiled schema that validates JSON and TOML documents.
///
/// Extends JSON Schema with the `x-uniqueAcross`, `x-references`, and
/// `x-referencedBy` keywords for relational constraints.
///
/// ```swift
/// let schema = try Schema(json: #"{"type":"object","required":["name"]}"#)
/// let result = try schema.validate(json: #"{"name":"alice"}"#)
/// print(result.valid) // true
/// ```
public final class Schema {
    private let handle: OpaquePointer

    /// Compile a schema from a JSON schema string.
    /// - Throws: ``CohereError/invalidSchema(_:)`` if the schema is invalid.
    public init(json schemaJSON: String) throws {
        var errorPtr: UnsafeMutablePointer<CChar>? = nil
        guard let handle = cohere_schema_new(schemaJSON, &errorPtr) else {
            throw CohereError.invalidSchema(Schema.takeString(errorPtr) ?? "unknown error")
        }
        self.handle = handle
    }

    deinit {
        cohere_schema_free(handle)
    }

    /// Validate a JSON document against this schema.
    /// - Throws: ``CohereError/invalidData(_:)`` if the JSON is malformed.
    public func validate(json: String) throws -> ValidationResult {
        try validate(data: json) { cohere_validate_json(handle, $0, $1) }
    }

    /// Validate a TOML document against this schema.
    /// - Throws: ``CohereError/invalidData(_:)`` if the TOML is malformed.
    public func validate(toml: String) throws -> ValidationResult {
        try validate(data: toml) { cohere_validate_toml(handle, $0, $1) }
    }

    private func validate(
        data: String,
        _ call: (UnsafePointer<CChar>, UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> UnsafeMutablePointer<CChar>?
    ) throws -> ValidationResult {
        var errorPtr: UnsafeMutablePointer<CChar>? = nil
        let resultPtr = data.withCString { call($0, &errorPtr) }
        guard let resultPtr else {
            throw CohereError.invalidData(Schema.takeString(errorPtr) ?? "unknown error")
        }
        guard let resultJSON = Schema.takeString(resultPtr) else {
            throw CohereError.decodingFailed("result was not valid UTF-8")
        }
        do {
            return try JSONDecoder().decode(ValidationResult.self, from: Data(resultJSON.utf8))
        } catch {
            throw CohereError.decodingFailed("\(error)")
        }
    }

    /// Copy a C string into Swift and free the original. Returns nil for a null pointer.
    private static func takeString(_ ptr: UnsafeMutablePointer<CChar>?) -> String? {
        guard let ptr else { return nil }
        defer { cohere_string_free(ptr) }
        return String(cString: ptr)
    }
}
