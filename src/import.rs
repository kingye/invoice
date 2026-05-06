use std::path::Path;

use crate::extract_ofd;
use crate::extract_pdf;
use crate::extract_xml;
use crate::models;

pub fn extract_invoice(path: &str) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let file_path = Path::new(path);
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut inv = match ext.as_str() {
        "pdf" => extract_from_pdf(path)?,
        "xml" => extract_from_xml(path)?,
        "ofd" => extract_from_ofd(path)?,
        "zip" => extract_from_zip(path)?,
        _ => return Err(format!("Unsupported file format: .{}", ext).into()),
    };

    if ext == "pdf" {
        if let Ok(supplement) = try_supplement(path) {
            merge_invoice(&mut inv, &supplement);
        }
    }

    Ok(inv)
}

fn extract_from_pdf(path: &str) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    extract_pdf::extract_from_pdf(&data)
}

fn extract_from_xml(path: &str) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    extract_xml::extract_from_xml(&content)
}

fn extract_from_ofd(path: &str) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    extract_ofd::extract_from_ofd(&data)
}

fn extract_from_zip(path: &str) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    extract_xml::extract_from_xml_in_zip(&data)
}

fn try_supplement(pdf_path: &str) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let stem = Path::new(pdf_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let dir = Path::new(pdf_path).parent().unwrap_or(Path::new("."));

    let xml_path = dir.join(format!("{}.xml", stem));
    if xml_path.exists() {
        if let Ok(inv) = extract_from_xml(xml_path.to_str().unwrap_or("")) {
            return Ok(inv);
        }
    }

    let ofd_path = dir.join(format!("{}.ofd", stem));
    if ofd_path.exists() {
        if let Ok(inv) = extract_from_ofd(ofd_path.to_str().unwrap_or("")) {
            return Ok(inv);
        }
    }

    Err("No supplement file found".into())
}

fn merge_invoice(target: &mut models::Invoice, source: &models::Invoice) {
    if target.number.is_empty() && !source.number.is_empty() {
        target.number = source.number.clone();
    }
    if target.date.is_empty() && !source.date.is_empty() {
        target.date = source.date.clone();
    }
    if target.inv_type.is_empty() && !source.inv_type.is_empty() {
        target.inv_type = source.inv_type.clone();
    }
    if target.item_name.is_empty() && !source.item_name.is_empty() {
        target.item_name = source.item_name.clone();
    }
    if target.amount == 0.0 && source.amount != 0.0 {
        target.amount = source.amount;
    }
    if target.tax_rate == 0.0 && source.tax_rate != 0.0 {
        target.tax_rate = source.tax_rate;
    }
    if target.tax == 0.0 && source.tax != 0.0 {
        target.tax = source.tax;
    }
    if target.total == 0.0 && source.total != 0.0 {
        target.total = source.total;
    }
    if target.seller_name.is_empty() && !source.seller_name.is_empty() {
        target.seller_name = source.seller_name.clone();
    }
    if target.seller_tax_id.is_empty() && !source.seller_tax_id.is_empty() {
        target.seller_tax_id = source.seller_tax_id.clone();
    }
    if target.buyer_name.is_empty() && !source.buyer_name.is_empty() {
        target.buyer_name = source.buyer_name.clone();
    }
    if target.buyer_tax_id.is_empty() && !source.buyer_tax_id.is_empty() {
        target.buyer_tax_id = source.buyer_tax_id.clone();
    }
    if target.category.is_empty() && !source.category.is_empty() {
        target.category = source.category.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_invoice() {
        let mut target = models::Invoice {
            number: String::new(),
            date: "2026-01-01".to_string(),
            ..Default::default()
        };
        let source = models::Invoice {
            number: "12345".to_string(),
            amount: 100.0,
            ..Default::default()
        };
        merge_invoice(&mut target, &source);
        assert_eq!(target.number, "12345");
        assert_eq!(target.date, "2026-01-01");
        assert_eq!(target.amount, 100.0);
    }
}
