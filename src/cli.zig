const std = @import("std");
const models = @import("models.zig");

pub const Command = enum {
    init,
    add,
    list,
    show,
    edit,
    delete,
    close,
    export_cmd,
    help,
    version,
    unknown,
};

pub const AddArgs = struct {
    number: []const u8 = "",
    date: []const u8 = "",
    type: []const u8 = "",
    item_name: []const u8 = "",
    amount: f64 = 0,
    tax_rate: f64 = 0,
    tax: f64 = 0,
    total: f64 = 0,
    seller_name: []const u8 = "",
    seller_tax_id: []const u8 = "",
    buyer_name: []const u8 = "",
    buyer_tax_id: []const u8 = "",
    category: []const u8 = "",
    remark: []const u8 = "",
    attach: []const []const u8 = &.{},
};

pub const ListArgs = struct {
    month: []const u8 = "",
    year: []const u8 = "",
    category: []const u8 = "",
};

pub const ShowArgs = struct {
    id: i64 = 0,
};

pub const EditArgs = struct {
    id: i64 = 0,
    number: ?[]const u8 = null,
    date: ?[]const u8 = null,
    type: ?[]const u8 = null,
    item_name: ?[]const u8 = null,
    amount: ?f64 = null,
    tax_rate: ?f64 = null,
    tax: ?f64 = null,
    total: ?f64 = null,
    seller_name: ?[]const u8 = null,
    seller_tax_id: ?[]const u8 = null,
    buyer_name: ?[]const u8 = null,
    buyer_tax_id: ?[]const u8 = null,
    category: ?[]const u8 = null,
    remark: ?[]const u8 = null,
    attach: []const []const u8 = &.{},
};

pub const DeleteArgs = struct {
    id: i64 = 0,
};

pub const CloseArgs = struct {
    month: []const u8 = "",
    year: []const u8 = "",
};

pub const ExportArgs = struct {
    month: []const u8 = "",
    year: []const u8 = "",
    output: []const u8 = ".",
};

pub fn parseCommand(sub: []const u8) Command {
    if (std.mem.eql(u8, sub, "init")) return .init;
    if (std.mem.eql(u8, sub, "add")) return .add;
    if (std.mem.eql(u8, sub, "list")) return .list;
    if (std.mem.eql(u8, sub, "show")) return .show;
    if (std.mem.eql(u8, sub, "edit")) return .edit;
    if (std.mem.eql(u8, sub, "delete")) return .delete;
    if (std.mem.eql(u8, sub, "close")) return .close;
    if (std.mem.eql(u8, sub, "export")) return .export_cmd;
    if (std.mem.eql(u8, sub, "help")) return .help;
    if (std.mem.eql(u8, sub, "--version") or std.mem.eql(u8, sub, "-v")) return .version;
    return .unknown;
}

pub fn parseAddArgs(allocator: std.mem.Allocator, args: []const []const u8) !AddArgs {
    var result = AddArgs{};
    var attachList = std.ArrayList([]const u8).init(allocator);
    errdefer attachList.deinit();

    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        const arg = args[i];
        if (std.mem.eql(u8, arg, "--number") and i + 1 < args.len) {
            i += 1;
            result.number = args[i];
        } else if (std.mem.eql(u8, arg, "--date") and i + 1 < args.len) {
            i += 1;
            result.date = args[i];
        } else if (std.mem.eql(u8, arg, "--type") and i + 1 < args.len) {
            i += 1;
            result.type = args[i];
        } else if (std.mem.eql(u8, arg, "--item") and i + 1 < args.len) {
            i += 1;
            result.item_name = args[i];
        } else if (std.mem.eql(u8, arg, "--amount") and i + 1 < args.len) {
            i += 1;
            result.amount = try std.fmt.parseFloat(f64, args[i]);
        } else if (std.mem.eql(u8, arg, "--tax-rate") and i + 1 < args.len) {
            i += 1;
            result.tax_rate = try std.fmt.parseFloat(f64, args[i]);
        } else if (std.mem.eql(u8, arg, "--tax") and i + 1 < args.len) {
            i += 1;
            result.tax = try std.fmt.parseFloat(f64, args[i]);
        } else if (std.mem.eql(u8, arg, "--total") and i + 1 < args.len) {
            i += 1;
            result.total = try std.fmt.parseFloat(f64, args[i]);
        } else if (std.mem.eql(u8, arg, "--seller") and i + 1 < args.len) {
            i += 1;
            result.seller_name = args[i];
        } else if (std.mem.eql(u8, arg, "--seller-tax") and i + 1 < args.len) {
            i += 1;
            result.seller_tax_id = args[i];
        } else if (std.mem.eql(u8, arg, "--buyer") and i + 1 < args.len) {
            i += 1;
            result.buyer_name = args[i];
        } else if (std.mem.eql(u8, arg, "--buyer-tax") and i + 1 < args.len) {
            i += 1;
            result.buyer_tax_id = args[i];
        } else if (std.mem.eql(u8, arg, "--category") and i + 1 < args.len) {
            i += 1;
            result.category = args[i];
        } else if (std.mem.eql(u8, arg, "--remark") and i + 1 < args.len) {
            i += 1;
            result.remark = args[i];
        } else if (std.mem.eql(u8, arg, "--attach") and i + 1 < args.len) {
            i += 1;
            try attachList.append(args[i]);
        }
    }
    result.attach = try attachList.toOwnedSlice();
    return result;
}

