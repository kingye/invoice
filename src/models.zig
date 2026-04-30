const std = @import("std");
const db_mod = @import("db.zig");

pub const Invoice = struct {
    id: i64 = 0,
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
    created_at: []const u8 = "",
    updated_at: []const u8 = "",

    pub fn fromRowAlloc(stmt: db_mod.Stmt, allocator: std.mem.Allocator) !Invoice {
        return Invoice{
            .id = stmt.columnInt64(0),
            .number = (try stmt.columnTextAlloc(allocator, 1)) orelse "",
            .date = (try stmt.columnTextAlloc(allocator, 2)) orelse "",
            .type = (try stmt.columnTextAlloc(allocator, 3)) orelse "",
            .item_name = (try stmt.columnTextAlloc(allocator, 4)) orelse "",
            .amount = stmt.columnDouble(5),
            .tax_rate = stmt.columnDouble(6),
            .tax = stmt.columnDouble(7),
            .total = stmt.columnDouble(8),
            .seller_name = (try stmt.columnTextAlloc(allocator, 9)) orelse "",
            .seller_tax_id = (try stmt.columnTextAlloc(allocator, 10)) orelse "",
            .buyer_name = (try stmt.columnTextAlloc(allocator, 11)) orelse "",
            .buyer_tax_id = (try stmt.columnTextAlloc(allocator, 12)) orelse "",
            .category = (try stmt.columnTextAlloc(allocator, 13)) orelse "",
            .remark = (try stmt.columnTextAlloc(allocator, 14)) orelse "",
            .created_at = (try stmt.columnTextAlloc(allocator, 15)) orelse "",
            .updated_at = (try stmt.columnTextAlloc(allocator, 16)) orelse "",
        };
    }

    pub fn bindParams(self: Invoice, stmt: db_mod.Stmt) !void {
        var numBuf: [256]u8 = undefined;
        var dateBuf: [64]u8 = undefined;
        var typeBuf: [128]u8 = undefined;
        var itemBuf: [256]u8 = undefined;
        var sellerBuf: [256]u8 = undefined;
        var sellerTaxBuf: [64]u8 = undefined;
        var buyerBuf: [256]u8 = undefined;
        var buyerTaxBuf: [64]u8 = undefined;
        var catBuf: [128]u8 = undefined;
        var remarkBuf: [512]u8 = undefined;
        const numberZ = try std.fmt.bufPrintZ(&numBuf, "{s}", .{self.number});
        const dateZ = try std.fmt.bufPrintZ(&dateBuf, "{s}", .{self.date});
        const typeZ = try std.fmt.bufPrintZ(&typeBuf, "{s}", .{self.type});
        const itemNameZ = try std.fmt.bufPrintZ(&itemBuf, "{s}", .{self.item_name});
        const sellerNameZ = try std.fmt.bufPrintZ(&sellerBuf, "{s}", .{self.seller_name});
        const sellerTaxIdZ = try std.fmt.bufPrintZ(&sellerTaxBuf, "{s}", .{self.seller_tax_id});
        const buyerNameZ = try std.fmt.bufPrintZ(&buyerBuf, "{s}", .{self.buyer_name});
        const buyerTaxIdZ = try std.fmt.bufPrintZ(&buyerTaxBuf, "{s}", .{self.buyer_tax_id});
        const categoryZ = try std.fmt.bufPrintZ(&catBuf, "{s}", .{self.category});
        const remarkZ = try std.fmt.bufPrintZ(&remarkBuf, "{s}", .{self.remark});

        try stmt.bindText(1, numberZ);
        try stmt.bindText(2, dateZ);
        try stmt.bindText(3, typeZ);
        try stmt.bindText(4, itemNameZ);
        try stmt.bindDouble(5, self.amount);
        try stmt.bindDouble(6, self.tax_rate);
        try stmt.bindDouble(7, self.tax);
        try stmt.bindDouble(8, self.total);
        try stmt.bindText(9, sellerNameZ);
        try stmt.bindText(10, sellerTaxIdZ);
        try stmt.bindText(11, buyerNameZ);
        try stmt.bindText(12, buyerTaxIdZ);
        try stmt.bindText(13, categoryZ);
        try stmt.bindText(14, remarkZ);
    }
};

