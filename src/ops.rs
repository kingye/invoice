use rusqlite::Connection;
use std::fmt;

use crate::attachment;
use crate::closing;
use crate::db;
use crate::import;
use crate::models;
use crate::report;

#[derive(Debug)]
pub enum InvoiceError {
    ClosedPeriod(String),
    AlreadyClosed(String),
    NoInvoices(String),
    Other(Box<dyn std::error::Error>),
}

impl fmt::Display for InvoiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvoiceError::ClosedPeriod(msg) => write!(f, "{}", msg),
            InvoiceError::AlreadyClosed(msg) => write!(f, "{}", msg),
            InvoiceError::NoInvoices(msg) => write!(f, "{}", msg),
            InvoiceError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for InvoiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InvoiceError::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

fn is_period_already_closed_err(e: &dyn std::error::Error) -> bool {
    e.to_string().contains("already closed")
}

fn is_no_invoices_err(e: &dyn std::error::Error) -> bool {
    e.to_string().contains("No invoices")
}

pub fn open_db() -> Result<Connection, Box<dyn std::error::Error>> {
    let conn = db::init_db()?;
    db::init_schema(&conn)?;
    Ok(conn)
}

pub fn init_database() -> Result<String, Box<dyn std::error::Error>> {
    let conn = db::init_db()?;
    db::init_schema(&conn)?;
    let cwd = std::env::current_dir()?;
    let mut result = format!(
        "Initialized invoice database in {}/.invoice/invoice.db",
        cwd.display()
    );

    if crate::ocr::model_files_exist() {
        result.push_str(&format!(
            "\nOCR models already exist at {}",
            crate::ocr::ocr_model_dir().display()
        ));
    } else {
        match crate::ocr::download_models() {
            Ok(()) => {
                result.push_str(&format!(
                    "\nOCR models downloaded to {}",
                    crate::ocr::ocr_model_dir().display()
                ));
            }
            Err(e) => {
                result.push_str(&format!("\nWarning: Failed to download OCR models: {}", e));
                result.push_str("\nYou can manually download them later with `invoice init`");
            }
        }
    }

    Ok(result)
}

#[allow(clippy::too_many_arguments)]
pub fn add_invoice(
    conn: &Connection,
    number: &str,
    date: &str,
    inv_type: &str,
    item_name: &str,
    amount: f64,
    tax_rate: f64,
    tax: f64,
    total: f64,
    seller_name: &str,
    seller_tax_id: &str,
    buyer_name: &str,
    buyer_tax_id: &str,
    category: &str,
    remark: &str,
    attach_paths: &[String],
) -> Result<i64, Box<dyn std::error::Error>> {
    let inv = models::Invoice {
        number: number.to_string(),
        date: date.to_string(),
        inv_type: inv_type.to_string(),
        item_name: item_name.to_string(),
        amount,
        tax_rate,
        tax,
        total,
        seller_name: seller_name.to_string(),
        seller_tax_id: seller_tax_id.to_string(),
        buyer_name: buyer_name.to_string(),
        buyer_tax_id: buyer_tax_id.to_string(),
        category: category.to_string(),
        remark: remark.to_string(),
        ..Default::default()
    };
    let id = db::insert_invoice(conn, &inv)?;
    for att_path in attach_paths {
        if let Err(e) = attachment::add_attachment(conn, id, number, att_path) {
            eprintln!("Failed to add attachment '{}': {}", att_path, e);
        }
    }
    Ok(id)
}

pub fn list_invoices(
    conn: &Connection,
    month: Option<&str>,
    year: Option<&str>,
    category: Option<&str>,
) -> Result<Vec<models::Invoice>, Box<dyn std::error::Error>> {
    Ok(db::query_invoices(conn, month, year, category)?)
}

pub fn get_invoice(
    conn: &Connection,
    id: i64,
) -> Result<Option<models::Invoice>, Box<dyn std::error::Error>> {
    Ok(db::get_invoice(conn, id)?)
}

pub fn get_attachments(
    conn: &Connection,
    invoice_id: i64,
) -> Result<Vec<models::Attachment>, Box<dyn std::error::Error>> {
    Ok(db::get_attachments_for_invoice(conn, invoice_id)?)
}

