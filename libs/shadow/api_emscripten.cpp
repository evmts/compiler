#include <emscripten/bind.h>
#include <emscripten/val.h>
#include <string>
#include <cstring>

// Declare Zig-exported functions (implemented in api_wasm.zig)
extern "C" {
    const char* shadow_parse_source(const char* source_ptr, size_t source_len, const char* name_ptr, size_t name_len);
    void* shadow_init(const char* source_ptr, size_t source_len);
    void shadow_deinit(void* shadow);
    const char* shadow_stitch_into_source(void* shadow, const char* target_ptr, size_t target_len,
                                          const char* source_name_ptr, size_t source_name_len,
                                          const char* contract_name_ptr, size_t contract_name_len);
    const char* shadow_stitch_into_ast(void* shadow, const char* target_ast_ptr, size_t target_ast_len,
                                       const char* contract_name_ptr, size_t contract_name_len);
    void shadow_free_string(const char* ptr);
}

// Thin JavaScript-friendly wrappers (no logic, just marshaling)
namespace shadow {

class Shadow {
private:
    void* handle;

public:
    Shadow(const std::string& source) {
        handle = shadow_init(source.c_str(), source.length());
    }

    ~Shadow() {
        if (handle) shadow_deinit(handle);
    }

    static std::string parseSource(const std::string& source, const std::string& name) {
        const char* result = shadow_parse_source(
            source.c_str(), source.length(),
            name.empty() ? nullptr : name.c_str(), name.length()
        );
        if (!result) return "";
        std::string output(result);
        shadow_free_string(result);
        return output;
    }

    std::string stitchIntoSource(const std::string& target, const std::string& sourceName, const std::string& contractName) {
        const char* result = shadow_stitch_into_source(
            handle,
            target.c_str(), target.length(),
            sourceName.empty() ? nullptr : sourceName.c_str(), sourceName.length(),
            contractName.empty() ? nullptr : contractName.c_str(), contractName.length()
        );
        if (!result) return "";
        std::string output(result);
        shadow_free_string(result);
        return output;
    }

    std::string stitchIntoAst(const std::string& targetAst, const std::string& contractName) {
        const char* result = shadow_stitch_into_ast(
            handle,
            targetAst.c_str(), targetAst.length(),
            contractName.empty() ? nullptr : contractName.c_str(), contractName.length()
        );
        if (!result) return "";
        std::string output(result);
        shadow_free_string(result);
        return output;
    }
};

} // namespace shadow

// Emscripten bindings
EMSCRIPTEN_BINDINGS(shadow) {
    emscripten::class_<shadow::Shadow>("Shadow")
        .constructor<const std::string&>()
        .class_function("parseSource", &shadow::Shadow::parseSource)
        .function("stitchIntoSource", &shadow::Shadow::stitchIntoSource)
        .function("stitchIntoAst", &shadow::Shadow::stitchIntoAst);
}