pub const Attachment = struct {
    id: i64 = 0,
    invoice_id: i64 = 0,
    filename: []const u8 = "",
    filepath: []const u8 = "",
    file_hash: []const u8 = "",
    file_size: i64 = 0,
    created_at: []const u8 = "",

    pub fn fromRowAlloc(stmt: db_mod.Stmt, allocator: std.mem.Allocator) !Attachment {
        return Attachment{
            .id = stmt.columnInt64(0),
            .invoice_id = stmt.columnInt64(1),
            .filename = (try stmt.columnTextAlloc(allocator, 2)) orelse "",
            .filepath = (try stmt.columnTextAlloc(allocator, 3)) orelse "",
            .file_hash = (try stmt.columnTextAlloc(allocator, 4)) orelse "",
            .file_size = stmt.columnInt64(5),
            .created_at = (try stmt.columnTextAlloc(allocator, 6)) orelse "",
        };
    }

    pub fn bindParams(self: Attachment, stmt: db_mod.Stmt) !void {
        var fnBuf: [512]u8 = undefined;
        var fpBuf: [1024]u8 = undefined;
        var hashBuf: [128]u8 = undefined;
        const filenameZ = try std.fmt.bufPrintZ(&fnBuf, "{s}", .{self.filename});
        const filepathZ = try std.fmt.bufPrintZ(&fpBuf, "{s}", .{self.filepath});
        const hashZ = try std.fmt.bufPrintZ(&hashBuf, "{s}", .{self.file_hash});

        try stmt.bindInt64(1, self.invoice_id);
        try stmt.bindText(2, filenameZ);
        try stmt.bindText(3, filepathZ);
        try stmt.bindText(4, hashZ);
        try stmt.bindInt64(5, self.file_size);
    }
};

pub const Closing = struct {
    id: i64 = 0,
    type: []const u8 = "",
    period: []const u8 = "",
    closed_at: []const u8 = "",
    archive_path: []const u8 = "",

    pub fn fromRowAlloc(stmt: db_mod.Stmt, allocator: std.mem.Allocator) !Closing {
        return Closing{
            .id = stmt.columnInt64(0),
            .type = (try stmt.columnTextAlloc(allocator, 1)) orelse "",
            .period = (try stmt.columnTextAlloc(allocator, 2)) orelse "",
            .closed_at = (try stmt.columnTextAlloc(allocator, 3)) orelse "",
            .archive_path = (try stmt.columnTextAlloc(allocator, 4)) orelse "",
        };
    }

    pub fn bindParams(self: Closing, stmt: db_mod.Stmt) !void {
        var typeBuf: [64]u8 = undefined;
        var periodBuf: [32]u8 = undefined;
        var archiveBuf: [1024]u8 = undefined;
        const typeZ = try std.fmt.bufPrintZ(&typeBuf, "{s}", .{self.type});
        const periodZ = try std.fmt.bufPrintZ(&periodBuf, "{s}", .{self.period});
        const archiveZ = try std.fmt.bufPrintZ(&archiveBuf, "{s}", .{self.archive_path});

        try stmt.bindText(1, typeZ);
        try stmt.bindText(2, periodZ);
        try stmt.bindText(3, archiveZ);
    }
};

test "Invoice field consistency" {
    const inv = Invoice{
        .id = 1,
        .number = "FP001",
        .date = "2026-04-01",
        .type = "电子发票",
        .item_name = "技术服务",
        .amount = 1000,
        .tax_rate = 0.06,
        .tax = 60,
        .total = 1060,
        .seller_name = "XX公司",
        .seller_tax_id = "91110000MA01",
        .buyer_name = "YY公司",
        .buyer_tax_id = "91310000MB01",
        .category = "服务",
        .remark = "测试",
    };

    try std.testing.expectEqualStrings("FP001", inv.number);
    try std.testing.expectEqualStrings("2026-04-01", inv.date);
    try std.testing.expectEqualStrings("电子发票", inv.type);
    try std.testing.expectEqualStrings("技术服务", inv.item_name);
    try std.testing.expectEqual(@as(f64, 1000), inv.amount);
    try std.testing.expectEqual(@as(f64, 0.06), inv.tax_rate);
    try std.testing.expectEqual(@as(f64, 60), inv.tax);
    try std.testing.expectEqual(@as(f64, 1060), inv.total);
    try std.testing.expectEqualStrings("XX公司", inv.seller_name);
    try std.testing.expectEqualStrings("91110000MA01", inv.seller_tax_id);
    try std.testing.expectEqualStrings("YY公司", inv.buyer_name);
    try std.testing.expectEqualStrings("91310000MB01", inv.buyer_tax_id);
    try std.testing.expectEqualStrings("服务", inv.category);
    try std.testing.expectEqualStrings("测试", inv.remark);
}

test "Attachment fields" {
    const att = Attachment{
        .id = 1,
        .invoice_id = 1,
        .filename = "test.pdf",
        .filepath = ".invoice/data/FP001/test.pdf",
        .file_hash = "abc123",
        .file_size = 1024,
    };

    try std.testing.expectEqualStrings("test.pdf", att.filename);
    try std.testing.expectEqual(@as(i64, 1024), att.file_size);
}

test "Closing fields" {
    const cl = Closing{
        .id = 1,
        .type = "month",
        .period = "2026-04",
        .closed_at = "2026-04-30 00:00:00",
        .archive_path = "close_2026-04.zip",
    };

    try std.testing.expectEqualStrings("month", cl.type);
    try std.testing.expectEqualStrings("2026-04", cl.period);
}
