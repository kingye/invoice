const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const fetch_sqlite = b.addSystemCommand(&.{ "bash", "libs/fetch_sqlite.sh" });

    const sqlite_dep = b.addStaticLibrary(.{
        .name = "sqlite3",
        .target = target,
        .optimize = optimize,
    });
    sqlite_dep.step.dependOn(&fetch_sqlite.step);
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
            "libs/libxlsxwriter/src/app.c",
            "libs/libxlsxwriter/src/chart.c",
            "libs/libxlsxwriter/src/chartsheet.c",
            "libs/libxlsxwriter/src/comment.c",
            "libs/libxlsxwriter/src/content_types.c",
            "libs/libxlsxwriter/src/core.c",
            "libs/libxlsxwriter/src/custom.c",
            "libs/libxlsxwriter/src/drawing.c",
            "libs/libxlsxwriter/src/format.c",
            "libs/libxlsxwriter/src/hash_table.c",
            "libs/libxlsxwriter/src/metadata.c",
            "libs/libxlsxwriter/src/packager.c",
            "libs/libxlsxwriter/src/relationships.c",
            "libs/libxlsxwriter/src/rich_value.c",
            "libs/libxlsxwriter/src/rich_value_rel.c",
            "libs/libxlsxwriter/src/rich_value_structure.c",
            "libs/libxlsxwriter/src/rich_value_types.c",
            "libs/libxlsxwriter/src/shared_strings.c",
            "libs/libxlsxwriter/src/styles.c",
            "libs/libxlsxwriter/src/table.c",
            "libs/libxlsxwriter/src/theme.c",
            "libs/libxlsxwriter/src/utility.c",
            "libs/libxlsxwriter/src/vml.c",
            "libs/libxlsxwriter/src/workbook.c",
            "libs/libxlsxwriter/src/worksheet.c",
            "libs/libxlsxwriter/src/xmlwriter.c",
            "libs/libxlsxwriter/third_party/dtoa/emyg_dtoa.c",
            "libs/libxlsxwriter/third_party/md5/md5.c",
            "libs/libxlsxwriter/third_party/minizip/ioapi.c",
            "libs/libxlsxwriter/third_party/minizip/zip.c",
            "libs/libxlsxwriter/third_party/tmpfileplus/tmpfileplus.c",
        },
        .flags = &.{
            "-DNOCRYPT",
            "-DSTANDARD_LICENSE",
        },
    });
    xlsxwriter_dep.addIncludePath(b.path("libs/libxlsxwriter/include"));
    xlsxwriter_dep.addIncludePath(b.path("libs/libxlsxwriter/src"));
    xlsxwriter_dep.addIncludePath(b.path("libs/libxlsxwriter/third_party"));
    xlsxwriter_dep.addIncludePath(b.path("libs/libxlsxwriter/third_party/minizip"));
    xlsxwriter_dep.linkSystemLibrary("z");
    xlsxwriter_dep.linkLibC();
    b.installArtifact(xlsxwriter_dep);

    const exe = b.addExecutable(.{
        .name = "invoice",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    exe.step.dependOn(&fetch_sqlite.step);
    exe.addIncludePath(b.path("libs"));
    exe.addIncludePath(b.path("libs/libxlsxwriter/include"));
    exe.linkLibrary(sqlite_dep);
    exe.linkLibrary(xlsxwriter_dep);
    exe.linkSystemLibrary("z");
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
    exe_tests.step.dependOn(&fetch_sqlite.step);
    exe_tests.addIncludePath(b.path("libs"));
    exe_tests.addIncludePath(b.path("libs/libxlsxwriter/include"));
    exe_tests.linkLibrary(sqlite_dep);
    exe_tests.linkLibrary(xlsxwriter_dep);
    exe_tests.linkSystemLibrary("z");
    exe_tests.linkLibC();

    const run_exe_tests = b.addRunArtifact(exe_tests);
    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_exe_tests.step);
}
