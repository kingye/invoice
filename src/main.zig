const std = @import("std");
const db = @import("db.zig");

pub const version = "0.1.0";

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    const allocator = std.heap.page_allocator;

    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    if (args.len > 1) {
        const sub = std.mem.sliceTo(args[1], 0);
        if (std.mem.eql(u8, sub, "--version") or std.mem.eql(u8, sub, "-v")) {
            try stdout.print("invoice v{s}\n", .{version});
            return;
        }
        if (std.mem.eql(u8, sub, "init")) {
            var cwdBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
            const cwd = try std.posix.getcwd(&cwdBuf);
            var database = try db.initDb(cwd);
            defer database.close();
            try database.initSchema();
            try stdout.print("Initialized invoice database in {s}/.invoice/invoice.db\n", .{cwd});
            return;
        }
    }

    try stdout.print("invoice v{s}\n", .{version});
    try stdout.print("Usage: invoice <command> [options]\n", .{});
    try stdout.print("Commands: init, add, list, show, edit, delete, close, export\n", .{});
}

test "version is set" {
    try std.testing.expectEqualStrings("0.1.0", version);
}
