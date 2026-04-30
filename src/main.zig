const std = @import("std");
const db_mod = @import("db.zig");
const models = @import("models.zig");
const cli = @import("cli.zig");
const attachment = @import("attachment.zig");
const closing = @import("closing.zig");
const report_mod = @import("report.zig");
const archive_mod = @import("archive.zig");

pub const version = "0.1.0";

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    const allocator = std.heap.page_allocator;

    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    if (args.len < 2) {
        try cli.printHelp(stdout);
        return;
    }

    const subCmd = std.mem.sliceTo(args[1], 0);
    const cmd = cli.parseCommand(subCmd);

    switch (cmd) {
        .version => {
            try stdout.print("invoice v{s}\n", .{version});
        },
        .help => {
            try cli.printHelp(stdout);
        },
        .init => {
            try cmdInit(stdout);
        },
        .add => {
            if (args.len > 2 and std.mem.eql(u8, std.mem.sliceTo(args[2], 0), "--help")) {
                try cli.printAddHelp(stdout);
                return;
            }
            try cmdAdd(stdout, allocator, args[2..]);
        },
        .list => {
            try cmdList(stdout, allocator, args[2..]);
        },
        .show => {
            try cmdShow(stdout, allocator, args[2..]);
        },
        .edit => {
            try cmdEdit(stdout, allocator, args[2..]);
        },
        .delete => {
            try cmdDelete(stdout, allocator, args[2..]);
        },
        .close => {
            try cmdClose(stdout, allocator, args[2..]);
        },
        .export_cmd => {
            try cmdExport(stdout, allocator, args[2..]);
        },
        .unknown => {
            try stdout.print("Unknown command: {s}\n", .{subCmd});
            try cli.printHelp(stdout);
        },
    }
}

fn openDb() !db_mod.Db {
    var cwdBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const cwd = try std.posix.getcwd(&cwdBuf);
    var database = try db_mod.initDb(cwd);
    try database.initSchema();
    return database;
}

fn cmdInit(writer: anytype) !void {
    var cwdBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const cwd = try std.posix.getcwd(&cwdBuf);
    var database = try db_mod.initDb(cwd);
    defer database.close();
    try database.initSchema();
    try writer.print("Initialized invoice database in {s}/.invoice/invoice.db\n", .{cwd});
}

fn cmdAdd(writer: anytype, allocator: std.mem.Allocator, args: []const []const u8) !void {
    const addArgs = try cli.parseAddArgs(allocator, args);
    defer allocator.free(addArgs.attach);

    if (addArgs.number.len == 0) {
        try writer.print("Error: --number is required\n", .{});
        return;
    }
    if (addArgs.date.len == 0) {
        try writer.print("Error: --date is required\n", .{});
        return;
    }

    var database = try openDb();
    defer database.close();

    const sql = "INSERT INTO invoices (number, date, type, item_name, amount, tax_rate, tax, total, seller_name, seller_tax_id, buyer_name, buyer_tax_id, category, remark) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
    const stmt = try database.prepare(sql);
    defer stmt.deinit();

    const inv = models.Invoice{
        .number = addArgs.number,
        .date = addArgs.date,
        .type = addArgs.type,
        .item_name = addArgs.item_name,
        .amount = addArgs.amount,
        .tax_rate = addArgs.tax_rate,
        .tax = addArgs.tax,
        .total = addArgs.total,
        .seller_name = addArgs.seller_name,
        .seller_tax_id = addArgs.seller_tax_id,
        .buyer_name = addArgs.buyer_name,
        .buyer_tax_id = addArgs.buyer_tax_id,
        .category = addArgs.category,
        .remark = addArgs.remark,
    };
    try inv.bindParams(stmt);
    const rowDone = try stmt.step();
    if (rowDone) return error.UnexpectedRow;
    const id = database.lastInsertRowId();

    for (addArgs.attach) |attPath| {
        var cwdBuf2: [std.fs.MAX_PATH_BYTES]u8 = undefined;
        const cwd2 = try std.posix.getcwd(&cwdBuf2);
        attachment.addAttachment(allocator, &database, id, addArgs.number, cwd2, attPath) catch |err| {
            std.log.err("Failed to add attachment '{s}': {any}", .{ attPath, err });
        };
    }

    try writer.print("Invoice added with id={d}\n", .{id});
}

