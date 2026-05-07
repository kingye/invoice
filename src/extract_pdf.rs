use crate::models;
use regex::Regex;

pub fn extract_from_pdf(data: &[u8]) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let mut inv = models::Invoice::default();

    if let Ok(pdf) = lopdf::Document::load_mem(data) {
        let has_text = extract_text(&pdf, &mut inv).unwrap_or(false);
        if !has_text {
            extract_metadata(&pdf, &mut inv);
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
        if inv.number.is_empty() {
            if let Some(val) = info_dict
                .get(b"InvoiceNumber")
                .ok()
                .and_then(|o| decode_pdf_string(o))
            {
                inv.number = val;
            }
        }
        if inv.date.is_empty() {
            if let Some(val) = info_dict
                .get(b"IssueTime")
                .ok()
                .and_then(|o| decode_pdf_string(o))
            {
                inv.date = val.replace("年", "-").replace("月", "-").replace("日", "");
            }
        }
        if inv.amount == 0.0 {
            if let Some(val) = info_dict
                .get(b"TotalAmWithoutTax")
                .ok()
                .and_then(|o| decode_pdf_string(o))
            {
                inv.amount = val.parse().unwrap_or(0.0);
            }
        }
        if inv.tax == 0.0 {
            if let Some(val) = info_dict
                .get(b"TotalTaxAm")
                .ok()
                .and_then(|o| decode_pdf_string(o))
            {
                inv.tax = val.parse().unwrap_or(0.0);
            }
        }
        if inv.total == 0.0 {
            if let Some(val) = info_dict
                .get(b"TotalTax-includedAmount")
                .ok()
                .and_then(|o| decode_pdf_string(o))
            {
                inv.total = val.parse().unwrap_or(0.0);
            }
        }
        if inv.seller_tax_id.is_empty() {
            if let Some(val) = info_dict
                .get(b"SellerIdNum")
                .ok()
                .and_then(|o| decode_pdf_string(o))
            {
                inv.seller_tax_id = val;
            }
        }
    }
}

