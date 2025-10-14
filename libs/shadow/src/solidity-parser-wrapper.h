#ifndef SOLIDITY_PARSER_WRAPPER_H
#define SOLIDITY_PARSER_WRAPPER_H

#ifdef __cplusplus
extern "C" {
#endif

// Opaque types
typedef struct SolParserContext SolParserContext;
typedef struct SolAST SolAST;

// Create a new parser context
SolParserContext* sol_parser_create(void);

// Destroy parser context
void sol_parser_destroy(SolParserContext* ctx);

// Parse Solidity source code and return AST as JSON string (parsed, not analyzed)
// Returns NULL on failure
// Caller must free the returned string with sol_free_string
char* sol_parser_parse(SolParserContext* ctx, const char* source, const char* source_name);

// Analyze a single parsed AST JSON
// Takes a parsed AST JSON (from sol_parser_parse), runs full semantic analysis
// Returns fully analyzed AST JSON with type information, scope, references, etc.
// Returns NULL on failure. Caller must free with sol_free_string
char* sol_analyze_parsed_ast_json(
    SolParserContext* ctx,
    const char* parsed_ast_json,
    const char* source_name
);

// Free a string returned by the parser
void sol_free_string(char* str);

// Get error messages (if any)
char* sol_parser_get_errors(SolParserContext* ctx);

#ifdef __cplusplus
}
#endif

#endif // SOLIDITY_PARSER_WRAPPER_H
