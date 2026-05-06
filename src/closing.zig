const std = @import("std");
const db_mod = @import("db.zig");
const models = @import("models.zig");
const report = @import("report.zig");
const archive = @import("archive.zig");
const attachment_mod = @import("attachment.zig");

pub const CloseType = enum {
    month,
    year,
};

pub fn closePeriod(allocator: std.mem.Allocator, database: *db_mod.Db, basePath: []const u8, closeType: CloseType, period: []const u8) !void {
    const typeStr: [:0]const u8 = switch (closeType) {
        .month => "month",
        .year => "year",
    };

    if (try database.isClosed(period, typeStr)) {
        std.log.err("Period {s} ({s}) is already closed", .{ period, typeStr });
        return error.AlreadyClosed;
    }

    const invoices = try queryInvoicesForPeriod(allocator, database, closeType, period);
    defer {
        for (invoices) |inv| {
            allocator.free(inv.number);
            allocator.free(inv.date);
            allocator.free(inv.type);
            allocator.free(inv.item_name);
            allocator.free(inv.seller_name);
            allocator.free(inv.seller_tax_id);
            allocator.free(inv.buyer_name);
            allocator.free(inv.buyer_tax_id);
            allocator.free(inv.category);
            allocator.free(inv.remark);
            allocator.free(inv.created_at);
            allocator.free(inv.updated_at);
        }
        allocator.free(invoices);
    }

    if (invoices.len == 0) {
        std.log.err("No invoices found for period {s}", .{period});
        return error.NoInvoices;
    }

    var detailPathBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    var summaryPathBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const detailPath = try std.fmt.bufPrintZ(&detailPathBuf, "{s}/.invoice/明细表_{s}.xlsx", .{ basePath, period });
    const summaryPath = try std.fmt.bufPrintZ(&summaryPathBuf, "{s}/.invoice/汇总表_{s}.xlsx", .{ basePath, period });

    try report.generateDetailReport(invoices, detailPath);

    const summaryEntries = try report.computeSummary(allocator, invoices);
    defer allocator.free(summaryEntries);
    try report.generateSummaryReport(summaryEntries, summaryPath);

    var attList = std.ArrayList(archive.AttachmentEntry).init(allocator);
    defer attList.deinit();

    for (invoices) |inv| {
        const atts = attachment_mod.getAttachmentsForInvoice(database, allocator, inv.id) catch &[_]models.Attachment{};
        defer allocator.free(atts);
        for (atts) |att| {
            try attList.append(.{
                .invoice_number = inv.number,
                .filepath = att.filepath,
                .filename = att.filename,
            });
        }
    }

    var archivePathBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const archivePath = try std.fmt.bufPrintZ(&archivePathBuf, "{s}/.invoice/close_{s}.zip", .{ basePath, period });

    try archive.createArchive(allocator, detailPath, summaryPath, attList.items, archivePath);

    const insertSql = "INSERT INTO closings (type, period, archive_path) VALUES (?, ?, ?)";
    const insertStmt = try database.prepare(insertSql);
    defer insertStmt.deinit();

    var typeBuf: [64]u8 = undefined;
    var periodBuf: [32]u8 = undefined;
    var archiveBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const typeZ = try std.fmt.bufPrintZ(&typeBuf, "{s}", .{typeStr});
    const periodZ = try std.fmt.bufPrintZ(&periodBuf, "{s}", .{period});
    const archiveZ = try std.fmt.bufPrintZ(&archiveBuf, ".invoice/close_{s}.zip", .{period});

    try insertStmt.bindText(1, typeZ);
    try insertStmt.bindText(2, periodZ);
    try insertStmt.bindText(3, archiveZ);

    const rowDone = try insertStmt.step();
    if (rowDone) return error.UnexpectedRow;
}

pub fn checkPeriodClosed(database: *db_mod.Db, date: []const u8) !bool {
    if (date.len >= 7) {
        const month = date[0..7];
        if (try database.isClosed(month, "month")) return true;
    }
    if (date.len >= 4) {
        const year = date[0..4];
        if (try database.isClosed(year, "year")) return true;
    }
    return false;
}

pub fn queryInvoicesForPeriod(allocator: std.mem.Allocator, database: *db_mod.Db, closeType: CloseType, period: []const u8) ![]models.Invoice {
    _ = closeType;
    var list = std.ArrayList(models.Invoice).init(allocator);
    errdefer list.deinit();

    const sql = "SELECT id, number, date, type, item_name, amount, tax_rate, tax, total, seller_name, seller_tax_id, buyer_name, buyer_tax_id, category, remark, created_at, updated_at FROM invoices WHERE date LIKE ? ORDER BY date, id";
    const stmt = try database.prepare(sql);
    defer stmt.deinit();

    var patternBuf: [64]u8 = undefined;
    const pattern = try std.fmt.bufPrintZ(&patternBuf, "{s}-%", .{period});
    try stmt.bindText(1, pattern);

    while (try stmt.step()) {
        const inv = try models.Invoice.fromRowAlloc(stmt, allocator);
        try list.append(inv);
    }

    return try list.toOwnedSlice();
}