fn cmdList(writer: anytype, allocator: std.mem.Allocator, args: []const []const u8) !void {
    const listArgs = try cli.parseListArgs(args);

    var database = try openDb();
    defer database.close();

    var sqlList = std.ArrayList(u8).init(allocator);
    defer sqlList.deinit();

    const baseSql = "SELECT id, number, date, type, item_name, amount, tax_rate, tax, total, seller_name, seller_tax_id, buyer_name, buyer_tax_id, category, remark, created_at, updated_at FROM invoices";
    try sqlList.appendSlice(baseSql);

    var bindIdx: i32 = 0;
    var bindMonth: ?[]const u8 = null;
    var bindYear: ?[]const u8 = null;
    var bindCategory: ?[]const u8 = null;

    var hasWhere = false;
    if (listArgs.month.len > 0) {
        if (!hasWhere) try sqlList.appendSlice(" WHERE");
        try sqlList.appendSlice(" date LIKE ?");
        bindIdx += 1;
        bindMonth = listArgs.month;
        hasWhere = true;
    }
    if (listArgs.year.len > 0) {
        if (!hasWhere) {
            try sqlList.appendSlice(" WHERE");
        } else {
            try sqlList.appendSlice(" AND");
        }
        try sqlList.appendSlice(" date LIKE ?");
        bindIdx += 1;
        bindYear = listArgs.year;
        hasWhere = true;
    }
    if (listArgs.category.len > 0) {
        if (!hasWhere) {
            try sqlList.appendSlice(" WHERE");
        } else {
            try sqlList.appendSlice(" AND");
        }
        try sqlList.appendSlice(" category = ?");
        bindIdx += 1;
        bindCategory = listArgs.category;
        hasWhere = true;
    }

    try sqlList.appendSlice(" ORDER BY date DESC, id DESC");

    const sqlZ = try sqlList.toOwnedSliceSentinel(0);
    defer allocator.free(sqlZ);

    const stmt = try database.prepare(sqlZ);
    defer stmt.deinit();

    var idx: i32 = 1;
    if (bindMonth) |m| {
        var buf: [64]u8 = undefined;
        const pattern = try std.fmt.bufPrintZ(&buf, "{s}-%", .{m});
        try stmt.bindText(idx, pattern);
        idx += 1;
    }
    if (bindYear) |y| {
        var buf: [64]u8 = undefined;
        const pattern = try std.fmt.bufPrintZ(&buf, "{s}-%", .{y});
        try stmt.bindText(idx, pattern);
        idx += 1;
    }
    if (bindCategory) |c| {
        var buf: [256]u8 = undefined;
        const catZ = try std.fmt.bufPrintZ(&buf, "{s}", .{c});
        try stmt.bindText(idx, catZ);
        idx += 1;
    }

    try writer.print("{s:>4}  {s:<12} {s:<12} {s:<10} {s:<12} {s:>10} {s:>6} {s:>8} {s:>10} {s:<16}\n", .{ "ID", "Number", "Date", "Type", "Item", "Amount", "Tax%", "Tax", "Total", "Seller" });
    try writer.print("{s:-<120}\n", .{""});

    while (try stmt.step()) {
        const inv = try models.Invoice.fromRowAlloc(stmt, std.heap.page_allocator);
        try writer.print("{d:>4}  {s:<12} {s:<12} {s:<10} {s:<12} {d:>10.2} {d:>5.0}% {d:>8.2} {d:>10.2} {s:<16}\n", .{
            @as(u64, @intCast(inv.id)),
            inv.number,
            inv.date,
            inv.type,
            inv.item_name,
            inv.amount,
            inv.tax_rate * 100,
            inv.tax,
            inv.total,
            inv.seller_name,
        });
    }
}

