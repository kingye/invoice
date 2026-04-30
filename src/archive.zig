const std = @import("std");

pub const AttachmentEntry = struct {
    invoice_number: []const u8,
    filepath: []const u8,
    filename: []const u8,
};

pub fn createArchive(allocator: std.mem.Allocator, detailPath: []const u8, summaryPath: []const u8, attachmentPaths: []const AttachmentEntry, outputPath: [:0]const u8) !void {
    const tmpDir = ".invoice/tmp_archive";
    std.fs.cwd().makePath(tmpDir) catch |err| switch (err) {
        error.PathAlreadyExists => {},
        else => return err,
    };
    defer std.fs.cwd().deleteTree(tmpDir) catch {};

    var detailNameBuf: [256]u8 = undefined;
    const detailName = try std.fmt.bufPrintZ(&detailNameBuf, "{s}/明细表.xlsx", .{tmpDir});
    try copyFile(detailPath, detailName);

    var summaryNameBuf: [256]u8 = undefined;
    const summaryName = try std.fmt.bufPrintZ(&summaryNameBuf, "{s}/汇总表.xlsx", .{tmpDir});
    try copyFile(summaryPath, summaryName);

    for (attachmentPaths) |att| {
        var attDirBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
        const attDir = try std.fmt.bufPrintZ(&attDirBuf, "{s}/attachments/{s}", .{ tmpDir, att.invoice_number });
        std.fs.cwd().makePath(attDir) catch |err| switch (err) {
            error.PathAlreadyExists => {},
            else => return err,
        };

        var destBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
        const destPath = try std.fmt.bufPrintZ(&destBuf, "{s}/attachments/{s}/{s}", .{ tmpDir, att.invoice_number, att.filename });
        try copyFile(att.filepath, destPath);
    }

    var argv = std.ArrayList([]const u8).init(allocator);
    defer argv.deinit();

    try argv.append("zip");
    try argv.append("-r");
    try argv.append(outputPath);
    try argv.append(".");

    const result = std.process.Child.run(.{
        .allocator = allocator,
        .argv = argv.items,
        .cwd_dir = std.fs.cwd().openDir(tmpDir, .{}) catch |err| {
            std.log.err("Failed to open tmp dir '{s}': {any}", .{ tmpDir, err });
            return err;
        },
    }) catch |err| {
        std.log.err("Failed to create zip archive: {any}. Is 'zip' installed?", .{err});
        return err;
    };
    allocator.free(result.stdout);
    allocator.free(result.stderr);
}

fn copyFile(src: []const u8, dest: []const u8) !void {
    var srcBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    var destBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const srcZ = try std.fmt.bufPrintZ(&srcBuf, "{s}", .{src});
    const destZ = try std.fmt.bufPrintZ(&destBuf, "{s}", .{dest});

    const srcFile = std.fs.cwd().openFileZ(srcZ, .{}) catch |err| {
        std.log.err("Cannot open source file '{s}': {any}", .{ src, err });
        return err;
    };
    defer srcFile.close();

    const destFile = try std.fs.cwd().createFileZ(destZ, .{});
    defer destFile.close();

    var buf: [4096]u8 = undefined;
    while (true) {
        const bytesRead = srcFile.read(&buf) catch |err| {
            std.log.err("Error reading '{s}': {any}", .{ src, err });
            return err;
        };
        if (bytesRead == 0) break;
        try destFile.writeAll(buf[0..bytesRead]);
    }
}