pub fn parseListArgs(args: []const []const u8) !ListArgs {
    var result = ListArgs{};
    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--month") and i + 1 < args.len) {
            i += 1;
            result.month = args[i];
        } else if (std.mem.eql(u8, args[i], "--year") and i + 1 < args.len) {
            i += 1;
            result.year = args[i];
        } else if (std.mem.eql(u8, args[i], "--category") and i + 1 < args.len) {
            i += 1;
            result.category = args[i];
        }
    }
    return result;
}

pub fn parseShowArgs(args: []const []const u8) !ShowArgs {
    if (args.len > 0) {
        return ShowArgs{ .id = try std.fmt.parseInt(i64, args[0], 10) };
    }
    return ShowArgs{};
}

pub fn parseEditArgs(allocator: std.mem.Allocator, args: []const []const u8) !EditArgs {
    var result = EditArgs{};
    var attachList = std.ArrayList([]const u8).init(allocator);
    errdefer attachList.deinit();

    if (args.len > 0) {
        result.id = try std.fmt.parseInt(i64, args[0], 10);
    }

    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        const arg = args[i];
        if (std.mem.eql(u8, arg, "--number") and i + 1 < args.len) {
            i += 1;
            result.number = args[i];
        } else if (std.mem.eql(u8, arg, "--date") and i + 1 < args.len) {
            i += 1;
            result.date = args[i];
        } else if (std.mem.eql(u8, arg, "--type") and i + 1 < args.len) {
            i += 1;
            result.type = args[i];
        } else if (std.mem.eql(u8, arg, "--item") and i + 1 < args.len) {
            i += 1;
            result.item_name = args[i];
        } else if (std.mem.eql(u8, arg, "--amount") and i + 1 < args.len) {
            i += 1;
            result.amount = try std.fmt.parseFloat(f64, args[i]);
        } else if (std.mem.eql(u8, arg, "--tax-rate") and i + 1 < args.len) {
            i += 1;
            result.tax_rate = try std.fmt.parseFloat(f64, args[i]);
        } else if (std.mem.eql(u8, arg, "--tax") and i + 1 < args.len) {
            i += 1;
            result.tax = try std.fmt.parseFloat(f64, args[i]);
        } else if (std.mem.eql(u8, arg, "--total") and i + 1 < args.len) {
            i += 1;
            result.total = try std.fmt.parseFloat(f64, args[i]);
        } else if (std.mem.eql(u8, arg, "--seller") and i + 1 < args.len) {
            i += 1;
            result.seller_name = args[i];
        } else if (std.mem.eql(u8, arg, "--seller-tax") and i + 1 < args.len) {
            i += 1;
            result.seller_tax_id = args[i];
        } else if (std.mem.eql(u8, arg, "--buyer") and i + 1 < args.len) {
            i += 1;
            result.buyer_name = args[i];
        } else if (std.mem.eql(u8, arg, "--buyer-tax") and i + 1 < args.len) {
            i += 1;
            result.buyer_tax_id = args[i];
        } else if (std.mem.eql(u8, arg, "--category") and i + 1 < args.len) {
            i += 1;
            result.category = args[i];
        } else if (std.mem.eql(u8, arg, "--remark") and i + 1 < args.len) {
            i += 1;
            result.remark = args[i];
        } else if (std.mem.eql(u8, arg, "--attach") and i + 1 < args.len) {
            i += 1;
            try attachList.append(args[i]);
        }
    }
    result.attach = try attachList.toOwnedSlice();
    return result;
}

pub fn parseDeleteArgs(args: []const []const u8) !DeleteArgs {
    if (args.len > 0) {
        return DeleteArgs{ .id = try std.fmt.parseInt(i64, args[0], 10) };
    }
    return DeleteArgs{};
}

