use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::models;

pub fn init_db() -> Result<Connection, Box<dyn std::error::Error>> {
    let db_dir = get_db_dir()?;
    std::fs::create_dir_all(&db_dir)?;
    let db_path = db_dir.join("invoice.db");
    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

pub fn get_db_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    Ok(cwd.join(".invoice"))
}

pub fn init_schema(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );

        CREATE TABLE IF NOT EXISTS invoices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            number TEXT NOT NULL UNIQUE,
            date TEXT NOT NULL,
            type TEXT NOT NULL DEFAULT '',
            item_name TEXT NOT NULL DEFAULT '',
            amount REAL NOT NULL DEFAULT 0,
            tax_rate REAL NOT NULL DEFAULT 0,
            tax REAL NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            seller_name TEXT NOT NULL DEFAULT '',
            seller_tax_id TEXT NOT NULL DEFAULT '',
            buyer_name TEXT NOT NULL DEFAULT '',
            buyer_tax_id TEXT NOT NULL DEFAULT '',
            category TEXT NOT NULL DEFAULT '',
            remark TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS attachments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            invoice_id INTEGER NOT NULL,
            filename TEXT NOT NULL,
            filepath TEXT NOT NULL,
            file_hash TEXT NOT NULL DEFAULT '',
            file_size INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS closings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            period TEXT NOT NULL UNIQUE,
            closed_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            archive_path TEXT NOT NULL DEFAULT ''
        );

        INSERT OR IGNORE INTO schema_version (version) VALUES (1);",
    )?;
    Ok(())
}

