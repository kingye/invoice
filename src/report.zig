const std = @import("std");
const models = @import("models.zig");
const c = @cImport({
    @cInclude("xlsxwriter.h");
});

pub fn generateDetailReport(invoices: []const models.Invoice, outputPath: [:0]const u8) !void {
    const workbook = c.workbook_new(outputPath);
    defer _ = c.workbook_close(workbook);

    const worksheet = c.workbook_add_worksheet(workbook, "明细表");

    const headerFmt = c.workbook_add_format(workbook);
    _ = c.format_set_bold(headerFmt);
    _ = c.format_set_bg_color(headerFmt, 0x4472C4);
    _ = c.format_set_font_color(headerFmt, 0xFFFFFF);
    _ = c.format_set_border(headerFmt, 1);
    _ = c.format_set_align(headerFmt, c.LXW_ALIGN_CENTER);

    const numFmt = c.workbook_add_format(workbook);
    _ = c.format_set_num_format(numFmt, "#,##0.00");
    _ = c.format_set_border(numFmt, 1);

    const pctFmt = c.workbook_add_format(workbook);
    _ = c.format_set_num_format(pctFmt, "0%");
    _ = c.format_set_border(pctFmt, 1);

    const cellFmt = c.workbook_add_format(workbook);
    _ = c.format_set_border(cellFmt, 1);

    const headers = [_][:0]const u8{
        "序号", "发票号码", "日期", "类型", "项目名称",
        "金额", "税率", "税金", "价税合计",
        "销售方", "销售方税号", "购买方", "购买方税号",
        "分类", "备注",
    };

    for (headers, 0..) |h, col| {
        _ = c.worksheet_write_string(worksheet, 0, @intCast(col), h, headerFmt);
    }

    for (invoices, 0..) |inv, rowIdx| {
        const row: u32 = @intCast(rowIdx + 1);

        _ = c.worksheet_write_number(worksheet, row, 0, @floatFromInt(inv.id), cellFmt);

        var numBuf: [256]u8 = undefined;
        const numberZ = try std.fmt.bufPrintZ(&numBuf, "{s}", .{inv.number});
        _ = c.worksheet_write_string(worksheet, row, 1, numberZ, cellFmt);

        var dateBuf: [64]u8 = undefined;
        const dateZ = try std.fmt.bufPrintZ(&dateBuf, "{s}", .{inv.date});
        _ = c.worksheet_write_string(worksheet, row, 2, dateZ, cellFmt);

        var typeBuf: [128]u8 = undefined;
        const typeZ = try std.fmt.bufPrintZ(&typeBuf, "{s}", .{inv.type});
        _ = c.worksheet_write_string(worksheet, row, 3, typeZ, cellFmt);

        var itemBuf: [256]u8 = undefined;
        const itemZ = try std.fmt.bufPrintZ(&itemBuf, "{s}", .{inv.item_name});
        _ = c.worksheet_write_string(worksheet, row, 4, itemZ, cellFmt);

        _ = c.worksheet_write_number(worksheet, row, 5, inv.amount, numFmt);
        _ = c.worksheet_write_number(worksheet, row, 6, inv.tax_rate, pctFmt);
        _ = c.worksheet_write_number(worksheet, row, 7, inv.tax, numFmt);
        _ = c.worksheet_write_number(worksheet, row, 8, inv.total, numFmt);

        var sellerBuf: [256]u8 = undefined;
        const sellerZ = try std.fmt.bufPrintZ(&sellerBuf, "{s}", .{inv.seller_name});
        _ = c.worksheet_write_string(worksheet, row, 9, sellerZ, cellFmt);

        var sellerTaxBuf: [64]u8 = undefined;
        const sellerTaxZ = try std.fmt.bufPrintZ(&sellerTaxBuf, "{s}", .{inv.seller_tax_id});
        _ = c.worksheet_write_string(worksheet, row, 10, sellerTaxZ, cellFmt);

        var buyerBuf: [256]u8 = undefined;
        const buyerZ = try std.fmt.bufPrintZ(&buyerBuf, "{s}", .{inv.buyer_name});
        _ = c.worksheet_write_string(worksheet, row, 11, buyerZ, cellFmt);

        var buyerTaxBuf: [64]u8 = undefined;
        const buyerTaxZ = try std.fmt.bufPrintZ(&buyerTaxBuf, "{s}", .{inv.buyer_tax_id});
        _ = c.worksheet_write_string(worksheet, row, 12, buyerTaxZ, cellFmt);

        var catBuf: [128]u8 = undefined;
        const catZ = try std.fmt.bufPrintZ(&catBuf, "{s}", .{inv.category});
        _ = c.worksheet_write_string(worksheet, row, 13, catZ, cellFmt);

        var remarkBuf: [512]u8 = undefined;
        const remarkZ = try std.fmt.bufPrintZ(&remarkBuf, "{s}", .{inv.remark});
        _ = c.worksheet_write_string(worksheet, row, 14, remarkZ, cellFmt);
    }

    _ = c.worksheet_set_column(worksheet, 0, 0, 6, null);
    _ = c.worksheet_set_column(worksheet, 1, 1, 14, null);
    _ = c.worksheet_set_column(worksheet, 2, 2, 12, null);
    _ = c.worksheet_set_column(worksheet, 5, 5, 14, null);
    _ = c.worksheet_set_column(worksheet, 8, 8, 14, null);
}