fn cmdShow(writer: anytype, allocator: std.mem.Allocator, args: []const []const u8) !void {
    _ = allocator;
    const showArgs = try cli.parseShowArgs(args);
    if (showArgs.id == 0) {
        try writer.print("Usage: invoice show <id>\n", .{});
        return;
    }

    var database = try openDb();
    defer database.close();

    const sql = "SELECT id, number, date, type, item_name, amount, tax_rate, tax, total, seller_name, seller_tax_id, buyer_name, buyer_tax_id, category, remark, created_at, updated_at FROM invoices WHERE id = ?";
    const stmt = try database.prepare(sql);
    defer stmt.deinit();

    try stmt.bindInt64(1, showArgs.id);

    if (try stmt.step()) {
        const inv = try models.Invoice.fromRowAlloc(stmt, std.heap.page_allocator);

        try writer.print("Invoice #{d}\n", .{inv.id});
        try writer.print("  Number:       {s}\n", .{inv.number});
        try writer.print("  Date:         {s}\n", .{inv.date});
        try writer.print("  Type:         {s}\n", .{inv.type});
        try writer.print("  Item:         {s}\n", .{inv.item_name});
        try writer.print("  Amount:       {d:.2}\n", .{inv.amount});
        try writer.print("  Tax Rate:     {d:.0}%\n", .{inv.tax_rate * 100});
        try writer.print("  Tax:          {d:.2}\n", .{inv.tax});
        try writer.print("  Total:        {d:.2}\n", .{inv.total});
        try writer.print("  Seller:       {s}\n", .{inv.seller_name});
        try writer.print("  Seller TaxID: {s}\n", .{inv.seller_tax_id});
        try writer.print("  Buyer:        {s}\n", .{inv.buyer_name});
        try writer.print("  Buyer TaxID:  {s}\n", .{inv.buyer_tax_id});
        try writer.print("  Category:     {s}\n", .{inv.category});
        try writer.print("  Remark:       {s}\n", .{inv.remark});
        try writer.print("  Created:      {s}\n", .{inv.created_at});
        try writer.print("  Updated:      {s}\n", .{inv.updated_at});

        const attSql = "SELECT id, invoice_id, filename, filepath, file_hash, file_size, created_at FROM attachments WHERE invoice_id = ?";
        const attStmt = try database.prepare(attSql);
        defer attStmt.deinit();
        try attStmt.bindInt64(1, showArgs.id);

        try writer.print("\n  Attachments:\n", .{});
        var hasAttachments = false;
        while (try attStmt.step()) {
            hasAttachments = true;
            const att = try models.Attachment.fromRowAlloc(attStmt, std.heap.page_allocator);
            try writer.print("    - {s} ({d} bytes) sha256:{s}\n", .{ att.filename, att.file_size, att.file_hash });
        }
        if (!hasAttachments) {
            try writer.print("    (none)\n", .{});
        }
    } else {
        try writer.print("Invoice #{d} not found\n", .{showArgs.id});
    }
}

