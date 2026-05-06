use crate::models;
use regex::Regex;

pub fn extract_from_pdf(data: &[u8]) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let mut inv = models::Invoice::default();

    if let Ok(pdf) = lopdf::Document::load_mem(data) {
        extract_metadata(&pdf, &mut inv);
        if inv.number.is_empty() {
            let _ = extract_text(&pdf, &mut inv);
        }
    }

    Ok(inv)
}

fn extract_metadata(pdf: &lopdf::Document, inv: &mut models::Invoice) {
    if let Some(info_dict) = pdf.trailer.get(b"Info").ok().and_then(|o| o.as_dict().ok()) {
        if let Some(val) = info_dict
            .get(b"InvoiceNumber")
            .ok()
            .and_then(|o| o.as_string().ok())
        {
            inv.number = val.to_string();
        }
        if let Some(val) = info_dict
            .get(b"IssueTime")
            .ok()
            .and_then(|o| o.as_string().ok())
        {
            inv.date = val.to_string();
        }
        if let Some(val) = info_dict
            .get(b"TotalAmWithoutTax")
            .ok()
            .and_then(|o| o.as_string().ok())
        {
            inv.amount = val.parse().unwrap_or(0.0);
        }
        if let Some(val) = info_dict
            .get(b"TotalTaxAm")
            .ok()
            .and_then(|o| o.as_string().ok())
        {
            inv.tax = val.parse().unwrap_or(0.0);
        }
        if let Some(val) = info_dict
            .get(b"TotalTax-includedAmount")
            .ok()
            .and_then(|o| o.as_string().ok())
        {
            inv.total = val.parse().unwrap_or(0.0);
        }
        if let Some(val) = info_dict
            .get(b"SellerIdNum")
            .ok()
            .and_then(|o| o.as_string().ok())
        {
            inv.seller_tax_id = val.to_string();
        }
    }
}

fn extract_text(
    pdf: &lopdf::Document,
    inv: &mut models::Invoice,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut text = String::new();
    let pages = pdf.get_pages();
    for (_, obj_id) in pages {
        if let Ok(page_text) = extract_page_text(pdf, obj_id) {
            text.push_str(&page_text);
            text.push(' ');
        }
    }

    if text.trim().is_empty() {
        return Ok(());
    }

    let re_number = Regex::new(r"\d{20}")?;
    if let Some(m) = re_number.find(&text) {
        inv.number = m.as_str().to_string();
    }

    let re_date = Regex::new(r"\d{4}年\d{2}月\d{2}日")?;
    if let Some(m) = re_date.find(&text) {
        let d = m
            .as_str()
            .replace("年", "-")
            .replace("月", "-")
            .replace("日", "");
        inv.date = d;
    }

    let re_seller = Regex::new(r"销\s*名称：\s*(.+)")?;
    if let Some(caps) = re_seller.captures(&text) {
        if let Some(m) = caps.get(1) {
            inv.seller_name = m.as_str().trim().to_string();
        }
    }

    let re_buyer = Regex::new(r"购\s*名称：\s*(.+)")?;
    if let Some(caps) = re_buyer.captures(&text) {
        if let Some(m) = caps.get(1) {
            inv.buyer_name = m.as_str().trim().to_string();
        }
    }

    let re_item = Regex::new(r"\*[^*]+\*(.+)")?;
    if let Some(caps) = re_item.captures(&text) {
        if let Some(m) = caps.get(1) {
            inv.item_name = m.as_str().trim().to_string();
        }
    }

    Ok(())
}

fn extract_page_text(
    pdf: &lopdf::Document,
    obj_id: lopdf::ObjectId,
) -> Result<String, lopdf::Error> {
    let page = pdf.get_object(obj_id)?;
    let dict = page.as_dict()?;
    let contents = dict.get(b"Contents")?;
    match contents {
        lopdf::Object::Reference(ref_id) => {
            let stream = pdf.get_object(*ref_id)?.as_stream()?;
            let decoded = stream.decompressed_content()?;
            Ok(String::from_utf8_lossy(&decoded).to_string())
        }
        lopdf::Object::Array(arr) => {
            let mut result = String::new();
            for item in arr {
                if let lopdf::Object::Reference(ref_id) = item {
                    if let Ok(stream) = pdf.get_object(*ref_id).and_then(|o| o.as_stream()) {
                        if let Ok(decoded) = stream.decompressed_content() {
                            result.push_str(&String::from_utf8_lossy(&decoded));
                        }
                    }
                }
            }
            Ok(result)
        }
        _ => Ok(String::new()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_from_pdf_placeholder() {
        assert!(true);
    }
}