#[allow(clippy::too_many_arguments)]
pub fn edit_invoice(
    conn: &Connection,
    id: i64,
    number: Option<String>,
    date: Option<String>,
    inv_type: Option<String>,
    item_name: Option<String>,
    amount: Option<f64>,
    tax_rate: Option<f64>,
    tax: Option<f64>,
    total: Option<f64>,
    seller_name: Option<String>,
    seller_tax_id: Option<String>,
    buyer_name: Option<String>,
    buyer_tax_id: Option<String>,
    category: Option<String>,
    remark: Option<String>,
    attach_paths: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    if closing::check_invoice_closed(conn, id)? {
        return Err(Box::new(InvoiceError::ClosedPeriod(
            "Invoice is in a closed period and cannot be modified".to_string(),
        )));
    }

    let updates = models::InvoiceUpdate {
        number,
        date,
        inv_type,
        item_name,
        amount,
        tax_rate,
        tax,
        total,
        seller_name,
        seller_tax_id,
        buyer_name,
        buyer_tax_id,
        category,
        remark,
    };
    let changed = db::update_invoice(conn, id, &updates)?;

    for att_path in attach_paths {
        if let Ok(inv_number) = db::get_invoice_number(conn, id) {
            match attachment::add_attachment(conn, id, &inv_number, att_path) {
                Ok(()) => {}
                Err(e) => eprintln!("Failed to add attachment '{}': {}", att_path, e),
            }
        }
    }
    Ok(changed)
}

pub fn delete_invoice(conn: &Connection, id: i64) -> Result<usize, Box<dyn std::error::Error>> {
    if closing::check_invoice_closed(conn, id)? {
        return Err(Box::new(InvoiceError::ClosedPeriod(
            "Invoice is in a closed period and cannot be deleted".to_string(),
        )));
    }
    Ok(db::delete_invoice(conn, id)?)
}

pub fn import_invoice(
    path: &str,
    category: Option<&str>,
    remark: Option<&str>,
    ocr_model_dir: Option<&str>,
) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let ocr_dir = match ocr_model_dir {
        Some(dir) => Some(dir.to_string()),
        None if crate::ocr::model_files_exist() => Some(
            crate::ocr::ocr_model_dir()
                .to_str()
                .unwrap_or("")
                .to_string(),
        ),
        None => None,
    };

    let extracted = import::extract_invoice_with_ocr(path, ocr_dir.as_deref())?;
    let mut inv = extracted;

    if inv.number.is_empty() && inv.seller_name.is_empty() && ocr_dir.is_none() {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext == "pdf" && !crate::ocr::model_files_exist() {
            eprintln!(
                "Hint: No text extracted from PDF. OCR models not found — run `invoice init` to download them."
            );
        }
    }
    if let Some(cat) = category {
        inv.category = cat.to_string();
    }
    if let Some(rem) = remark {
        inv.remark = rem.to_string();
    }
    Ok(inv)
}

pub fn insert_imported_invoice(
    conn: &Connection,
    inv: &models::Invoice,
    path: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let id = db::insert_invoice(conn, inv)?;
    if let Err(e) = attachment::add_attachment(conn, id, &inv.number, path) {
        eprintln!("Warning: Failed to save original file as attachment: {}", e);
    }
    Ok(id)
}

pub fn close_period(
    conn: &Connection,
    close_type: closing::CloseType,
    period: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match closing::close_period(conn, close_type, period) {
        Ok(()) => Ok(()),
        Err(e) => {
            if is_period_already_closed_err(e.as_ref()) {
                Err(Box::new(InvoiceError::AlreadyClosed(e.to_string())))
            } else if is_no_invoices_err(e.as_ref()) {
                Err(Box::new(InvoiceError::NoInvoices(e.to_string())))
            } else {
                Err(e)
            }
        }
    }
}

pub fn export_reports(
    conn: &Connection,
    close_type: closing::CloseType,
    period: &str,
    output_dir: &str,
) -> Result<(Vec<models::Invoice>, String, String), Box<dyn std::error::Error>> {
    let invoices = closing::query_invoices_for_period(conn, close_type, period)?;
    if invoices.is_empty() {
        return Err(Box::new(InvoiceError::NoInvoices(format!(
            "No invoices found for period {}",
            period
        ))));
    }
    std::fs::create_dir_all(output_dir)?;
    let detail_path = format!("{}/明细表_{}.xlsx", output_dir, period);
    let summary_path = format!("{}/汇总表_{}.xlsx", output_dir, period);
    report::generate_detail_report(&invoices, &detail_path)?;
    let summary_entries = report::compute_summary(&invoices);
    report::generate_summary_report(&summary_entries, &summary_path)?;
    Ok((invoices, detail_path, summary_path))
}
