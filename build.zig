const std = @import("std");

const DIST_WASM = "libs/shadow-ts/wasm";

// All Solidity sources for parser and semantic analysis
const solidity_sources = [_][]const u8{
    "solidity/liblangutil/CharStream.cpp",
    "solidity/liblangutil/DebugInfoSelection.cpp",
    "solidity/liblangutil/ErrorReporter.cpp",
    "solidity/liblangutil/EVMVersion.cpp",
    "solidity/liblangutil/Exceptions.cpp",
    "solidity/liblangutil/ParserBase.cpp",
    "solidity/liblangutil/Scanner.cpp",
    "solidity/liblangutil/SemVerHandler.cpp",
    "solidity/liblangutil/SourceLocation.cpp",
    "solidity/liblangutil/Token.cpp",
    "solidity/libsolidity/parsing/DocStringParser.cpp",
    "solidity/libsolidity/parsing/Parser.cpp",
    "solidity/libsolidity/ast/AST.cpp",
    "solidity/libsolidity/ast/ASTAnnotations.cpp",
    "solidity/libsolidity/ast/ASTJsonExporter.cpp",
    "solidity/libsolidity/ast/ASTJsonImporter.cpp",
    "solidity/libsolidity/ast/ASTUtils.cpp",
    "solidity/libsolidity/ast/CallGraph.cpp",
    "solidity/libsolidity/ast/Types.cpp",
    "solidity/libsolidity/ast/TypeProvider.cpp",
    "solidity/libsolidity/interface/Version.cpp",
    "solidity/libsolidity/analysis/ConstantEvaluator.cpp",
    "solidity/libsolidity/analysis/ContractLevelChecker.cpp",
    "solidity/libsolidity/analysis/ControlFlowAnalyzer.cpp",
    "solidity/libsolidity/analysis/ControlFlowBuilder.cpp",
    "solidity/libsolidity/analysis/ControlFlowGraph.cpp",
    "solidity/libsolidity/analysis/FunctionCallGraph.cpp",
    "solidity/libsolidity/analysis/DeclarationContainer.cpp",
    "solidity/libsolidity/analysis/DeclarationTypeChecker.cpp",
    "solidity/libsolidity/analysis/DocStringAnalyser.cpp",
    "solidity/libsolidity/analysis/DocStringTagParser.cpp",
    "solidity/libsolidity/analysis/GlobalContext.cpp",
    "solidity/libsolidity/analysis/ImmutableValidator.cpp",
    "solidity/libsolidity/analysis/NameAndTypeResolver.cpp",
    "solidity/libsolidity/analysis/PostTypeChecker.cpp",
    "solidity/libsolidity/analysis/PostTypeContractLevelChecker.cpp",
    "solidity/libsolidity/analysis/ReferencesResolver.cpp",
    "solidity/libsolidity/analysis/Scoper.cpp",
    "solidity/libsolidity/analysis/StaticAnalyzer.cpp",
    "solidity/libsolidity/analysis/SyntaxChecker.cpp",
    "solidity/libsolidity/analysis/TypeChecker.cpp",
    "solidity/libsolidity/analysis/ViewPureChecker.cpp",
    "solidity/libsolutil/CommonData.cpp",
    "solidity/libsolutil/CommonIO.cpp", // Excluded for WASM
    "solidity/libsolutil/Exceptions.cpp",
    "solidity/libsolutil/JSON.cpp",
    "solidity/libsolutil/Keccak256.cpp",
    "solidity/libsolutil/Numeric.cpp",
    "solidity/libsolutil/StringUtils.cpp",
    "solidity/libsolutil/UTF8.cpp",
    "solidity/libsolutil/Whiskers.cpp",
    "solidity/libyul/AsmParser.cpp",
    "solidity/libyul/AsmPrinter.cpp",
    "solidity/libyul/AsmJsonConverter.cpp",
    "solidity/libyul/Dialect.cpp",
    "solidity/libyul/Utilities.cpp",
    "solidity/libyul/AST.cpp",
    "solidity/libyul/Object.cpp",
    "solidity/libyul/ObjectParser.cpp",
    "solidity/libyul/backends/evm/EVMDialect.cpp",
    "solidity/libyul/backends/evm/EVMBuiltins.cpp",
    "solidity/libyul/backends/evm/EVMObjectCompiler.cpp",
    "solidity/libyul/AsmAnalysis.cpp",
    "solidity/libyul/AsmJsonImporter.cpp",
    "solidity/libyul/optimiser/ASTWalker.cpp",
    "solidity/libyul/optimiser/Semantics.cpp",
    "solidity/libyul/optimiser/CallGraphGenerator.cpp",
    "solidity/libyul/Scope.cpp",
    "solidity/libyul/ScopeFiller.cpp",
    "solidity/libsolidity/analysis/OverrideChecker.cpp",
    "solidity/libevmasm/Instruction.cpp",
    "solidity/libevmasm/SemanticInformation.cpp",
    "solidity/libsolidity/codegen/ContractCompiler.cpp", // Excluded for WASM
    "solidity/deps/fmtlib/src/format.cc",
};