pub const SummaryEntry = struct {
    category: []const u8,
    invoice_type: []const u8,
    count: u32,
    total_amount: f64,
    total_tax: f64,
    total: f64,
    weighted_tax_rate: f64,
};

pub fn generateSummaryReport(entries: []const SummaryEntry, outputPath: [:0]const u8) !void {
    const workbook = c.workbook_new(outputPath);
    defer _ = c.workbook_close(workbook);

    const worksheet = c.workbook_add_worksheet(workbook, "汇总表");

    const headerFmt = c.workbook_add_format(workbook);
    _ = c.format_set_bold(headerFmt);
    _ = c.format_set_bg_color(headerFmt, 0x70AD47);
    _ = c.format_set_font_color(headerFmt, 0xFFFFFF);
    _ = c.format_set_border(headerFmt, 1);
    _ = c.format_set_align(headerFmt, c.LXW_ALIGN_CENTER);

    const numFmt = c.workbook_add_format(workbook);
    _ = c.format_set_num_format(numFmt, "#,##0.00");
    _ = c.format_set_border(numFmt, 1);

    const pctFmt = c.workbook_add_format(workbook);
    _ = c.format_set_num_format(pctFmt, "0.00%");
    _ = c.format_set_border(pctFmt, 1);

    const cellFmt = c.workbook_add_format(workbook);
    _ = c.format_set_border(cellFmt, 1);

    const headers = [_][:0]const u8{
        "分类", "发票类型", "数量", "金额合计",
        "税金合计", "价税合计", "加权平均税率",
    };

    for (headers, 0..) |h, col| {
        _ = c.worksheet_write_string(worksheet, 0, @intCast(col), h, headerFmt);
    }

    for (entries, 0..) |entry, rowIdx| {
        const row: u32 = @intCast(rowIdx + 1);

        var catBuf: [128]u8 = undefined;
        const catZ = try std.fmt.bufPrintZ(&catBuf, "{s}", .{entry.category});
        _ = c.worksheet_write_string(worksheet, row, 0, catZ, cellFmt);

        var typeBuf: [128]u8 = undefined;
        const typeZ = try std.fmt.bufPrintZ(&typeBuf, "{s}", .{entry.invoice_type});
        _ = c.worksheet_write_string(worksheet, row, 1, typeZ, cellFmt);

        _ = c.worksheet_write_number(worksheet, row, 2, @floatFromInt(entry.count), cellFmt);
        _ = c.worksheet_write_number(worksheet, row, 3, entry.total_amount, numFmt);
        _ = c.worksheet_write_number(worksheet, row, 4, entry.total_tax, numFmt);
        _ = c.worksheet_write_number(worksheet, row, 5, entry.total, numFmt);
        _ = c.worksheet_write_number(worksheet, row, 6, entry.weighted_tax_rate, pctFmt);
    }

    _ = c.worksheet_set_column(worksheet, 0, 0, 14, null);
    _ = c.worksheet_set_column(worksheet, 1, 1, 14, null);
    _ = c.worksheet_set_column(worksheet, 3, 3, 14, null);
    _ = c.worksheet_set_column(worksheet, 5, 5, 14, null);
}

pub fn computeSummary(allocator: std.mem.Allocator, invoices: []const models.Invoice) ![]SummaryEntry {
    var map = std.StringHashMap(*SummaryEntry).init(allocator);
    defer map.deinit();

    var list = std.ArrayList(SummaryEntry).init(allocator);
    errdefer list.deinit();

    for (invoices) |inv| {
        var keyBuf: [512]u8 = undefined;
        const keyZ = try std.fmt.bufPrintZ(&keyBuf, "{s}\x00{s}", .{ inv.category, inv.type });

        if (map.get(keyZ)) |entry| {
            entry.count += 1;
            entry.total_amount += inv.amount;
            entry.total_tax += inv.tax;
            entry.total += inv.total;
        } else {
            var catBuf: [128]u8 = undefined;
            var typeBuf: [128]u8 = undefined;
            const cat = try std.fmt.bufPrint(&catBuf, "{s}", .{inv.category});
            const invType = try std.fmt.bufPrint(&typeBuf, "{s}", .{inv.type});

            const entry = try list.addOne();
            entry.* = .{
                .category = cat,
                .invoice_type = invType,
                .count = 1,
                .total_amount = inv.amount,
                .total_tax = inv.tax,
                .total = inv.total,
                .weighted_tax_rate = 0,
            };
            try map.put(keyZ, entry);
        }
    }

    for (list.items) |*entry| {
        if (entry.total_amount > 0) {
            entry.weighted_tax_rate = entry.total_tax / entry.total_amount;
        }
    }

    return try list.toOwnedSlice();
}
