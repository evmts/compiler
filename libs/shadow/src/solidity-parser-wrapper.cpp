#include "solidity-parser-wrapper.h"
#include "libsolidity/parsing/Parser.h"
#include "libsolidity/ast/AST.h"
#include "libsolidity/ast/ASTJsonExporter.h"
#include "libsolidity/ast/ASTJsonImporter.h"
#include "libsolidity/analysis/Scoper.h"
#include "libsolidity/analysis/SyntaxChecker.h"
#include "libsolidity/analysis/GlobalContext.h"
#include "libsolidity/analysis/NameAndTypeResolver.h"
#include "libsolidity/analysis/DeclarationTypeChecker.h"
#include "libsolidity/analysis/ContractLevelChecker.h"
#include "libsolidity/analysis/TypeChecker.h"
#include "libsolidity/analysis/PostTypeChecker.h"
#include "libsolidity/analysis/PostTypeContractLevelChecker.h"
#include "libsolidity/analysis/DocStringTagParser.h"
#include "libsolidity/analysis/DocStringAnalyser.h"
#include "libsolidity/analysis/FunctionCallGraph.h"
#include "liblangutil/ErrorReporter.h"
#include "liblangutil/Exceptions.h"
#include "liblangutil/CharStream.h"
#include "liblangutil/Scanner.h"
#include "libsolutil/JSON.h"
#include <memory>
#include <sstream>
#include <vector>
#include <cstring>

using namespace solidity;
using namespace solidity::langutil;
using namespace solidity::frontend;

// ============================================================================
// FFI Types & Lifecycle
// ============================================================================

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

void sol_free_string(char* str) {
    if (str)
        free(str);
}

char* sol_parser_get_errors(SolParserContext* ctx) {
    if (!ctx)
        return nullptr;

    std::ostringstream oss;
    for (const auto& error : ctx->errorList) {
        oss << langutil::Error::formatErrorType(error->type()) << ": " << error->what() << "\n";
    }

    std::string result = oss.str();
    if (result.empty())
        return nullptr;

    char* cResult = (char*)malloc(result.length() + 1);
    if (cResult)
        std::strcpy(cResult, result.c_str());
    return cResult;
}

// ============================================================================
// Custom Analysis Pipeline
// ============================================================================
// WHY: Solidity's full CompilerStack is 1000+ lines with codegen, metadata,
//      optimization passes. We only need semantic analysis to restore AST
//      annotations (scope, type, referencedDeclaration) lost in JSON.
// DOES: Runs 13-step analysis pipeline from CompilerStack without codegen
// ============================================================================

namespace {

bool analyzeSourceUnit(
    SourceUnit& _ast,
    ErrorList& _errors,
    EVMVersion _evmVersion = EVMVersion{}
) {
    ErrorReporter errorReporter(_errors);
    bool noErrors = true;

    try {
        // Assign scopes to AST nodes
        Scoper::assignScopes(_ast);

        // Syntax checking
        SyntaxChecker syntaxChecker(errorReporter, false);
        if (!syntaxChecker.checkSyntax(_ast))
            noErrors = false;

        // Create global context (built-in types and functions)
        auto globalContext = std::make_shared<GlobalContext>(_evmVersion);

        // Name and type resolution
        NameAndTypeResolver resolver(*globalContext, _evmVersion, errorReporter, false);

        if (!resolver.registerDeclarations(_ast))
            return false;

        // For single-file analysis, no external imports
        std::map<std::string, SourceUnit const*> sourceUnits = {{"Contract.sol", &_ast}};
        if (!resolver.performImports(_ast, sourceUnits))
            return false;

        resolver.warnHomonymDeclarations();

        // Parse doc strings
        {
            DocStringTagParser docStringTagParser(errorReporter);
            if (!docStringTagParser.parseDocStrings(_ast))
                noErrors = false;
        }

        // Resolve names and types
        if (!resolver.resolveNamesAndTypes(_ast))
            return false;

        // Declaration type checking
        DeclarationTypeChecker declarationTypeChecker(errorReporter, _evmVersion);
        if (!declarationTypeChecker.check(_ast))
            return false;

        // Validate doc strings using types
        {
            DocStringTagParser docStringTagParser(errorReporter);
            if (!docStringTagParser.validateDocStringsUsingTypes(_ast))
                noErrors = false;
        }

        // Contract-level checks (inheritance, overrides, etc.)
        ContractLevelChecker contractLevelChecker(errorReporter);
        if (!contractLevelChecker.check(_ast))
            noErrors = false;

        // Type checker
        TypeChecker typeChecker(_evmVersion, std::nullopt, errorReporter);
        if (!typeChecker.checkTypeRequirements(_ast))
            noErrors = false;

        if (noErrors) {
            // Analyze doc strings
            DocStringAnalyser docStringAnalyser(errorReporter);
            if (!docStringAnalyser.analyseDocStrings(_ast))
                noErrors = false;
        }

        if (noErrors) {
            // Post-type checking
            PostTypeChecker postTypeChecker(errorReporter);
            if (!postTypeChecker.check(_ast))
                noErrors = false;
            if (!postTypeChecker.finalize())
                noErrors = false;
        }

        if (noErrors) {
            // Create and assign call graphs (required by PostTypeContractLevelChecker)
            for (auto* contract: ASTNode::filteredNodes<ContractDefinition>(_ast.nodes())) {
                ContractDefinitionAnnotation& annotation = contract->annotation();
                annotation.creationCallGraph = std::make_unique<CallGraph>(
                    FunctionCallGraphBuilder::buildCreationGraph(*contract)
                );
                annotation.deployedCallGraph = std::make_unique<CallGraph>(
                    FunctionCallGraphBuilder::buildDeployedGraph(*contract, **annotation.creationCallGraph)
                );
            }
        }

        if (noErrors) {
            // Post-type contract-level checks
            if (!PostTypeContractLevelChecker{errorReporter}.check(_ast))
                noErrors = false;
        }

    } catch (FatalError const&) {
        return false;
    } catch (std::exception const&) {
        return false;
    }

    return noErrors;
}

} // anonymous namespace

