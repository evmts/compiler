const std = @import("std");

// All Solidity sources for parser
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
    "solidity/libsolidity/ast/ASTUtils.cpp",
    "solidity/libsolidity/ast/Types.cpp",
    "solidity/libsolidity/ast/TypeProvider.cpp",
    "solidity/libsolidity/interface/Version.cpp",
    "solidity/libsolidity/analysis/ConstantEvaluator.cpp",
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
    native_wrapper.addCSourceFile(.{ .file = b.path("solidity-parser-wrapper.cpp"), .flags = cpp_flags });
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
            .root_source_file = b.path("shadow.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    native_parser.linkLibrary(native_wrapper);
    native_parser.linkLibCpp();
    native_parser.addIncludePath(b.path("."));
    b.getInstallStep().dependOn(&b.addInstallArtifact(native_parser, .{}).step);

    // Tests (inline in shadow.zig)
    const tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("shadow.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    tests.linkLibrary(native_wrapper);
    tests.linkLibCpp();
    tests.addIncludePath(b.path("."));
    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&b.addRunArtifact(tests).step);

    // WASM: stub CommonIO.h (no Boost.Filesystem)
    const wasm_headers = b.addWriteFiles();
    _ = wasm_headers.add("libsolutil/CommonIO.h",
        \\#pragma once
        \\#include <string>
        \\#include <iosfwd>
        \\namespace solidity::util {
        \\std::string readFileAsString(std::string const&);
        \\std::string readUntilEnd(std::istream&);
        \\std::string readBytes(std::istream&, size_t);
        \\int readStandardInputChar();
        \\template<typename T> inline std::string toString(T const& t) { return std::to_string(t); }
        \\std::string absolutePath(std::string const&, std::string const&);
        \\std::string sanitizePath(std::string const&);
        \\}
        \\
    );

    // WASM C++ wrapper
    const wasm_target = b.resolveTargetQuery(.{ .cpu_arch = .wasm32, .os_tag = .wasi });
    const wasm_wrapper = b.addLibrary(.{
        .name = "solidity-parser-wrapper-wasm",
        .linkage = .static,
        .root_module = b.createModule(.{ .target = wasm_target, .optimize = .ReleaseSmall }),
    });
    wasm_wrapper.step.dependOn(&init_submodules.step);
    wasm_wrapper.step.dependOn(&gen_buildinfo.step);
    wasm_wrapper.step.dependOn(&wasm_headers.step);
    wasm_wrapper.addCSourceFile(.{ .file = b.path("solidity-parser-wrapper.cpp"), .flags = cpp_flags });
    for (solidity_sources) |src| {
        if (!isExcludedForWasm(src)) {
            wasm_wrapper.addCSourceFile(.{ .file = b.path(src), .flags = cpp_flags });
        }
    }
    wasm_wrapper.addIncludePath(wasm_headers.getDirectory()); // Override CommonIO.h
    addSolidityIncludes(wasm_wrapper, b, gen_buildinfo.getDirectory());
    wasm_wrapper.linkLibCpp();

    // WASM Zig parser
    const wasm_parser = b.addLibrary(.{
        .name = "shadow-parser-wasm",
        .linkage = .static,
        .root_module = b.createModule(.{
            .root_source_file = b.path("shadow.zig"),
            .target = wasm_target,
            .optimize = .ReleaseSmall,
        }),
    });
    wasm_parser.linkLibrary(wasm_wrapper);
    wasm_parser.addIncludePath(b.path("."));
    const wasm_step = b.step("wasm", "Build WASM parser");
    wasm_step.dependOn(&b.addInstallArtifact(wasm_parser, .{}).step);

    // TypeScript bindings
    const ts_gen = b.addSystemCommand(&.{ "node", "scripts/generate-ts-bindings.js" });
    ts_gen.step.dependOn(wasm_step);
    const ts_step = b.step("typescript", "Generate TypeScript bindings");
    ts_step.dependOn(&ts_gen.step);

    // All
    const all_step = b.step("all", "Build everything");
    all_step.dependOn(b.getInstallStep());
    all_step.dependOn(wasm_step);
    all_step.dependOn(&ts_gen.step);

    // Clean
    const clean_step = b.step("clean", "Remove build artifacts");
    clean_step.dependOn(&b.addSystemCommand(&.{ "rm", "-rf", "zig-out", "zig-cache" }).step);
}
