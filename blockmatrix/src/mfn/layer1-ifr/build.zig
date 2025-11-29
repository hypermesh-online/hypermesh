const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // IFR library module
    const ifr_lib = b.createModule(.{
        .root_source_file = b.path("src/ifr.zig"),
        .target = target,
        .optimize = optimize,
    });

    // Main executable module
    const exe_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    exe_mod.addImport("ifr", ifr_lib);

    // Static library for C FFI integration
    const lib = b.addLibrary(.{
        .linkage = .static,
        .name = "hypermesh_ifr",
        .root_module = ifr_lib,
    });
    lib.linkLibC();
    b.installArtifact(lib);

    // Executable
    const exe = b.addExecutable(.{
        .name = "hypermesh_ifr",
        .root_module = exe_mod,
    });
    exe.linkLibC();
    b.installArtifact(exe);

    // Run command
    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run the IFR service");
    run_step.dependOn(&run_cmd.step);

    // Benchmarks
    const bench = b.addExecutable(.{
        .name = "ifr_bench",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/bench.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    bench.root_module.addImport("ifr", ifr_lib);
    bench.linkLibC();

    const run_bench = b.addRunArtifact(bench);
    const bench_step = b.step("bench", "Run IFR performance benchmarks");
    bench_step.dependOn(&run_bench.step);

    // Unit tests
    const lib_unit_tests = b.addTest(.{
        .root_module = ifr_lib,
    });
    lib_unit_tests.linkLibC();

    const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);
    
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_lib_unit_tests.step);

    // Integration tests
    const integration_tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("tests/integration.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    integration_tests.root_module.addImport("ifr", ifr_lib);
    integration_tests.linkLibC();

    const run_integration_tests = b.addRunArtifact(integration_tests);
    const integration_step = b.step("test-integration", "Run integration tests");
    integration_step.dependOn(&run_integration_tests.step);
}