// ============================================================================
// Parsing Phase
// ============================================================================

char* sol_parser_parse(SolParserContext* ctx, const char* source, const char* source_name) {
    if (!ctx || !source || !source_name)
        return nullptr;

    try {
        ctx->errorList.clear();

        // Create CharStream from source
        std::string sourceStr(source);
        std::string nameStr(source_name);
        CharStream charStream(sourceStr, nameStr);

        // Parse (syntax only, no semantic analysis)
        Parser parser(*ctx->errorReporter, EVMVersion(), std::nullopt);
        ASTPointer<SourceUnit> ast = parser.parse(charStream);

        if (!ast)
            return nullptr;

        // Export AST to JSON
        std::map<std::string, unsigned> sourceIndices;
        sourceIndices[nameStr] = 0;

        ASTJsonExporter exporter(CompilerStack::State::Parsed, sourceIndices);
        Json json = exporter.toJson(*ast);

        // Convert JSON to string
        std::string result = solidity::util::jsonPrettyPrint(json);
        char* cResult = (char*)malloc(result.length() + 1);
        if (cResult)
            std::strcpy(cResult, result.c_str());
        return cResult;

    } catch (const FatalError&) {
        return nullptr;
    } catch (const std::exception&) {
        return nullptr;
    }
}

// ============================================================================
// Analysis Phase
// ============================================================================
// Imports JSON → runs full analysis pipeline → restores pointers → exports JSON

char* sol_analyze_parsed_ast_json(
    SolParserContext* ctx,
    const char* parsed_ast_json,
    const char* source_name
) {
    if (!ctx || !parsed_ast_json || !source_name)
        return nullptr;

    try {
        ctx->errorList.clear();
        std::string nameStr(source_name);

        // Parse JSON
        Json astJson = Json::parse(parsed_ast_json);

        // Import parsed AST from JSON to C++ AST
        std::map<std::string, Json> sources = {{nameStr, astJson}};
        ASTJsonImporter importer(EVMVersion{}, std::nullopt);
        auto asts = importer.jsonToSourceUnit(sources);

        if (asts.empty() || !asts[nameStr])
            return nullptr;

        ASTPointer<SourceUnit>& ast = asts[nameStr];

        // Run full semantic analysis
        if (!analyzeSourceUnit(*ast, ctx->errorList))
            return nullptr;

        // Export analyzed AST to JSON
        std::map<std::string, unsigned> sourceIndices = {{nameStr, 0}};
        ASTJsonExporter exporter(CompilerStack::State::AnalysisSuccessful, sourceIndices);
        Json analyzedJson = exporter.toJson(*ast);

        // Convert to string
        std::string result = solidity::util::jsonPrettyPrint(analyzedJson);
        char* cResult = (char*)malloc(result.length() + 1);
        if (cResult)
            std::strcpy(cResult, result.c_str());
        return cResult;

    } catch (std::exception const&) {
        return nullptr;
    } catch (...) {
        return nullptr;
    }
}

} // extern "C"
