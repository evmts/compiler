const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Step 1: Initialize submodules
    const init_submodules = b.addSystemCommand(&.{ "git", "submodule", "update", "--init", "--recursive" });
    const init_step = b.step("init", "Initialize git submodules");
    init_step.dependOn(&init_submodules.step);

    // Build the C++ wrapper as a static library
    const wrapper_lib = b.addStaticLibrary(.{
        .name = "solidity-parser-wrapper",
        .target = target,
        .optimize = optimize,
    });

    wrapper_lib.step.dependOn(&init_submodules.step);

    // Add C++ source
    wrapper_lib.addCSourceFile(.{
        .file = .{ .path = "solidity-parser-wrapper.cpp" },
        .flags = &.{
            "-std=c++17",
            "-fno-exceptions",
            "-fno-rtti",
        },
    });

    // Add Solidity include paths
    wrapper_lib.addIncludePath(.{ .path = "solidity" });
    wrapper_lib.addIncludePath(.{ .path = "solidity/libsolidity" });
    wrapper_lib.addIncludePath(.{ .path = "solidity/liblangutil" });
    wrapper_lib.addIncludePath(.{ .path = "solidity/libsolutil" });
    wrapper_lib.addIncludePath(.{ .path = "solidity/libevmasm" });

    // Link C++ standard library
    wrapper_lib.linkLibCpp();

    // Build the Zig executable
    const exe = b.addExecutable(.{
        .name = "shadow",
        .root_source_file = .{ .path = "shadow.zig" },
        .target = target,
        .optimize = optimize,
    });

    // Link the wrapper library
    exe.linkLibrary(wrapper_lib);
    exe.linkLibCpp();

    // Add include path for C header
    exe.addIncludePath(.{ .path = "." });

    b.installArtifact(exe);

    // Run step
    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());

    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run the shadow parser demo");
    run_step.dependOn(&run_cmd.step);

    // Test step
    const tests = b.addTest(.{
        .root_source_file = .{ .path = "shadow_test.zig" },
        .target = target,
        .optimize = optimize,
    });

    // Link the wrapper library for tests
    tests.linkLibrary(wrapper_lib);
    tests.linkLibCpp();
    tests.addIncludePath(.{ .path = "." });

    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_tests.step);

    // WASM build target
    const wasm_target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
    });

    const wasm_lib = b.addStaticLibrary(.{
        .name = "shadow-wasm",
        .root_source_file = .{ .path = "shadow.zig" },
        .target = wasm_target,
        .optimize = .ReleaseSmall,
    });

    wasm_lib.step.dependOn(&init_submodules.step);

    // For WASM, we'll need to compile the C++ wrapper to WASM too
    const wasm_wrapper = b.addStaticLibrary(.{
        .name = "solidity-parser-wrapper-wasm",
        .target = wasm_target,
        .optimize = .ReleaseSmall,
    });

    wasm_wrapper.addCSourceFile(.{
        .file = .{ .path = "solidity-parser-wrapper.cpp" },
        .flags = &.{
            "-std=c++17",
            "-fno-exceptions",
            "-fno-rtti",
        },
    });

    wasm_wrapper.addIncludePath(.{ .path = "solidity" });
    wasm_wrapper.addIncludePath(.{ .path = "solidity/libsolidity" });
    wasm_wrapper.addIncludePath(.{ .path = "solidity/liblangutil" });
    wasm_wrapper.addIncludePath(.{ .path = "solidity/libsolutil" });
    wasm_wrapper.addIncludePath(.{ .path = "solidity/libevmasm" });

    wasm_lib.linkLibrary(wasm_wrapper);
    wasm_lib.addIncludePath(.{ .path = "." });

    const wasm_install = b.addInstallArtifact(wasm_lib, .{});
    const wasm_step = b.step("wasm", "Build WASM library");
    wasm_step.dependOn(&wasm_install.step);

    // TypeScript wrapper generation
    const generate_ts = b.addSystemCommand(&.{ "node", "scripts/generate-ts-bindings.js" });
    generate_ts.step.dependOn(&wasm_install.step);
    const ts_step = b.step("typescript", "Generate TypeScript bindings for WASM");
    ts_step.dependOn(&generate_ts.step);

    // Full build step that does everything
    const all_step = b.step("all", "Build everything: native, wasm, and typescript bindings");
    all_step.dependOn(init_step);
    all_step.dependOn(b.getInstallStep());
    all_step.dependOn(&wasm_install.step);
    all_step.dependOn(&generate_ts.step);

    // Clean step
    const clean_cmd = b.addSystemCommand(&.{ "rm", "-rf", "zig-out", "zig-cache" });
    const clean_step = b.step("clean", "Remove build artifacts");
    clean_step.dependOn(&clean_cmd.step);
}
