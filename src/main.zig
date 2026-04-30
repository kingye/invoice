const std = @import("std");

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
    }

    try stdout.print("invoice v{s}\n", .{version});
}

test "version is set" {
    try std.testing.expectEqualStrings("0.1.0", version);
}
