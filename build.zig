const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const sqlite_dep = b.addStaticLibrary(.{
        .name = "sqlite3",
        .target = target,
        .optimize = optimize,
    });
    sqlite_dep.addCSourceFiles(.{
        .files = &.{"libs/sqlite3.c"},
        .flags = &.{
            "-DSQLITE_THREADSAFE=0",
            "-DSQLITE_OMIT_LOAD_EXTENSION",
            "-DSQLITE_DEFAULT_FOREIGN_KEYS=1",
        },
    });
    sqlite_dep.addIncludePath(b.path("libs"));
    sqlite_dep.linkLibC();
    b.installArtifact(sqlite_dep);

    const xlsxwriter_dep = b.addStaticLibrary(.{
        .name = "xlsxwriter",
        .target = target,
        .optimize = optimize,
    });
    xlsxwriter_dep.addCSourceFiles(.{
        .files = &.{
            "libs/xlsxwriter_src/app.c",
            "libs/xlsxwriter_src/chart.c",
            "libs/xlsxwriter_src/chartsheet.c",
            "libs/xlsxwriter_src/comment.c",
            "libs/xlsxwriter_src/content_types.c",
            "libs/xlsxwriter_src/core.c",
            "libs/xlsxwriter_src/custom.c",
            "libs/xlsxwriter_src/drawing.c",
            "libs/xlsxwriter_src/format.c",
            "libs/xlsxwriter_src/hash_table.c",
            "libs/xlsxwriter_src/metadata.c",
            "libs/xlsxwriter_src/packager.c",
            "libs/xlsxwriter_src/relationships.c",
            "libs/xlsxwriter_src/rich_value.c",
            "libs/xlsxwriter_src/rich_value_rel.c",
            "libs/xlsxwriter_src/rich_value_structure.c",
            "libs/xlsxwriter_src/rich_value_types.c",
            "libs/xlsxwriter_src/shared_strings.c",
            "libs/xlsxwriter_src/styles.c",
            "libs/xlsxwriter_src/table.c",
            "libs/xlsxwriter_src/theme.c",
            "libs/xlsxwriter_src/utility.c",
            "libs/xlsxwriter_src/vml.c",
            "libs/xlsxwriter_src/workbook.c",
            "libs/xlsxwriter_src/worksheet.c",
            "libs/xlsxwriter_src/xmlwriter.c",
            "libs/xlsxwriter_third_party/dtoa/emyg_dtoa.c",
            "libs/xlsxwriter_third_party/md5/md5.c",
            "libs/xlsxwriter_third_party/minizip/ioapi.c",
            "libs/xlsxwriter_third_party/minizip/zip.c",
            "libs/xlsxwriter_third_party/tmpfileplus/tmpfileplus.c",
        },
        .flags = &.{
            "-DNOCRYPT",
            "-DUSE_MINIZIP",
            "-DSTANDARD_LICENSE",
        },
    });
    xlsxwriter_dep.addIncludePath(b.path("libs/xlsxwriter"));
    xlsxwriter_dep.addIncludePath(b.path("libs/xlsxwriter_src"));
    xlsxwriter_dep.addIncludePath(b.path("libs/xlsxwriter_third_party"));
    xlsxwriter_dep.addIncludePath(b.path("libs/xlsxwriter_third_party/minizip"));
    xlsxwriter_dep.linkLibC();
    b.installArtifact(xlsxwriter_dep);

    const miniz_dep = b.addStaticLibrary(.{
        .name = "miniz",
        .target = target,
        .optimize = optimize,
    });
    miniz_dep.addCSourceFiles(.{
        .files = &.{
            "libs/miniz/miniz.c",
            "libs/miniz/miniz_tdef.c",
            "libs/miniz/miniz_tinfl.c",
            "libs/miniz/miniz_zip.c",
        },
        .flags = &.{
            "-DMINIZ_DISABLE_EXPORT",
        },
    });
    miniz_dep.addIncludePath(b.path("libs/miniz"));
    miniz_dep.linkLibC();
    b.installArtifact(miniz_dep);

    const exe = b.addExecutable(.{
        .name = "invoice",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    exe.addIncludePath(b.path("libs"));
    exe.addIncludePath(b.path("libs/xlsxwriter"));
    exe.addIncludePath(b.path("libs/miniz"));
    exe.linkLibrary(sqlite_dep);
    exe.linkLibrary(xlsxwriter_dep);
    exe.linkLibrary(miniz_dep);
    exe.linkLibC();
    b.installArtifact(exe);

    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }
    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);

    const exe_tests = b.addTest(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    exe_tests.addIncludePath(b.path("libs"));
    exe_tests.addIncludePath(b.path("libs/xlsxwriter"));
    exe_tests.addIncludePath(b.path("libs/miniz"));
    exe_tests.linkLibrary(sqlite_dep);
    exe_tests.linkLibrary(xlsxwriter_dep);
    exe_tests.linkLibrary(miniz_dep);
    exe_tests.linkLibC();

    const run_exe_tests = b.addRunArtifact(exe_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_exe_tests.step);
}
