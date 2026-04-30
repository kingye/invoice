const std = @import("std");

pub const AttachmentEntry = struct {
    invoice_number: []const u8,
    filepath: []const u8,
    filename: []const u8,
};

pub fn createArchive(allocator: std.mem.Allocator, detailPath: []const u8, summaryPath: []const u8, attachmentPaths: []const AttachmentEntry, outputPath: [:0]const u8) !void {
    var argv = std.ArrayList([]const u8).init(allocator);
    defer argv.deinit();

    try argv.append("zip");
    try argv.append("-j");
    try argv.append(outputPath);

    var detailNameBuf: [256]u8 = undefined;
    const detailName = try std.fmt.bufPrintZ(&detailNameBuf, "{s}", .{detailPath});

    var summaryNameBuf: [256]u8 = undefined;
    const summaryName = try std.fmt.bufPrintZ(&summaryNameBuf, "{s}", .{summaryPath});

    try argv.append(detailName);
    try argv.append(summaryName);

    for (attachmentPaths) |att| {
        var fpBuf: [1024]u8 = undefined;
        const fp = try std.fmt.bufPrintZ(&fpBuf, "{s}", .{att.filepath});
        try argv.append(fp);
    }

    const result = std.process.Child.run(.{
        .allocator = allocator,
        .argv = argv.items,
    }) catch |err| {
        std.log.err("Failed to create zip archive: {any}. Is 'zip' installed?", .{err});
        return err;
    };
    allocator.free(result.stdout);
    allocator.free(result.stderr);
}
