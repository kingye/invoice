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

fn get_info_dict(pdf: &lopdf::Document) -> Option<lopdf::Dictionary> {
    match pdf.trailer.get(b"Info").ok()? {
        lopdf::Object::Reference(ref_id) => match pdf.get_object(*ref_id) {
            Ok(lopdf::Object::Dictionary(dict)) => Some(dict.clone()),
            _ => None,
        },
        lopdf::Object::Dictionary(dict) => Some(dict.clone()),
        _ => None,
    }
}

fn decode_pdf_string(obj: &lopdf::Object) -> Option<String> {
    match obj {
        lopdf::Object::String(bytes, _format) => {
            if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                let utf16: Vec<u16> = bytes[2..]
                    .chunks(2)
                    .filter_map(|c| {
                        if c.len() == 2 {
                            Some(u16::from_be_bytes([c[0], c[1]]))
                        } else {
                            None
                        }
                    })
                    .collect();
                String::from_utf16(&utf16).ok()
            } else {
                String::from_utf8(bytes.to_vec()).ok()
            }
        }
        _ => obj.as_string().ok().map(|s| s.to_string()),
    }
}

fn extract_metadata(pdf: &lopdf::Document, inv: &mut models::Invoice) {
    if let Some(info_dict) = get_info_dict(pdf) {
        if let Some(val) = info_dict
            .get(b"InvoiceNumber")
            .ok()
            .and_then(|o| decode_pdf_string(o))
        {
            inv.number = val;
        }
        if let Some(val) = info_dict
            .get(b"IssueTime")
            .ok()
            .and_then(|o| decode_pdf_string(o))
        {
            inv.date = val.replace("年", "-").replace("月", "-").replace("日", "");
        }
        if let Some(val) = info_dict
            .get(b"TotalAmWithoutTax")
            .ok()
            .and_then(|o| decode_pdf_string(o))
        {
            inv.amount = val.parse().unwrap_or(0.0);
        }
        if let Some(val) = info_dict
            .get(b"TotalTaxAm")
            .ok()
            .and_then(|o| decode_pdf_string(o))
        {
            inv.tax = val.parse().unwrap_or(0.0);
        }
        if let Some(val) = info_dict
            .get(b"TotalTax-includedAmount")
            .ok()
            .and_then(|o| decode_pdf_string(o))
        {
            inv.total = val.parse().unwrap_or(0.0);
        }
        if let Some(val) = info_dict
            .get(b"SellerIdNum")
            .ok()
            .and_then(|o| decode_pdf_string(o))
        {
            inv.seller_tax_id = val;
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

    if text.trim().is_empty() || is_cid_font_content(&text) {
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

fn is_cid_font_content(text: &str) -> bool {
    let re_cid = Regex::new(r"<[0-9A-Fa-f]{4,}>").unwrap();
    re_cid.find_iter(text).count() > 5
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
    use super::*;

    #[test]
    fn test_extract_from_pdf_empty_data() {
        let result = extract_from_pdf(&[]);
        assert!(result.is_ok());
        let inv = result.unwrap();
        assert!(inv.number.is_empty());
    }

    #[test]
    fn test_extract_from_pdf_invalid_data() {
        let result = extract_from_pdf(&[0x00, 0x01, 0x02, 0x03]);
        assert!(result.is_ok());
        let inv = result.unwrap();
        assert!(inv.number.is_empty());
    }

    #[test]
    fn test_decode_pdf_string_utf8() {
        let obj = lopdf::Object::string_literal("hello");
        let result = decode_pdf_string(&obj);
        assert!(result.is_some());
    }

    #[test]
    fn test_decode_pdf_string_utf16be() {
        let input = "你好";
        let utf16: Vec<u16> = input.encode_utf16().collect();
        let mut bytes = vec![0xFE, 0xFF];
        for code in utf16 {
            bytes.push((code >> 8) as u8);
            bytes.push((code & 0xFF) as u8);
        }
        let obj = lopdf::Object::string_literal(bytes);
        let result = decode_pdf_string(&obj);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "你好");
    }

    #[test]
    fn test_extract_text_regex_number() {
        let re = Regex::new(r"\d{20}").unwrap();
        let text = "发票号码 24112000000015301234 金额";
        assert!(re.find(text).is_some());
        assert_eq!(re.find(text).unwrap().as_str(), "24112000000015301234");
    }

    #[test]
    fn test_extract_text_regex_date() {
        let re = Regex::new(r"\d{4}年\d{2}月\d{2}日").unwrap();
        let text = "开票日期 2026年04月15日";
        let m = re.find(text).unwrap().as_str();
        let d = m.replace("年", "-").replace("月", "-").replace("日", "");
        assert_eq!(d, "2026-04-15");
    }

    #[test]
    fn test_extract_text_regex_seller() {
        let re = Regex::new(r"销\s*名称：\s*(.+)").unwrap();
        let text = "销名称：测试公司A";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str().trim(), "测试公司A");
    }

    #[test]
    fn test_extract_text_regex_item() {
        let re = Regex::new(r"\*[^*]+\*(.+)").unwrap();
        let text = "项目 *服务*技术咨询费";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str().trim(), "技术咨询费");
    }

    #[test]
    fn test_is_cid_font_content() {
        let cid_text = "BT <00140015> <00160017> <00120013> <00180019> <00200021> <00220023> ET";
        assert!(is_cid_font_content(cid_text));
        let normal_text = "BT (Hello World) Tj ET";
        assert!(!is_cid_font_content(normal_text));
    }
}
