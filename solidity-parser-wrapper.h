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

// Parse Solidity source code and return AST as JSON string
// Returns NULL on failure
// Caller must free the returned string with sol_free_string
char* sol_parser_parse(SolParserContext* ctx, const char* source, const char* source_name);

// Free a string returned by the parser
void sol_free_string(char* str);

// Get error messages (if any)
char* sol_parser_get_errors(SolParserContext* ctx);

#ifdef __cplusplus
}
#endif

#endif // SOLIDITY_PARSER_WRAPPER_H
