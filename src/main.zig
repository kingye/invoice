const std = @import("std");
const db_mod = @import("db.zig");
const models = @import("models.zig");
const cli = @import("cli.zig");
const attachment = @import("attachment.zig");

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
            try stdout.print("Command 'close' not yet implemented\n", .{});
        },
        .export_cmd => {
            try stdout.print("Command 'export' not yet implemented\n", .{});
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

    var hasWhere = false;
    if (listArgs.month.len > 0) {
        if (!hasWhere) try sqlList.appendSlice(" WHERE");
        try sqlList.writer().print(" date LIKE '{s}-%'", .{listArgs.month});
        hasWhere = true;
    }
    if (listArgs.year.len > 0) {
        if (!hasWhere) {
            try sqlList.appendSlice(" WHERE");
        } else {
            try sqlList.appendSlice(" AND");
        }
        try sqlList.writer().print(" date LIKE '{s}-%'", .{listArgs.year});
        hasWhere = true;
    }
    if (listArgs.category.len > 0) {
        if (!hasWhere) {
            try sqlList.appendSlice(" WHERE");
        } else {
            try sqlList.appendSlice(" AND");
        }
        try sqlList.writer().print(" category = '{s}'", .{listArgs.category});
        hasWhere = true;
    }

    try sqlList.appendSlice(" ORDER BY date DESC, id DESC");

    const sqlZ = try sqlList.toOwnedSliceSentinel(0);
    defer allocator.free(sqlZ);

    const stmt = try database.prepare(sqlZ);
    defer stmt.deinit();

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

    var sqlList = std.ArrayList(u8).init(allocator);
    defer sqlList.deinit();

    try sqlList.appendSlice("UPDATE invoices SET updated_at = datetime('now', 'localtime')");

    if (editArgs.number) |v| try sqlList.writer().print(", number = '{s}'", .{v});
    if (editArgs.date) |v| try sqlList.writer().print(", date = '{s}'", .{v});
    if (editArgs.type) |v| try sqlList.writer().print(", type = '{s}'", .{v});
    if (editArgs.item_name) |v| try sqlList.writer().print(", item_name = '{s}'", .{v});
    if (editArgs.amount) |v| try sqlList.writer().print(", amount = {d}", .{v});
    if (editArgs.tax_rate) |v| try sqlList.writer().print(", tax_rate = {d}", .{v});
    if (editArgs.tax) |v| try sqlList.writer().print(", tax = {d}", .{v});
    if (editArgs.total) |v| try sqlList.writer().print(", total = {d}", .{v});
    if (editArgs.seller_name) |v| try sqlList.writer().print(", seller_name = '{s}'", .{v});
    if (editArgs.seller_tax_id) |v| try sqlList.writer().print(", seller_tax_id = '{s}'", .{v});
    if (editArgs.buyer_name) |v| try sqlList.writer().print(", buyer_name = '{s}'", .{v});
    if (editArgs.buyer_tax_id) |v| try sqlList.writer().print(", buyer_tax_id = '{s}'", .{v});
    if (editArgs.category) |v| try sqlList.writer().print(", category = '{s}'", .{v});
    if (editArgs.remark) |v| try sqlList.writer().print(", remark = '{s}'", .{v});

    try sqlList.writer().print(" WHERE id = {d}", .{editArgs.id});

    const sqlZ = try sqlList.toOwnedSliceSentinel(0);
    defer allocator.free(sqlZ);

    try database.exec(sqlZ);

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

    var sqlBuf: [256]u8 = undefined;
    const sql = try std.fmt.bufPrintZ(&sqlBuf, "DELETE FROM invoices WHERE id = {d}", .{deleteArgs.id});
    try database.exec(sql);

    const changed = database.changes();
    if (changed > 0) {
        try writer.print("Invoice #{d} deleted\n", .{deleteArgs.id});
    } else {
        try writer.print("Invoice #{d} not found\n", .{deleteArgs.id});
    }
}

test "version is set" {
    try std.testing.expectEqualStrings("0.1.0", version);
}
