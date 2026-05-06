const std = @import("std");
const c = @cImport({
    @cInclude("sqlite3.h");
});

pub const Db = struct {
    db: *c.sqlite3,

    pub fn open(path: [:0]const u8) !Db {
        var db: ?*c.sqlite3 = null;
        const rc = c.sqlite3_open(path, &db);
        if (rc != c.SQLITE_OK) {
            if (db) |d| {
                _ = c.sqlite3_close(d);
            }
            return error.OpenFailed;
        }
        return Db{ .db = db.? };
    }

    pub fn close(self: Db) void {
        _ = c.sqlite3_close(self.db);
    }

    pub fn exec(self: Db, sql: [:0]const u8) !void {
        var errMsg: ?[*:0]u8 = null;
        const rc = c.sqlite3_exec(self.db, sql, null, null, &errMsg);
        if (rc != c.SQLITE_OK) {
            defer if (errMsg) |msg| c.sqlite3_free(msg);
            std.log.err("SQL error: {s}", .{errMsg orelse "unknown"});
            return error.ExecFailed;
        }
    }

    pub fn prepare(self: Db, sql: [:0]const u8) !Stmt {
        var stmt: ?*c.sqlite3_stmt = null;
        const rc = c.sqlite3_prepare_v2(self.db, sql, -1, &stmt, null);
        if (rc != c.SQLITE_OK) {
            std.log.err("Prepare error: {s}", .{c.sqlite3_errmsg(self.db)});
            return error.PrepareFailed;
        }
        return Stmt{ .stmt = stmt.? };
    }

    pub fn lastInsertRowId(self: Db) i64 {
        return c.sqlite3_last_insert_rowid(self.db);
    }

    pub fn changes(self: Db) u32 {
        return @intCast(c.sqlite3_changes(self.db));
    }

    pub fn isClosed(self: Db, period: []const u8, closeType: []const u8) !bool {
        const sql = "SELECT COUNT(*) FROM closings WHERE type = ? AND period = ?";
        var typeBuf: [64]u8 = undefined;
        var periodBuf: [32]u8 = undefined;
        const typeZ = try std.fmt.bufPrintZ(&typeBuf, "{s}", .{closeType});
        const periodZ = try std.fmt.bufPrintZ(&periodBuf, "{s}", .{period});

        const stmt = try self.prepare(sql);
        defer stmt.deinit();
        try stmt.bindText(1, typeZ);
        try stmt.bindText(2, periodZ);

        if (try stmt.step()) {
            const count = stmt.columnInt(0);
            return count > 0;
        }
        return false;
    }

    pub fn initSchema(self: Db) !void {
        const ddl =
            \\CREATE TABLE IF NOT EXISTS schema_version (
            \\    version INTEGER PRIMARY KEY
            \\);
            \\
            \\CREATE TABLE IF NOT EXISTS invoices (
            \\    id INTEGER PRIMARY KEY AUTOINCREMENT,
            \\    number TEXT NOT NULL UNIQUE,
            \\    date TEXT NOT NULL,
            \\    type TEXT NOT NULL DEFAULT '',
            \\    item_name TEXT NOT NULL DEFAULT '',
            \\    amount REAL NOT NULL DEFAULT 0,
            \\    tax_rate REAL NOT NULL DEFAULT 0,
            \\    tax REAL NOT NULL DEFAULT 0,
            \\    total REAL NOT NULL DEFAULT 0,
            \\    seller_name TEXT NOT NULL DEFAULT '',
            \\    seller_tax_id TEXT NOT NULL DEFAULT '',
            \\    buyer_name TEXT NOT NULL DEFAULT '',
            \\    buyer_tax_id TEXT NOT NULL DEFAULT '',
            \\    category TEXT NOT NULL DEFAULT '',
            \\    remark TEXT NOT NULL DEFAULT '',
            \\    created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            \\    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            \\);
            \\
            \\CREATE TABLE IF NOT EXISTS attachments (
            \\    id INTEGER PRIMARY KEY AUTOINCREMENT,
            \\    invoice_id INTEGER NOT NULL,
            \\    filename TEXT NOT NULL,
            \\    filepath TEXT NOT NULL,
            \\    file_hash TEXT NOT NULL DEFAULT '',
            \\    file_size INTEGER NOT NULL DEFAULT 0,
            \\    created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            \\    FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
            \\);
            \\
            \\CREATE TABLE IF NOT EXISTS closings (
            \\    id INTEGER PRIMARY KEY AUTOINCREMENT,
            \\    type TEXT NOT NULL,
            \\    period TEXT NOT NULL UNIQUE,
            \\    closed_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            \\    archive_path TEXT NOT NULL DEFAULT ''
            \\);
            \\
            \\INSERT OR IGNORE INTO schema_version (version) VALUES (1);
        ;
        try self.exec(ddl);
    }
};

