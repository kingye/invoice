const std = @import("std");
const db_mod = @import("db.zig");
const models = @import("models.zig");

pub fn addAttachment(allocator: std.mem.Allocator, database: *db_mod.Db, invoiceId: i64, invoiceNumber: []const u8, basePath: []const u8, filePath: []const u8) !void {
    var dataDirBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const dataDir = try std.fmt.bufPrint(&dataDirBuf, "{s}/.invoice/data/{s}", .{ basePath, invoiceNumber });
    try std.fs.cwd().makePath(dataDir);

    const srcFile = std.fs.cwd().openFile(filePath, .{}) catch |err| {
        std.log.err("Cannot open file '{s}': {any}", .{ filePath, err });
        return err;
    };
    defer srcFile.close();

    const filename = std.fs.path.basename(filePath);

    var destPathBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const destPath = try std.fmt.bufPrint(&destPathBuf, "{s}/.invoice/data/{s}/{s}", .{ basePath, invoiceNumber, filename });

    const destFile = try std.fs.cwd().createFile(destPath, .{});
    defer destFile.close();

    var buf: [4096]u8 = undefined;
    var hasher = std.crypto.hash.sha2.Sha256.init(.{});
    var totalWritten: u64 = 0;

    while (true) {
        const bytesRead = try srcFile.read(&buf);
        if (bytesRead == 0) break;
        try destFile.writeAll(buf[0..bytesRead]);
        hasher.update(buf[0..bytesRead]);
        totalWritten += bytesRead;
    }

    var hashBuf: [128]u8 = undefined;
    const digest = hasher.finalResult();
    const hashHex = try std.fmt.bufPrintZ(&hashBuf, "{}", .{std.fmt.fmtSliceHexLower(&digest)});

    var dbPathBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const dbPath = try std.fmt.bufPrintZ(&dbPathBuf, ".invoice/data/{s}/{s}", .{ invoiceNumber, filename });

    const sql = "INSERT INTO attachments (invoice_id, filename, filepath, file_hash, file_size) VALUES (?, ?, ?, ?, ?)";
    const stmt = try database.prepare(sql);
    defer stmt.deinit();

    var fnBuf: [512]u8 = undefined;
    const filenameZ = try std.fmt.bufPrintZ(&fnBuf, "{s}", .{filename});

    try stmt.bindInt64(1, invoiceId);
    try stmt.bindText(2, filenameZ);
    try stmt.bindText(3, dbPath);
    try stmt.bindText(4, hashHex);
    try stmt.bindInt64(5, @intCast(totalWritten));

    const rowDone = try stmt.step();
    if (rowDone) return error.UnexpectedRow;

    _ = allocator;
}

pub fn getAttachmentsForInvoice(database: *db_mod.Db, allocator: std.mem.Allocator, invoiceId: i64) ![]models.Attachment {
    var list = std.ArrayList(models.Attachment).init(allocator);
    errdefer list.deinit();

    const sql = "SELECT id, invoice_id, filename, filepath, file_hash, file_size, created_at FROM attachments WHERE invoice_id = ? ORDER BY id";
    const stmt = try database.prepare(sql);
    defer stmt.deinit();
    try stmt.bindInt64(1, invoiceId);

    while (try stmt.step()) {
        const att = try models.Attachment.fromRowAlloc(stmt, allocator);
        try list.append(att);
    }

    return try list.toOwnedSlice();
}
