use crate::models;
use regex::Regex;

pub fn extract_from_pdf(data: &[u8]) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let mut inv = models::Invoice::default();

    if let Ok(pdf) = lopdf::Document::load_mem(data) {
        extract_metadata(&pdf, &mut inv);
        let _ = extract_text(&pdf, &mut inv);
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
    let pages = pdf.get_pages();
    let page_nums: Vec<u32> = pages.keys().copied().collect();

    let text = match lopdf::Document::extract_text(pdf, &page_nums) {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            let mut raw = String::new();
            for (_, obj_id) in pages {
                if let Ok(page_text) = extract_page_text(pdf, obj_id) {
                    raw.push_str(&page_text);
                    raw.push(' ');
                }
            }
            if raw.trim().is_empty() || is_cid_font_content(&raw) {
                return Ok(());
            }
            raw
        }
    };

    if text.trim().is_empty() {
        return Ok(());
    }

    let normalized = Regex::new(r"\s+")?.replace_all(&text, " ").to_string();

    if inv.number.is_empty() {
        let re = Regex::new(r"\d{20}")?;
        if let Some(m) = re.find(&normalized) {
            inv.number = m.as_str().to_string();
        }
    }

    if inv.date.is_empty() {
        let re = Regex::new(r"\d{4}年\d{2}月\d{2}日")?;
        if let Some(m) = re.find(&normalized) {
            inv.date = m
                .as_str()
                .replace("年", "-")
                .replace("月", "-")
                .replace("日", "");
        }
    }

    if inv.inv_type.is_empty() {
        let re = Regex::new(r"电子发票[（(]([^）)]+)[)）]")?;
        if let Some(caps) = re.captures(&normalized) {
            inv.inv_type = format!("电子发票（{}）", caps.get(1).unwrap().as_str());
        } else {
            let re2 = Regex::new(r"电\s*子\s*发\s*票\s*[（(]\s*普\s*通\s*发\s*票\s*[)）]")?;
            if let Some(m) = re2.find(&normalized) {
                inv.inv_type = m.as_str().replace(" ", "");
            }
        }
    }

    if inv.seller_name.is_empty() {
        let re = Regex::new(r"销\s*售\s*方.*?名称[：:]\s*(\S+)")?;
        if let Some(caps) = re.captures(&normalized) {
            inv.seller_name = caps.get(1).unwrap().as_str().to_string();
        } else {
            let re2 = Regex::new(r"销\s*名称[：:]\s*(\S+)")?;
            if let Some(caps) = re2.captures(&normalized) {
                inv.seller_name = caps.get(1).unwrap().as_str().to_string();
            }
        }
    }

    if inv.buyer_name.is_empty() {
        let re = Regex::new(r"购\s*买\s*方.*?名称[：:]\s*(\S+)")?;
        if let Some(caps) = re.captures(&normalized) {
            inv.buyer_name = caps.get(1).unwrap().as_str().to_string();
        } else {
            let re2 = Regex::new(r"购\s*名称[：:]\s*(\S+)")?;
            if let Some(caps) = re2.captures(&normalized) {
                inv.buyer_name = caps.get(1).unwrap().as_str().to_string();
            }
        }
    }

    if inv.item_name.is_empty() {
        let re = Regex::new(r"\*[^*]+\*([^*\s]+)")?;
        if let Some(caps) = re.captures(&normalized) {
            inv.item_name = caps.get(1).unwrap().as_str().to_string();
        }
    }

    if inv.tax_rate == 0.0 {
        let re = Regex::new(r"(\d+)%")?;
        for cap in re.captures_iter(&normalized) {
            if let Some(m) = cap.get(1) {
                let pct: f64 = m.as_str().parse().unwrap_or(0.0);
                if pct > 0.0 && pct <= 100.0 {
                    inv.tax_rate = pct / 100.0;
                    break;
                }
            }
        }
    }

    if inv.amount == 0.0 {
        let re = Regex::new(r"¥\s*(\d+\.?\d*)")?;
        let amounts: Vec<f64> = re
            .captures_iter(&normalized)
            .filter_map(|c| c.get(1)?.as_str().parse().ok())
            .collect();
        if amounts.len() >= 2 {
            inv.amount = amounts[0];
            inv.total = *amounts.last().unwrap();
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
        let re = Regex::new(r"销\s*售\s*方.*?名称[：:]\s*(\S+)").unwrap();
        let text = "销售方 信息 名称：上海壹佰米网络科技有限公司";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "上海壹佰米网络科技有限公司");
    }

    #[test]
    fn test_extract_text_regex_buyer() {
        let re = Regex::new(r"购\s*买\s*方.*?名称[：:]\s*(\S+)").unwrap();
        let text = "购买方 信息 名称：张三";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "张三");
    }

    #[test]
    fn test_extract_text_regex_item() {
        let re = Regex::new(r"\*[^*]+\*([^*\s]+)").unwrap();
        let text = "项目 *服务*技术咨询费";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "技术咨询费");
    }

    #[test]
    fn test_extract_text_regex_type() {
        let re = Regex::new(r"电子发票[（(]([^）)]+)[)）]").unwrap();
        let text = "电子发票（普通发票） 金额";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "普通发票");
    }

    #[test]
    fn test_is_cid_font_content() {
        let cid_text = "BT <00140015> <00160017> <00120013> <00180019> <00200021> <00220023> ET";
        assert!(is_cid_font_content(cid_text));
        let normal_text = "BT (Hello World) Tj ET";
        assert!(!is_cid_font_content(normal_text));
    }
}