fn cmdEdit(writer: anytype, allocator: std.mem.Allocator, args: []const []const u8) !void {
    const editArgs = try cli.parseEditArgs(allocator, args);
    defer allocator.free(editArgs.attach);

    if (editArgs.id == 0) {
        try writer.print("Usage: invoice edit <id> [options]\n", .{});
        return;
    }

    var database = try openDb();
    defer database.close();

    if (try checkInvoiceClosed(&database, editArgs.id)) {
        try writer.print("Error: Invoice #{d} is in a closed period and cannot be modified\n", .{editArgs.id});
        return;
    }

    var sqlList = std.ArrayList(u8).init(allocator);
    defer sqlList.deinit();

    try sqlList.appendSlice("UPDATE invoices SET updated_at = datetime('now', 'localtime')");

    var bindIdx: i32 = 0;
    const MaxBinds = 16;
    var textBinds: [MaxBinds][]const u8 = undefined;
    var doubleBinds: [MaxBinds]f64 = undefined;
    var textCount: i32 = 0;
    var doubleCount: i32 = 0;

    if (editArgs.number) |v| {
        try sqlList.appendSlice(", number = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.date) |v| {
        try sqlList.appendSlice(", date = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.type) |v| {
        try sqlList.appendSlice(", type = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.item_name) |v| {
        try sqlList.appendSlice(", item_name = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.amount) |v| {
        try sqlList.appendSlice(", amount = ?");
        bindIdx += 1;
        doubleBinds[@intCast(doubleCount)] = v;
        doubleCount += 1;
    }
    if (editArgs.tax_rate) |v| {
        try sqlList.appendSlice(", tax_rate = ?");
        bindIdx += 1;
        doubleBinds[@intCast(doubleCount)] = v;
        doubleCount += 1;
    }
    if (editArgs.tax) |v| {
        try sqlList.appendSlice(", tax = ?");
        bindIdx += 1;
        doubleBinds[@intCast(doubleCount)] = v;
        doubleCount += 1;
    }
    if (editArgs.total) |v| {
        try sqlList.appendSlice(", total = ?");
        bindIdx += 1;
        doubleBinds[@intCast(doubleCount)] = v;
        doubleCount += 1;
    }
    if (editArgs.seller_name) |v| {
        try sqlList.appendSlice(", seller_name = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.seller_tax_id) |v| {
        try sqlList.appendSlice(", seller_tax_id = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.buyer_name) |v| {
        try sqlList.appendSlice(", buyer_name = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.buyer_tax_id) |v| {
        try sqlList.appendSlice(", buyer_tax_id = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.category) |v| {
        try sqlList.appendSlice(", category = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }
    if (editArgs.remark) |v| {
        try sqlList.appendSlice(", remark = ?");
        bindIdx += 1;
        textBinds[@intCast(textCount)] = v;
        textCount += 1;
    }

    try sqlList.appendSlice(" WHERE id = ?");
    bindIdx += 1;

    const sqlZ = try sqlList.toOwnedSliceSentinel(0);
    defer allocator.free(sqlZ);

    const stmt = try database.prepare(sqlZ);
    defer stmt.deinit();

    var paramIdx: i32 = 1;
    var tIdx: i32 = 0;
    var dIdx: i32 = 0;

    if (editArgs.number) |_| {
        var buf: [256]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.date) |_| {
        var buf: [64]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.type) |_| {
        var buf: [128]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.item_name) |_| {
        var buf: [256]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.amount) |_| {
        try stmt.bindDouble(paramIdx, doubleBinds[@intCast(dIdx)]);
        paramIdx += 1;
        dIdx += 1;
    }
    if (editArgs.tax_rate) |_| {
        try stmt.bindDouble(paramIdx, doubleBinds[@intCast(dIdx)]);
        paramIdx += 1;
        dIdx += 1;
    }
    if (editArgs.tax) |_| {
        try stmt.bindDouble(paramIdx, doubleBinds[@intCast(dIdx)]);
        paramIdx += 1;
        dIdx += 1;
    }
    if (editArgs.total) |_| {
        try stmt.bindDouble(paramIdx, doubleBinds[@intCast(dIdx)]);
        paramIdx += 1;
        dIdx += 1;
    }
    if (editArgs.seller_name) |_| {
        var buf: [256]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.seller_tax_id) |_| {
        var buf: [64]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.buyer_name) |_| {
        var buf: [256]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.buyer_tax_id) |_| {
        var buf: [64]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.category) |_| {
        var buf: [128]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }
    if (editArgs.remark) |_| {
        var buf: [512]u8 = undefined;
        const z = try std.fmt.bufPrintZ(&buf, "{s}", .{textBinds[@intCast(tIdx)]});
        try stmt.bindText(paramIdx, z);
        paramIdx += 1;
        tIdx += 1;
    }

    try stmt.bindInt64(paramIdx, editArgs.id);

    const rowDone = try stmt.step();
    if (rowDone) return error.UnexpectedRow;

    const changed = database.changes();
    if (changed > 0) {
        try writer.print("Invoice #{d} updated\n", .{editArgs.id});
    } else {
        try writer.print("Invoice #{d} not found\n", .{editArgs.id});
    }

    for (editArgs.attach) |attPath| {
        const invSql = "SELECT number FROM invoices WHERE id = ?";
        const invStmt = try database.prepare(invSql);
        defer invStmt.deinit();
        try invStmt.bindInt64(1, editArgs.id);

        if (try invStmt.step()) {
            const number = invStmt.columnText(0) orelse "";
            var cwdBuf3: [std.fs.MAX_PATH_BYTES]u8 = undefined;
            const cwd3 = try std.posix.getcwd(&cwdBuf3);
            attachment.addAttachment(allocator, &database, editArgs.id, number, cwd3, attPath) catch |err| {
                std.log.err("Failed to add attachment '{s}': {any}", .{ attPath, err });
                continue;
            };
            try writer.print("  Attachment added: {s}\n", .{attPath});
        }
    }
}

fn cmdDelete(writer: anytype, allocator: std.mem.Allocator, args: []const []const u8) !void {
    _ = allocator;
    const deleteArgs = try cli.parseDeleteArgs(args);
    if (deleteArgs.id == 0) {
        try writer.print("Usage: invoice delete <id>\n", .{});
        return;
    }

    var database = try openDb();
    defer database.close();

    if (try checkInvoiceClosed(&database, deleteArgs.id)) {
        try writer.print("Error: Invoice #{d} is in a closed period and cannot be deleted\n", .{deleteArgs.id});
        return;
    }

    const sql = "DELETE FROM invoices WHERE id = ?";
    const stmt = try database.prepare(sql);
    defer stmt.deinit();
    try stmt.bindInt64(1, deleteArgs.id);

    const rowDone = try stmt.step();
    if (rowDone) return error.UnexpectedRow;

    const changed = database.changes();
    if (changed > 0) {
        try writer.print("Invoice #{d} deleted\n", .{deleteArgs.id});
    } else {
        try writer.print("Invoice #{d} not found\n", .{deleteArgs.id});
    }
}

fn checkInvoiceClosed(database: *db_mod.Db, invoiceId: i64) !bool {
    const sql = "SELECT date FROM invoices WHERE id = ?";
    const stmt = try database.prepare(sql);
    defer stmt.deinit();
    try stmt.bindInt64(1, invoiceId);

    if (try stmt.step()) {
        const date = stmt.columnText(0) orelse return false;
        const dateSlice = std.mem.sliceTo(date, 0);
        return try closing.checkPeriodClosed(database, dateSlice);
    }
    return false;
}

fn cmdClose(writer: anytype, allocator: std.mem.Allocator, args: []const []const u8) !void {
    const closeArgs = try cli.parseCloseArgs(args);

    var closeType: closing.CloseType = undefined;
    var period: []const u8 = "";

    if (closeArgs.month.len > 0) {
        period = closeArgs.month;
        closeType = .month;
    } else if (closeArgs.year.len > 0) {
        period = closeArgs.year;
        closeType = .year;
    } else {
        try writer.print("Usage: invoice close --month YYYY-MM or --year YYYY\n", .{});
        return;
    }

    var cwdBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const cwd = try std.posix.getcwd(&cwdBuf);

    var database = try openDb();
    defer database.close();

    closing.closePeriod(allocator, &database, cwd, closeType, period) catch |err| {
        switch (err) {
            error.AlreadyClosed => {
                try writer.print("Error: Period {s} is already closed\n", .{period});
                return;
            },
            error.NoInvoices => {
                try writer.print("Error: No invoices found for period {s}\n", .{period});
                return;
            },
            else => return err,
        }
    };

    try writer.print("Period {s} closed successfully. Archive: .invoice/close_{s}.zip\n", .{ period, period });
}

fn cmdExport(writer: anytype, allocator: std.mem.Allocator, args: []const []const u8) !void {
    const exportArgs = try cli.parseExportArgs(args);

    var period: []const u8 = "";
    const closeType: closing.CloseType = if (exportArgs.month.len > 0) .month else .year;

    if (exportArgs.month.len > 0) {
        period = exportArgs.month;
    } else if (exportArgs.year.len > 0) {
        period = exportArgs.year;
    } else {
        try writer.print("Usage: invoice export --month YYYY-MM or --year YYYY [--output DIR]\n", .{});
        return;
    }

    var database = try openDb();
    defer database.close();

    const invoices = closing.queryInvoicesForPeriod(allocator, &database, closeType, period) catch |err| {
        try writer.print("Error querying invoices: {any}\n", .{err});
        return;
    };
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
        try writer.print("No invoices found for period {s}\n", .{period});
        return;
    }

    const outputDir = if (exportArgs.output.len > 0) exportArgs.output else ".";

    std.fs.cwd().makePath(outputDir) catch |err| switch (err) {
        error.PathAlreadyExists => {},
        else => return err,
    };

    var detailPathBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    var summaryPathBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const detailPath = try std.fmt.bufPrintZ(&detailPathBuf, "{s}/明细表_{s}.xlsx", .{ outputDir, period });
    const summaryPath = try std.fmt.bufPrintZ(&summaryPathBuf, "{s}/汇总表_{s}.xlsx", .{ outputDir, period });

    try report_mod.generateDetailReport(invoices, detailPath);

    const summaryEntries = try report_mod.computeSummary(allocator, invoices);
    defer allocator.free(summaryEntries);
    try report_mod.generateSummaryReport(summaryEntries, summaryPath);

    try writer.print("Reports exported to {s}/\n", .{outputDir});
    try writer.print("  Detail: 明细表_{s}.xlsx\n", .{period});
    try writer.print("  Summary: 汇总表_{s}.xlsx\n", .{period});
}

test "version is set" {
    try std.testing.expectEqualStrings("0.1.0", version);
}