pub fn insert_invoice(
    conn: &Connection,
    inv: &models::Invoice,
) -> Result<i64, Box<dyn std::error::Error>> {
    conn.execute(
        "INSERT INTO invoices (number, date, type, item_name, amount, tax_rate, tax, total, seller_name, seller_tax_id, buyer_name, buyer_tax_id, category, remark) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![inv.number, inv.date, inv.inv_type, inv.item_name, inv.amount, inv.tax_rate, inv.tax, inv.total, inv.seller_name, inv.seller_tax_id, inv.buyer_name, inv.buyer_tax_id, inv.category, inv.remark]
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn query_invoices(
    conn: &Connection,
    month: Option<&str>,
    year: Option<&str>,
    category: Option<&str>,
) -> Result<Vec<models::Invoice>, Box<dyn std::error::Error>> {
    let mut sql = String::from("SELECT id, number, date, type, item_name, amount, tax_rate, tax, total, seller_name, seller_tax_id, buyer_name, buyer_tax_id, category, remark, created_at, updated_at FROM invoices");
    let mut conditions: Vec<String> = Vec::new();
    let mut param_values: Vec<String> = Vec::new();

    if let Some(m) = month {
        conditions.push("date LIKE ?".to_string());
        param_values.push(format!("{}-%", m));
    }
    if let Some(y) = year {
        conditions.push("date LIKE ?".to_string());
        param_values.push(format!("{}-%", y));
    }
    if let Some(c) = category {
        conditions.push("category = ?".to_string());
        param_values.push(c.to_string());
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY date DESC, id DESC");

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = param_values
        .iter()
        .map(|p| p as &dyn rusqlite::ToSql)
        .collect();
    let invoices = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(models::Invoice {
                id: row.get(0)?,
                number: row.get(1)?,
                date: row.get(2)?,
                inv_type: row.get(3)?,
                item_name: row.get(4)?,
                amount: row.get(5)?,
                tax_rate: row.get(6)?,
                tax: row.get(7)?,
                total: row.get(8)?,
                seller_name: row.get(9)?,
                seller_tax_id: row.get(10)?,
                buyer_name: row.get(11)?,
                buyer_tax_id: row.get(12)?,
                category: row.get(13)?,
                remark: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(invoices)
}

pub fn get_invoice(
    conn: &Connection,
    id: i64,
) -> Result<Option<models::Invoice>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, number, date, type, item_name, amount, tax_rate, tax, total, seller_name, seller_tax_id, buyer_name, buyer_tax_id, category, remark, created_at, updated_at FROM invoices WHERE id = ?1"
    )?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(Some(models::Invoice {
            id: row.get(0)?,
            number: row.get(1)?,
            date: row.get(2)?,
            inv_type: row.get(3)?,
            item_name: row.get(4)?,
            amount: row.get(5)?,
            tax_rate: row.get(6)?,
            tax: row.get(7)?,
            total: row.get(8)?,
            seller_name: row.get(9)?,
            seller_tax_id: row.get(10)?,
            buyer_name: row.get(11)?,
            buyer_tax_id: row.get(12)?,
            category: row.get(13)?,
            remark: row.get(14)?,
            created_at: row.get(15)?,
            updated_at: row.get(16)?,
        })),
        None => Ok(None),
    }
}

pub fn get_invoice_number(
    conn: &Connection,
    id: i64,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT number FROM invoices WHERE id = ?1")?;
    let number: String = stmt.query_row(params![id], |row| row.get(0))?;
    Ok(number)
}

pub fn update_invoice(
    conn: &Connection,
    id: i64,
    update: &models::InvoiceUpdate,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut sets: Vec<String> = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    sets.push("updated_at = datetime('now', 'localtime')".to_string());

    if let Some(ref v) = update.number {
        sets.push("number = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(ref v) = update.date {
        sets.push("date = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(ref v) = update.inv_type {
        sets.push("type = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(ref v) = update.item_name {
        sets.push("item_name = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(v) = update.amount {
        sets.push("amount = ?".to_string());
        param_values.push(Box::new(v));
    }
    if let Some(v) = update.tax_rate {
        sets.push("tax_rate = ?".to_string());
        param_values.push(Box::new(v));
    }
    if let Some(v) = update.tax {
        sets.push("tax = ?".to_string());
        param_values.push(Box::new(v));
    }
    if let Some(v) = update.total {
        sets.push("total = ?".to_string());
        param_values.push(Box::new(v));
    }
    if let Some(ref v) = update.seller_name {
        sets.push("seller_name = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(ref v) = update.seller_tax_id {
        sets.push("seller_tax_id = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(ref v) = update.buyer_name {
        sets.push("buyer_name = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(ref v) = update.buyer_tax_id {
        sets.push("buyer_tax_id = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(ref v) = update.category {
        sets.push("category = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }
    if let Some(ref v) = update.remark {
        sets.push("remark = ?".to_string());
        param_values.push(Box::new(v.clone()));
    }

    let sql = format!("UPDATE invoices SET {} WHERE id = ?", sets.join(", "));
    param_values.push(Box::new(id));

    let param_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let changed = conn.execute(&sql, param_refs.as_slice())?;
    Ok(changed)
}

pub fn delete_invoice(conn: &Connection, id: i64) -> Result<usize, Box<dyn std::error::Error>> {
    let changed = conn.execute("DELETE FROM invoices WHERE id = ?1", params![id])?;
    Ok(changed)
}

pub fn is_period_closed(
    conn: &Connection,
    period: &str,
    close_type: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM closings WHERE type = ?1 AND period = ?2",
        params![close_type, period],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn insert_closing(
    conn: &Connection,
    close_type: &str,
    period: &str,
    archive_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute(
        "INSERT INTO closings (type, period, archive_path) VALUES (?1, ?2, ?3)",
        params![close_type, period, archive_path],
    )?;
    Ok(())
}

pub fn get_attachments_for_invoice(
    conn: &Connection,
    invoice_id: i64,
) -> Result<Vec<models::Attachment>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, invoice_id, filename, filepath, file_hash, file_size, created_at FROM attachments WHERE invoice_id = ?1 ORDER BY id"
    )?;
    let atts = stmt
        .query_map(params![invoice_id], |row| {
            Ok(models::Attachment {
                id: row.get(0)?,
                invoice_id: row.get(1)?,
                filename: row.get(2)?,
                filepath: row.get(3)?,
                file_hash: row.get(4)?,
                file_size: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(atts)
}

pub fn insert_attachment(
    conn: &Connection,
    att: &models::Attachment,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute(
        "INSERT INTO attachments (invoice_id, filename, filepath, file_hash, file_size) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![att.invoice_id, att.filename, att.filepath, att.file_hash, att.file_size]
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_db_and_schema() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join(".invoice");
        std::fs::create_dir_all(&db_path).unwrap();
        let conn = Connection::open(db_path.join("invoice.db")).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        init_schema(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM invoices", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_insert_and_query_invoice() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join(".invoice");
        std::fs::create_dir_all(&db_path).unwrap();
        let conn = Connection::open(db_path.join("invoice.db")).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        init_schema(&conn).unwrap();

        let inv = models::Invoice {
            number: "FP001".to_string(),
            date: "2026-04-01".to_string(),
            inv_type: "电子发票".to_string(),
            item_name: "技术服务".to_string(),
            amount: 1000.0,
            tax_rate: 0.06,
            tax: 60.0,
            total: 1060.0,
            seller_name: "XX公司".to_string(),
            seller_tax_id: "91110000MA01".to_string(),
            buyer_name: "YY公司".to_string(),
            buyer_tax_id: "91310000MB01".to_string(),
            category: "服务".to_string(),
            remark: "测试".to_string(),
            ..Default::default()
        };
        let id = insert_invoice(&conn, &inv).unwrap();
        assert!(id > 0);

        let fetched = get_invoice(&conn, id).unwrap().unwrap();
        assert_eq!(fetched.number, "FP001");
        assert_eq!(fetched.date, "2026-04-01");
        assert_eq!(fetched.amount, 1000.0);
    }
}
