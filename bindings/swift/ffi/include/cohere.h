/*
 * cohere - C ABI for the Swift binding.
 *
 * A `CohereSchema` is compiled once from a JSON schema string, then used to
 * validate JSON or TOML data. Strings cross the boundary as UTF-8 C strings.
 *
 * Ownership: every non-null `char *` returned by these functions is owned by
 * the caller and must be released with `cohere_string_free`. Schemas must be
 * released with `cohere_schema_free`.
 */

#ifndef COHERE_H
#define COHERE_H

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque compiled schema. */
typedef struct CohereSchema CohereSchema;

/*
 * Compile a schema from a JSON schema string.
 *
 * Returns a non-null schema pointer on success. On failure returns NULL and,
 * if `error_out` is non-null, writes an owned error string to `*error_out`
 * (release with `cohere_string_free`).
 */
CohereSchema *cohere_schema_new(const char *schema_json, char **error_out);

/* Release a schema from `cohere_schema_new`. Passing NULL is a no-op. */
void cohere_schema_free(CohereSchema *schema);

/*
 * Validate a JSON data string against `schema`.
 *
 * Returns an owned JSON result string of the form
 *   {"valid": true,  "errors": []}
 *   {"valid": false, "errors": [{"message": "...", "path": "...",
 *                                "value": "...", "line": 1, "column": 2}, ...]}
 * (release with `cohere_string_free`). Returns NULL and writes to `error_out`
 * only when the input itself could not be parsed (malformed JSON / null schema);
 * schema violations appear inside the returned result string.
 */
char *cohere_validate_json(const CohereSchema *schema, const char *data_json, char **error_out);

/* Validate a TOML data string against `schema`. See `cohere_validate_json`. */
char *cohere_validate_toml(const CohereSchema *schema, const char *data_toml, char **error_out);

/* Release a string returned by any `cohere_*` function. Passing NULL is a no-op. */
void cohere_string_free(char *s);

#ifdef __cplusplus
}
#endif

#endif /* COHERE_H */