fn extract_text(
    pdf: &lopdf::Document,
    inv: &mut models::Invoice,
) -> Result<bool, Box<dyn std::error::Error>> {
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
                return Ok(false);
            }
            raw
        }
    };

    if text.trim().is_empty() {
        return Ok(false);
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

    if inv.seller_name.is_empty() || inv.buyer_name.is_empty() {
        let re_names = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+(\S+)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)\s+(9\d{14,17})",
        )?;
        if let Some(caps) = re_names.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                inv.buyer_name = caps.get(2).unwrap().as_str().to_string();
            }
            if inv.seller_name.is_empty() {
                inv.seller_name = caps.get(3).unwrap().as_str().to_string();
            }
            if inv.seller_tax_id.is_empty() {
                inv.seller_tax_id = caps.get(4).unwrap().as_str().to_string();
            }
        }
    }

    if inv.seller_name.is_empty() || inv.buyer_name.is_empty() {
        let re_names2 = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+(\S+)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)",
        )?;
        if let Some(caps) = re_names2.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                inv.buyer_name = caps.get(2).unwrap().as_str().to_string();
            }
            if inv.seller_name.is_empty() {
                inv.seller_name = caps.get(3).unwrap().as_str().to_string();
            }
        }
    }

    if inv.seller_name.is_empty() {
        let re = Regex::new(r"销\s*售\s*方.*?名称[：:]\s*(\S+)")?;
        if let Some(caps) = re.captures(&normalized) {
            let val = caps.get(1).unwrap().as_str();
            if val != "名称" && val != "名称：" {
                inv.seller_name = val.to_string();
            }
        }
    }

    if inv.buyer_name.is_empty() {
        let re = Regex::new(r"购\s*买\s*方.*?名称[：:]\s*(\S+)")?;
        if let Some(caps) = re.captures(&normalized) {
            let val = caps.get(1).unwrap().as_str();
            if val != "名称" && val != "名称：" {
                inv.buyer_name = val.to_string();
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
        let re_before = Regex::new(r"(\d+\.?\d*)\s*¥")?;
        let amounts: Vec<f64> = re_before
            .captures_iter(&normalized)
            .filter_map(|c| c.get(1)?.as_str().parse().ok())
            .collect();
        if !amounts.is_empty() {
            inv.amount = amounts[0];
            if amounts.len() >= 2 {
                inv.tax = amounts[1];
            }
        }

        if inv.total == 0.0 {
            let re_after = Regex::new(r"¥\s*(\d+\.?\d*)")?;
            let totals: Vec<f64> = re_after
                .captures_iter(&normalized)
                .filter_map(|c| c.get(1)?.as_str().parse().ok())
                .collect();
            if let Some(&val) = totals.last() {
                inv.total = val;
            }
        }
    }

    Ok(!normalized.trim().is_empty())
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
        let re = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+(\S+)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)\s+(9\d{14,17})",
        ).unwrap();
        let text = "2026年04月07日 ChengQing 上海星巴克咖啡经营有限公司 913100006074138050";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str(), "ChengQing");
        assert_eq!(caps.get(3).unwrap().as_str(), "上海星巴克咖啡经营有限公司");
        assert_eq!(caps.get(4).unwrap().as_str(), "913100006074138050");
    }

    #[test]
    fn test_extract_text_regex_buyer() {
        let re = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+(\S+)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)",
        )
        .unwrap();
        let text = "2026年04月07日 程青 上海兰心荟餐饮管理有限公司";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str(), "程青");
        assert_eq!(caps.get(3).unwrap().as_str(), "上海兰心荟餐饮管理有限公司");
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

    fn create_pdf_with_metadata() -> Vec<u8> {
        let mut doc = lopdf::Document::new();
        let info_dict = lopdf::Dictionary::from_iter([
            (
                b"InvoiceNumber".to_vec(),
                lopdf::Object::string_literal("META_NUMBER"),
            ),
            (
                b"IssueTime".to_vec(),
                lopdf::Object::string_literal("2025年12月01日"),
            ),
            (
                b"TotalAmWithoutTax".to_vec(),
                lopdf::Object::string_literal("200.00"),
            ),
            (
                b"TotalTaxAm".to_vec(),
                lopdf::Object::string_literal("12.00"),
            ),
            (
                b"TotalTax-includedAmount".to_vec(),
                lopdf::Object::string_literal("212.00"),
            ),
            (
                b"SellerIdNum".to_vec(),
                lopdf::Object::string_literal("999999999999999"),
            ),
        ]);
        let info_id = doc.add_object(lopdf::Object::Dictionary(info_dict));
        let catalog = lopdf::Dictionary::from_iter([(
            b"Type".to_vec(),
            lopdf::Object::Name(b"Catalog".to_vec()),
        )]);
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer.set(b"Info", lopdf::Object::Reference(info_id));
        doc.trailer
            .set(b"Root", lopdf::Object::Reference(catalog_id));
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_metadata_does_not_overwrite_prefilled_fields() {
        let data = create_pdf_with_metadata();
        let pdf = lopdf::Document::load_mem(&data).unwrap();
        let mut inv = models::Invoice {
            number: "TEXT_NUMBER".to_string(),
            date: "2026-01-01".to_string(),
            amount: 100.0,
            tax: 6.0,
            total: 106.0,
            seller_tax_id: "91110000MA01".to_string(),
            ..Default::default()
        };
        extract_metadata(&pdf, &mut inv);
        assert_eq!(inv.number, "TEXT_NUMBER");
        assert_eq!(inv.date, "2026-01-01");
        assert_eq!(inv.amount, 100.0);
        assert_eq!(inv.tax, 6.0);
        assert_eq!(inv.total, 106.0);
        assert_eq!(inv.seller_tax_id, "91110000MA01");
    }

    #[test]
    fn test_metadata_fills_empty_fields() {
        let data = create_pdf_with_metadata();
        let pdf = lopdf::Document::load_mem(&data).unwrap();
        let mut inv = models::Invoice::default();
        extract_metadata(&pdf, &mut inv);
        assert_eq!(inv.number, "META_NUMBER");
        assert_eq!(inv.date, "2025-12-01");
        assert_eq!(inv.amount, 200.0);
        assert_eq!(inv.tax, 12.0);
        assert_eq!(inv.total, 212.0);
        assert_eq!(inv.seller_tax_id, "999999999999999");
    }

    #[test]
    fn test_text_first_extracts_from_pdf_with_metadata() {
        let data = create_pdf_with_metadata();
        let result = extract_from_pdf(&data);
        assert!(result.is_ok());
        let inv = result.unwrap();
        assert_eq!(inv.number, "META_NUMBER");
        assert_eq!(inv.date, "2025-12-01");
    }
}
