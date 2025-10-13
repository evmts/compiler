#include "solidity-parser-wrapper.h"
#include "libsolidity/parsing/Parser.h"
#include "libsolidity/ast/AST.h"
#include "libsolidity/ast/ASTJsonExporter.h"
#include "liblangutil/ErrorReporter.h"
#include "liblangutil/Exceptions.h"
#include "liblangutil/CharStream.h"
#include "liblangutil/Scanner.h"
#include <memory>
#include <sstream>
#include <vector>
#include <cstring>

using namespace solidity;
using namespace solidity::langutil;
using namespace solidity::frontend;

struct SolParserContext {
    std::vector<std::shared_ptr<Error const>> errors;
    ErrorList errorList;
    std::unique_ptr<ErrorReporter> errorReporter;

    SolParserContext() {
        errorReporter = std::make_unique<ErrorReporter>(errorList);
    }
};

extern "C" {

SolParserContext* sol_parser_create(void) {
    return new SolParserContext();
}

void sol_parser_destroy(SolParserContext* ctx) {
    delete ctx;
}

char* sol_parser_parse(SolParserContext* ctx, const char* source, const char* source_name) {
    if (!ctx || !source || !source_name) {
        return nullptr;
    }

    try {
        // Clear previous errors
        ctx->errorList.clear();

        // Create CharStream from source
        std::string sourceStr(source);
        std::string nameStr(source_name);
        CharStream charStream(sourceStr, nameStr);

        // Create parser - this does ONLY syntax parsing, no semantic analysis
        Parser parser(*ctx->errorReporter, EVMVersion(), std::nullopt);

        // Parse the source - returns AST even if there are semantic errors!
        ASTPointer<SourceUnit> ast = parser.parse(charStream);

        if (!ast) {
            // Parsing failed (syntax error)
            return nullptr;
        }

        // Export AST to JSON
        std::map<std::string, unsigned> sourceIndices;
        sourceIndices[nameStr] = 0;

        bool exportFormatted = true;
        ASTJsonExporter exporter(sourceIndices);
        Json::Value json = exporter.toJson(*ast);

        // Convert JSON to string
        std::ostringstream oss;
        if (exportFormatted) {
            oss << json;
        } else {
            Json::StreamWriterBuilder builder;
            builder["indentation"] = "";
            std::unique_ptr<Json::StreamWriter> writer(builder.newStreamWriter());
            writer->write(json, &oss);
        }

        std::string result = oss.str();
        char* cResult = (char*)malloc(result.length() + 1);
        if (cResult) {
            std::strcpy(cResult, result.c_str());
        }
        return cResult;

    } catch (const FatalError& e) {
        // Fatal error during parsing
        return nullptr;
    } catch (const std::exception& e) {
        // Other error
        return nullptr;
    }
}

void sol_free_string(char* str) {
    if (str) {
        free(str);
    }
}

char* sol_parser_get_errors(SolParserContext* ctx) {
    if (!ctx) {
        return nullptr;
    }

    std::ostringstream oss;
    for (const auto& error : ctx->errorList) {
        oss << error->typeName() << ": " << error->what() << "\n";
    }

    std::string result = oss.str();
    if (result.empty()) {
        return nullptr;
    }

    char* cResult = (char*)malloc(result.length() + 1);
    if (cResult) {
        std::strcpy(cResult, result.c_str());
    }
    return cResult;
}

} // extern "C"
