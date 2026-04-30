const std = @import("std");
const db_mod = @import("db.zig");
const cli = @import("cli.zig");

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
            var cwdBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
            const cwd = try std.posix.getcwd(&cwdBuf);
            var database = try db_mod.initDb(cwd);
            defer database.close();
            try database.initSchema();
            try stdout.print("Initialized invoice database in {s}/.invoice/invoice.db\n", .{cwd});
        },
        .add => {
            if (args.len > 2 and std.mem.eql(u8, std.mem.sliceTo(args[2], 0), "--help")) {
                try cli.printAddHelp(stdout);
                return;
            }
            const addArgs = try cli.parseAddArgs(allocator, args[2..]);
            defer allocator.free(addArgs.attach);

            var cwdBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
            const cwd = try std.posix.getcwd(&cwdBuf);
            var database = try db_mod.initDb(cwd);
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
            try stdout.print("Invoice added with id={d}\n", .{id});
        },
        .list, .show, .edit, .delete, .close, .export_cmd => {
            try stdout.print("Command '{s}' not yet implemented\n", .{subCmd});
        },
        .unknown => {
            try stdout.print("Unknown command: {s}\n", .{subCmd});
            try cli.printHelp(stdout);
        },
    }
}

const models = @import("models.zig");

test "version is set" {
    try std.testing.expectEqualStrings("0.1.0", version);
}
