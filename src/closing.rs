use rusqlite::params;

use crate::archive;
use crate::db;
use crate::models;
use crate::report;

#[derive(Debug, Clone, Copy)]
pub enum CloseType {
    Month,
    Year,
}

impl CloseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloseType::Month => "month",
            CloseType::Year => "year",
        }
    }
}

pub fn close_period(
    conn: &rusqlite::Connection,
    close_type: CloseType,
    period: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let type_str = close_type.as_str();

    if db::is_period_closed(conn, period, type_str)? {
        return Err(format!("Period {} ({}) is already closed", period, type_str).into());
    }

    let invoices = query_invoices_for_period(conn, close_type, period)?;
    if invoices.is_empty() {
        return Err(format!("No invoices found for period {}", period).into());
    }

    let cwd = std::env::current_dir()?;
    let detail_path = cwd.join(format!(".invoice/明细表_{}.xlsx", period));
    let summary_path = cwd.join(format!(".invoice/汇总表_{}.xlsx", period));

    report::generate_detail_report(&invoices, detail_path.to_str().unwrap_or(""))?;
    let summary_entries = report::compute_summary(&invoices);
    report::generate_summary_report(&summary_entries, summary_path.to_str().unwrap_or(""))?;

    let mut att_list: Vec<archive::AttachmentEntry> = Vec::new();
    for inv in &invoices {
        let atts = db::get_attachments_for_invoice(conn, inv.id)?;
        for att in atts {
            att_list.push(archive::AttachmentEntry {
                invoice_number: inv.number.clone(),
                filepath: att.filepath.clone(),
                filename: att.filename.clone(),
            });
        }
    }

    let archive_path = cwd.join(format!(".invoice/close_{}.zip", period));
    archive::create_archive(
        detail_path.to_str().unwrap_or(""),
        summary_path.to_str().unwrap_or(""),
        &att_list,
        archive_path.to_str().unwrap_or(""),
    )?;

    let db_archive_path = format!(".invoice/close_{}.zip", period);
    db::insert_closing(conn, type_str, period, &db_archive_path)?;

    Ok(())
}

pub fn check_invoice_closed(
    conn: &rusqlite::Connection,
    invoice_id: i64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let date: Option<String> = conn
        .query_row(
            "SELECT date FROM invoices WHERE id = ?1",
            params![invoice_id],
            |row| row.get(0),
        )
        .ok();

    match date {
        Some(d) => check_period_closed(conn, &d),
        None => Ok(false),
    }
}

pub fn check_period_closed(
    conn: &rusqlite::Connection,
    date: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    if date.len() >= 7 {
        let month = &date[..7];
        if db::is_period_closed(conn, month, "month")? {
            return Ok(true);
        }
    }
    if date.len() >= 4 {
        let year = &date[..4];
        if db::is_period_closed(conn, year, "year")? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn query_invoices_for_period(
    conn: &rusqlite::Connection,
    _close_type: CloseType,
    period: &str,
) -> Result<Vec<models::Invoice>, Box<dyn std::error::Error>> {
    let pattern = format!("{}-%", period);
    let mut stmt = conn.prepare(
        "SELECT id, number, date, type, item_name, amount, tax_rate, tax, total, seller_name, seller_tax_id, buyer_name, buyer_tax_id, category, remark, created_at, updated_at FROM invoices WHERE date LIKE ?1 ORDER BY date, id"
    )?;
    let invoices = stmt
        .query_map(params![pattern], |row| {
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_close_type_as_str() {
        assert_eq!(super::CloseType::Month.as_str(), "month");
        assert_eq!(super::CloseType::Year.as_str(), "year");
    }
}