// Files excluded from WASM (Boost.Filesystem dependencies)
const wasm_excluded = [_][]const u8{
    "solidity/libsolutil/CommonIO.cpp",
    "solidity/libsolidity/codegen/ContractCompiler.cpp",
};

fn isExcludedForWasm(file: []const u8) bool {
    for (wasm_excluded) |excluded| {
        if (std.mem.eql(u8, file, excluded)) return true;
    }
    return false;
}

fn addSolidityIncludes(lib: *std.Build.Step.Compile, b: *std.Build, buildinfo_dir: std.Build.LazyPath) void {
    lib.addIncludePath(buildinfo_dir);
    lib.addIncludePath(b.path("solidity"));
    lib.addIncludePath(b.path("solidity/libsolidity"));
    lib.addIncludePath(b.path("solidity/liblangutil"));
    lib.addIncludePath(b.path("solidity/libsolutil"));
    lib.addIncludePath(b.path("solidity/libevmasm"));
    lib.addIncludePath(b.path("solidity/deps/nlohmann-json/include"));
    lib.addIncludePath(b.path("solidity/deps/range-v3/include"));
    lib.addIncludePath(b.path("solidity/deps/fmtlib/include"));
    lib.addSystemIncludePath(.{ .cwd_relative = "/opt/homebrew/opt/boost/include" });
}

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const cpp_flags = &.{ "-std=c++20", "-Wno-deprecated", "-Wno-deprecated-declarations" };

    // Initialize submodules
    const init_submodules = b.addSystemCommand(&.{ "git", "submodule", "update", "--init", "--recursive" });

    // Generate BuildInfo.h
    const gen_buildinfo = b.addWriteFiles();
    _ = gen_buildinfo.add("solidity/BuildInfo.h",
        \\#pragma once
        \\#define ETH_PROJECT_VERSION "0.8.31"
        \\#define ETH_PROJECT_VERSION_MAJOR 0
        \\#define ETH_PROJECT_VERSION_MINOR 8
        \\#define ETH_PROJECT_VERSION_PATCH 31
        \\#define ETH_COMMIT_HASH "zig-build"
        \\#define ETH_BUILD_TYPE "Release"
        \\#define ETH_BUILD_PLATFORM "zig"
        \\#define SOL_VERSION_PRERELEASE ""
        \\#define SOL_VERSION_BUILDINFO ""
        \\#define SOL_VERSION_COMMIT ""
        \\
    );

    // Native C++ wrapper
    const native_wrapper = b.addLibrary(.{
        .name = "solidity-parser-wrapper",
        .linkage = .static,
        .root_module = b.createModule(.{ .target = target, .optimize = optimize }),
    });
    native_wrapper.step.dependOn(&init_submodules.step);
    native_wrapper.step.dependOn(&gen_buildinfo.step);
    native_wrapper.addCSourceFile(.{ .file = b.path("libs/shadow/src/solidity-parser-wrapper.cpp"), .flags = cpp_flags });
    for (solidity_sources) |src| {
        native_wrapper.addCSourceFile(.{ .file = b.path(src), .flags = cpp_flags });
    }
    addSolidityIncludes(native_wrapper, b, gen_buildinfo.getDirectory());
    native_wrapper.linkLibCpp();

    // Native Zig parser
    const native_parser = b.addLibrary(.{
        .name = "shadow-parser",
        .linkage = .static,
        .root_module = b.createModule(.{
            .root_source_file = b.path("libs/shadow/api.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    native_parser.linkLibrary(native_wrapper);
    native_parser.linkLibCpp();
    native_parser.addIncludePath(b.path("libs/shadow/src"));
    b.getInstallStep().dependOn(&b.addInstallArtifact(native_parser, .{}).step);

    // Tests
    const tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("libs/shadow/test/root.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    tests.root_module.addImport("shadow", native_parser.root_module);
    tests.linkLibrary(native_wrapper);
    tests.linkLibCpp();
    tests.addIncludePath(b.path("libs/shadow/src"));
    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&b.addRunArtifact(tests).step);

    // WASM: Build Zig code to static library (WASM object files)
    const wasm_target = b.resolveTargetQuery(.{ .cpu_arch = .wasm32, .os_tag = .wasi });
    const zig_wasm_lib = b.addLibrary(.{
        .name = "shadow-wasm",
        .linkage = .static,
        .root_module = b.createModule(.{
            .root_source_file = b.path("libs/shadow/api_wasm.zig"),
            .target = wasm_target,
            .optimize = .ReleaseSmall,
        }),
    });
    zig_wasm_lib.step.dependOn(&init_submodules.step);
    zig_wasm_lib.addIncludePath(b.path("libs/shadow/src"));

    // Install Zig WASM library for Emscripten to link
    const install_zig_lib = b.addInstallArtifact(zig_wasm_lib, .{});

    // WASM: Create output and buildinfo directories
    const mkdir_dist_wasm = b.addSystemCommand(&.{ "mkdir", "-p", DIST_WASM });

    const wasm_buildinfo_dir = "zig-out/wasm-buildinfo";
    const mkdir_buildinfo = b.addSystemCommand(&.{ "mkdir", "-p", wasm_buildinfo_dir ++ "/solidity" });
    const copy_buildinfo = b.addSystemCommand(&.{ "sh", "-c", "echo '#pragma once\n#define ETH_PROJECT_VERSION \"0.8.31\"\n#define ETH_PROJECT_VERSION_MAJOR 0\n#define ETH_PROJECT_VERSION_MINOR 8\n#define ETH_PROJECT_VERSION_PATCH 31\n#define ETH_COMMIT_HASH \"zig-build\"\n#define ETH_BUILD_TYPE \"Release\"\n#define ETH_BUILD_PLATFORM \"zig\"\n#define SOL_VERSION_PRERELEASE \"\"\n#define SOL_VERSION_BUILDINFO \"\"\n#define SOL_VERSION_COMMIT \"\"' > " ++ wasm_buildinfo_dir ++ "/solidity/BuildInfo.h" });
    copy_buildinfo.step.dependOn(&mkdir_buildinfo.step);

    // WASM: Incremental compilation - compile each C++ file to .o separately
    const wasm_obj_dir = "zig-out/wasm-obj";
    const mkdir_obj = b.addSystemCommand(&.{ "mkdir", "-p", wasm_obj_dir });
    mkdir_obj.step.dependOn(&init_submodules.step);
    mkdir_obj.step.dependOn(&copy_buildinfo.step);

    // Common emcc compile flags with ccache wrapper for incremental build caching
    const emcc_compile_flags = [_][]const u8{
        "ccache",
        "emcc",
        "-c",
        "-std=c++20",
        "-O3",
        "-I",
        "libs/shadow/src",
        "-I",
        "solidity",
        "-I",
        "solidity/libsolidity",
        "-I",
        "solidity/liblangutil",
        "-I",
        "solidity/libsolutil",
        "-I",
        "solidity/libevmasm",
        "-I",
        "solidity/deps/nlohmann-json/include",
        "-I",
        "solidity/deps/range-v3/include",
        "-I",
        "solidity/deps/fmtlib/include",
        "-I",
        "/opt/homebrew/opt/boost/include",
        "-I",
        wasm_buildinfo_dir,
    };

    // Compile Emscripten wrapper
    const api_emscripten_obj = b.fmt("{s}/api_emscripten.o", .{wasm_obj_dir});
    const compile_api_emscripten = b.addSystemCommand(&emcc_compile_flags);
    compile_api_emscripten.step.dependOn(&mkdir_obj.step);
    compile_api_emscripten.addArgs(&.{ "libs/shadow/api_emscripten.cpp", "-o", api_emscripten_obj });

    // Compile C++ wrapper
    const wrapper_obj = b.fmt("{s}/solidity-parser-wrapper.o", .{wasm_obj_dir});
    const compile_wrapper = b.addSystemCommand(&emcc_compile_flags);
    compile_wrapper.step.dependOn(&mkdir_obj.step);
    compile_wrapper.addArgs(&.{ "libs/shadow/src/solidity-parser-wrapper.cpp", "-o", wrapper_obj });

    // WASM: Link all object files using shell glob
    const emscripten_link = b.addSystemCommand(&.{
        "sh",
        "-c",
        b.fmt("emcc -std=c++20 -O3 -s WASM=1 -s MODULARIZE=1 -s EXPORT_ES6=1 -s ALLOW_MEMORY_GROWTH=1 -s INITIAL_MEMORY=16MB -s MAXIMUM_MEMORY=256MB -s \"EXPORTED_RUNTIME_METHODS=['cwrap','ccall']\" -s ERROR_ON_UNDEFINED_SYMBOLS=0 -s NO_DISABLE_EXCEPTION_CATCHING=1 -s STANDALONE_WASM=0 -sASSERTIONS=1 --bind --emit-tsd shadow.d.ts zig-out/lib/libshadow-wasm.a {s}/*.o -o {s}/shadow.js", .{ wasm_obj_dir, DIST_WASM }),
    });
    emscripten_link.step.dependOn(&install_zig_lib.step);
    emscripten_link.step.dependOn(&mkdir_dist_wasm.step);
    emscripten_link.step.dependOn(&compile_api_emscripten.step);
    emscripten_link.step.dependOn(&compile_wrapper.step);

    // Compile each Solidity source file separately and make link depend on each
    for (solidity_sources) |src| {
        if (isExcludedForWasm(src)) continue;

        // Create unique object file name by replacing path separators and removing extension
        const src_normalized = std.mem.replaceOwned(u8, b.allocator, src, "/", "_") catch @panic("OOM");
        // Remove file extension (.cpp, .cc, etc.)
        const last_dot = std.mem.lastIndexOfScalar(u8, src_normalized, '.') orelse src_normalized.len;
        const obj_name = b.fmt("{s}/{s}.o", .{ wasm_obj_dir, src_normalized[0..last_dot] });
        const compile_step = b.addSystemCommand(&emcc_compile_flags);
        compile_step.step.dependOn(&mkdir_obj.step);
        compile_step.addArgs(&.{ src, "-o", obj_name });

        // Make link step depend on this compilation
        emscripten_link.step.dependOn(&compile_step.step);
    }

    const wasm_step = b.step("wasm", "Build WASM module with Emscripten");
    wasm_step.dependOn(&emscripten_link.step);

    // All
    const all_step = b.step("all", "Build everything");
    all_step.dependOn(b.getInstallStep());
    all_step.dependOn(wasm_step);

    // Clean
    const clean_step = b.step("clean", "Remove build artifacts");
    clean_step.dependOn(&b.addSystemCommand(&.{ "rm", "-rf", "zig-out", "zig-cache" }).step);
}