pub const Stmt = struct {
    stmt: *c.sqlite3_stmt,

    pub fn deinit(self: Stmt) void {
        _ = c.sqlite3_finalize(self.stmt);
    }

    pub fn bindInt(self: Stmt, idx: i32, val: i32) !void {
        const rc = c.sqlite3_bind_int(self.stmt, idx, val);
        if (rc != c.SQLITE_OK) return error.BindFailed;
    }

    pub fn bindInt64(self: Stmt, idx: i32, val: i64) !void {
        const rc = c.sqlite3_bind_int64(self.stmt, idx, val);
        if (rc != c.SQLITE_OK) return error.BindFailed;
    }

    pub fn bindDouble(self: Stmt, idx: i32, val: f64) !void {
        const rc = c.sqlite3_bind_double(self.stmt, idx, val);
        if (rc != c.SQLITE_OK) return error.BindFailed;
    }

    pub fn bindText(self: Stmt, idx: i32, text: [:0]const u8) !void {
        const rc = c.sqlite3_bind_text(self.stmt, idx, text, -1, c.SQLITE_TRANSIENT);
        if (rc != c.SQLITE_OK) return error.BindFailed;
    }

    pub fn bindNull(self: Stmt, idx: i32) !void {
        const rc = c.sqlite3_bind_null(self.stmt, idx);
        if (rc != c.SQLITE_OK) return error.BindFailed;
    }

    pub fn step(self: Stmt) !bool {
        const rc = c.sqlite3_step(self.stmt);
        if (rc == c.SQLITE_ROW) return true;
        if (rc == c.SQLITE_DONE) return false;
        return error.StepFailed;
    }

    pub fn reset(self: Stmt) !void {
        const rc = c.sqlite3_reset(self.stmt);
        if (rc != c.SQLITE_OK) return error.ResetFailed;
    }

    pub fn columnInt(self: Stmt, idx: i32) i32 {
        return c.sqlite3_column_int(self.stmt, idx);
    }

    pub fn columnInt64(self: Stmt, idx: i32) i64 {
        return c.sqlite3_column_int64(self.stmt, idx);
    }

    pub fn columnDouble(self: Stmt, idx: i32) f64 {
        return c.sqlite3_column_double(self.stmt, idx);
    }

    pub fn columnText(self: Stmt, idx: i32) ?[:0]const u8 {
        const ptr = c.sqlite3_column_text(self.stmt, idx);
        if (ptr == null) return null;
        const nonNull: [*:0]const u8 = @ptrCast(@alignCast(ptr));
        const slice = std.mem.sliceTo(nonNull, 0);
        return slice;
    }

    pub fn columnTextAlloc(self: Stmt, allocator: std.mem.Allocator, idx: i32) !?[:0]u8 {
        const text = self.columnText(idx);
        if (text == null) return null;
        return try allocator.dupeZ(u8, text.?);
    }
};

pub fn initDb(baseDir: []const u8) !Db {
    var dirBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const dbDir = try std.fmt.bufPrint(&dirBuf, "{s}/.invoice", .{baseDir});

    std.fs.cwd().makePath(dbDir) catch |err| switch (err) {
        error.PathAlreadyExists => {},
        else => return err,
    };

    var dbPathBuf: [std.fs.MAX_PATH_BYTES]u8 = undefined;
    const dbPath = try std.fmt.bufPrintZ(&dbPathBuf, "{s}/invoice.db", .{dbDir});

    var db = try Db.open(dbPath);
    errdefer db.close();

    try db.exec("PRAGMA journal_mode=WAL;");
    try db.exec("PRAGMA foreign_keys=ON;");

    return db;
}
