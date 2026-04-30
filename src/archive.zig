const std = @import("std");
const c = @cImport({
    @cInclude("miniz.h");
});

pub fn createArchive(allocator: std.mem.Allocator, detailPath: []const u8, summaryPath: []const u8, attachmentPaths: []const struct { invoice_number: []const u8, filepath: []const u8, filename: []const u8 }, outputPath: [:0]const u8) !void {
    _ = allocator;

    var detailNameBuf: [256]u8 = undefined;
    var summaryNameBuf: [256]u8 = undefined;

    const period = extractPeriod(detailPath);
    if (period.len > 0) {
        _ = std.fmt.bufPrintZ(&detailNameBuf, "明细表_{s}.xlsx", .{period}) catch unreachable;
        _ = std.fmt.bufPrintZ(&summaryNameBuf, "汇总表_{s}.xlsx", .{period}) catch unreachable;
    } else {
        _ = std.fmt.bufPrintZ(&detailNameBuf, "明细表.xlsx", .{}) catch unreachable;
        _ = std.fmt.bufPrintZ(&summaryNameBuf, "汇总表.xlsx", .{}) catch unreachable;
    }

    const detailData = try readFile(allocator, detailPath);
    defer allocator.free(detailData);
    const summaryData = try readFile(allocator, summaryPath);
    defer allocator.free(summaryData);

    var success = c.mz_zip_add_mem_to_archive_file_in_place(outputPath, &detailNameBuf, detailData.ptr, detailData.len, null, 0, c.MZ_BEST_COMPRESSION);
    if (success == 0) return error.ArchiveFailed;

    success = c.mz_zip_add_mem_to_archive_file_in_place(outputPath, &summaryNameBuf, summaryData.ptr, summaryData.len, null, 0, c.MZ_BEST_COMPRESSION);
    if (success == 0) return error.ArchiveFailed;

    for (attachmentPaths) |att| {
        const attData = readFile(allocator, att.filepath) catch |err| {
            std.log.err("Cannot read attachment '{s}': {any}", .{ att.filepath, err });
            continue;
        };
        defer allocator.free(attData);

        var entryNameBuf: [1024]u8 = undefined;
        _ = std.fmt.bufPrintZ(&entryNameBuf, "attachments/{s}/{s}", .{ att.invoice_number, att.filename }) catch unreachable;

        success = c.mz_zip_add_mem_to_archive_file_in_place(outputPath, &entryNameBuf, attData.ptr, attData.len, null, 0, c.MZ_BEST_COMPRESSION);
        if (success == 0) return error.ArchiveFailed;
    }
}

fn readFile(allocator: std.mem.Allocator, path: []const u8) ![]u8 {
    const file = try std.fs.cwd().openFile(path, .{});
    defer file.close();
    const stat = try file.stat();
    const data = try allocator.alloc(u8, stat.size);
    errdefer allocator.free(data);
    const bytes_read = try file.readAll(data);
    if (bytes_read != stat.size) return error.IncompleteRead;
    return data;
}

fn extractPeriod(path: []const u8) []const u8 {
    if (std.mem.indexOf(u8, path, "明细表_")) |start| {
        const after = start + "明细表_".len;
        if (std.mem.indexOfScalarPos(u8, path, after, '.')) |end| {
            return path[after..end];
        }
    }
    return "";
}