pub fn parseCloseArgs(args: []const []const u8) !CloseArgs {
    var result = CloseArgs{};
    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--month") and i + 1 < args.len) {
            i += 1;
            result.month = args[i];
        } else if (std.mem.eql(u8, args[i], "--year") and i + 1 < args.len) {
            i += 1;
            result.year = args[i];
        }
    }
    return result;
}

pub fn parseExportArgs(args: []const []const u8) !ExportArgs {
    var result = ExportArgs{};
    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--month") and i + 1 < args.len) {
            i += 1;
            result.month = args[i];
        } else if (std.mem.eql(u8, args[i], "--year") and i + 1 < args.len) {
            i += 1;
            result.year = args[i];
        } else if (std.mem.eql(u8, args[i], "--output") and i + 1 < args.len) {
            i += 1;
            result.output = args[i];
        }
    }
    return result;
}

pub fn printHelp(writer: anytype) !void {
    try writer.print("invoice v{s} - 轻量级命令行记账系统\n\n", .{@import("main.zig").version});
    try writer.print("Usage: invoice <command> [options]\n\n", .{});
    try writer.print("Commands:\n", .{});
    try writer.print("  init              Initialize invoice database\n", .{});
    try writer.print("  add               Add a new invoice\n", .{});
    try writer.print("  list              List invoices\n", .{});
    try writer.print("  show <id>         Show invoice details\n", .{});
    try writer.print("  edit <id>         Edit invoice\n", .{});
    try writer.print("  delete <id>       Delete invoice\n", .{});
    try writer.print("  close             Close a period (month/year)\n", .{});
    try writer.print("  export            Export reports without closing\n", .{});
    try writer.print("  help              Show this help\n", .{});
    try writer.print("  --version, -v     Show version\n", .{});
}

pub fn printAddHelp(writer: anytype) !void {
    try writer.print("Usage: invoice add [options]\n\n", .{});
    try writer.print("Options:\n", .{});
    try writer.print("  --number <text>     Invoice number (required)\n", .{});
    try writer.print("  --date <YYYY-MM-DD> Invoice date (required)\n", .{});
    try writer.print("  --type <text>       Invoice type\n", .{});
    try writer.print("  --item <text>       Item name\n", .{});
    try writer.print("  --amount <number>   Amount (before tax)\n", .{});
    try writer.print("  --tax-rate <number> Tax rate (e.g. 0.06 for 6%%)\n", .{});
    try writer.print("  --tax <number>      Tax amount\n", .{});
    try writer.print("  --total <number>    Total (amount + tax)\n", .{});
    try writer.print("  --seller <text>     Seller name\n", .{});
    try writer.print("  --seller-tax <text> Seller tax ID\n", .{});
    try writer.print("  --buyer <text>      Buyer name\n", .{});
    try writer.print("  --buyer-tax <text>  Buyer tax ID\n", .{});
    try writer.print("  --category <text>   Category\n", .{});
    try writer.print("  --remark <text>     Remark\n", .{});
    try writer.print("  --attach <path>     Attach file (can repeat)\n", .{});
}

test "parseCommand" {
    try std.testing.expectEqual(Command.init, parseCommand("init"));
    try std.testing.expectEqual(Command.add, parseCommand("add"));
    try std.testing.expectEqual(Command.list, parseCommand("list"));
    try std.testing.expectEqual(Command.show, parseCommand("show"));
    try std.testing.expectEqual(Command.edit, parseCommand("edit"));
    try std.testing.expectEqual(Command.delete, parseCommand("delete"));
    try std.testing.expectEqual(Command.close, parseCommand("close"));
    try std.testing.expectEqual(Command.export_cmd, parseCommand("export"));
    try std.testing.expectEqual(Command.help, parseCommand("help"));
    try std.testing.expectEqual(Command.version, parseCommand("--version"));
    try std.testing.expectEqual(Command.unknown, parseCommand("foo"));
}

test "parseAddArgs basic" {
    const allocator = std.testing.allocator;
    const args = &[_][]const u8{ "--number", "FP001", "--date", "2026-01-01", "--amount", "1000", "--total", "1060" };
    const result = try parseAddArgs(allocator, args);
    defer allocator.free(result.attach);
    try std.testing.expectEqualStrings("FP001", result.number);
    try std.testing.expectEqualStrings("2026-01-01", result.date);
    try std.testing.expectEqual(@as(f64, 1000), result.amount);
    try std.testing.expectEqual(@as(f64, 1060), result.total);
}
