use crate::models;
use regex::Regex;

pub fn extract_from_pdf(data: &[u8]) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    extract_from_pdf_with_ocr(data, None)
}

pub fn extract_from_pdf_with_ocr(
    data: &[u8],
    ocr_model_dir: Option<&str>,
) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let mut inv = models::Invoice::default();

    if let Ok(doc) = pdf_oxide::PdfDocument::from_bytes(data.to_vec()) {
        let text = doc.extract_all_text().unwrap_or_default();
        if !text.trim().is_empty() {
            parse_invoice_text(&text, &mut inv);
        } else if let Some(model_dir) = ocr_model_dir {
            if let Ok(engine) = crate::ocr::get_ocr_engine_with_dir(model_dir) {
                try_ocr_extraction(&doc, &engine, &mut inv);
            }
        } else if crate::ocr::model_files_exist() {
            if let Ok(engine) = crate::ocr::get_ocr_engine() {
                try_ocr_extraction(&doc, &engine, &mut inv);
            }
        }
    }

    Ok(inv)
}

fn try_ocr_extraction(
    doc: &pdf_oxide::PdfDocument,
    engine: &pdf_oxide::ocr::OcrEngine,
    inv: &mut models::Invoice,
) {
    let page_count = doc.page_count().unwrap_or(0);
    let mut all_text = String::new();

    for page in 0..page_count {
        if let Ok(true) = pdf_oxide::ocr::needs_ocr(doc, page) {
            let options = pdf_oxide::ocr::OcrExtractOptions::default();
            if let Ok(text) =
                pdf_oxide::ocr::extract_text_with_ocr(doc, page, Some(engine), options)
            {
                if !text.trim().is_empty() {
                    if !all_text.is_empty() {
                        all_text.push(' ');
                    }
                    all_text.push_str(&text);
                }
            }
        }
    }

    if !all_text.trim().is_empty() {
        parse_invoice_text(&all_text, inv);
    }
}

fn parse_invoice_text(text: &str, inv: &mut models::Invoice) {
    let normalized = match Regex::new(r"\s+") {
        Ok(re) => re.replace_all(text, " ").to_string(),
        Err(_) => return,
    };

    if inv.number.is_empty() {
        // Match invoice number after "发票号码" label (prefer this over bare 20-digit match)
        let re_labeled = Regex::new(r"发票号码[：:]\s*(\d{8,20})").unwrap();
        if let Some(caps) = re_labeled.captures(&normalized) {
            inv.number = caps.get(1).unwrap().as_str().to_string();
        } else {
            let re = Regex::new(r"\d{20}").unwrap();
            if let Some(m) = re.find(&normalized) {
                inv.number = m.as_str().to_string();
            }
        }
    }

    if inv.date.is_empty() {
        let re = Regex::new(r"\d{4}年\d{2}月\d{2}日").unwrap();
        if let Some(m) = re.find(&normalized) {
            inv.date = m
                .as_str()
                .replace("年", "-")
                .replace("月", "-")
                .replace("日", "");
        } else {
            // Handle spaced format: "2026 04 03 年 月 日"
            let re_spaced = Regex::new(r"(\d{4})\s+(\d{2})\s+(\d{2})\s+年\s*月\s*日").unwrap();
            if let Some(caps) = re_spaced.captures(&normalized) {
                inv.date = format!(
                    "{}-{}-{}",
                    caps.get(1).unwrap().as_str(),
                    caps.get(2).unwrap().as_str(),
                    caps.get(3).unwrap().as_str()
                );
            }
        }
    }

    if inv.inv_type.is_empty() {
        let re = Regex::new(r"电子发票[（(]([^）)]+)[)）]").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            inv.inv_type = format!("电子发票（{}）", caps.get(1).unwrap().as_str());
        } else {
            let re2 = Regex::new(r"电\s*子\s*发\s*票\s*[（(]\s*普\s*通\s*发\s*票\s*[)）]").unwrap();
            if let Some(m) = re2.find(&normalized) {
                inv.inv_type = m.as_str().replace(" ", "");
            } else {
                // Traditional format: "增值税电子普通发票"
                let re3 = Regex::new(r"增值税电子普通发票").unwrap();
                if re3.is_match(&normalized) {
                    inv.inv_type = "增值税电子普通发票".to_string();
                }
            }
        }
    }

    let mut table_layout_matched = false;
    let mut ofd_layout_matched = false;

    // Layout variant: "购 销 <buyer> 名称： <seller> 名称："
    // In this layout, buyer name appears BEFORE the first "名称：" and seller between the two "名称："
    let re_ofd_detect = Regex::new(r"购\s+销\s+名称[：:]").unwrap();
    let re_ofd_detect_v2 = Regex::new(r"购\s+销\s+\S+.*?名称[：:]").unwrap();
    if re_ofd_detect.is_match(&normalized) {
        let re_ofd_names =
            Regex::new(r"名称[：:]\s*(\S+(?:\s+\S+)*?)\s+名称[：:]\s*([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)")
                .unwrap();
        if let Some(caps) = re_ofd_names.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                let val = caps.get(1).unwrap().as_str().trim();
                if !val.is_empty() {
                    inv.buyer_name = val.to_string();
                }
            }
            if inv.seller_name.is_empty() {
                let val = caps.get(2).unwrap().as_str().trim();
                if !val.is_empty() {
                    inv.seller_name = val.to_string();
                }
            }
            ofd_layout_matched = true;
        }

        if ofd_layout_matched {
            let re_ofd_tax = Regex::new(
                r"信\s*(9[A-Z0-9]{15,19})?\s*统一社会信用代码\s*/\s*纳税人识别号\s*[：:]",
            )
            .unwrap();
            let tax_ids: Vec<String> = re_ofd_tax
                .captures_iter(&normalized)
                .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                .collect();
            if tax_ids.len() >= 2 {
                if inv.buyer_tax_id.is_empty() {
                    inv.buyer_tax_id = tax_ids[0].clone();
                }
                if inv.seller_tax_id.is_empty() {
                    inv.seller_tax_id = tax_ids[1].clone();
                }
            } else if tax_ids.len() == 1 {
                if inv.seller_tax_id.is_empty() {
                    inv.seller_tax_id = tax_ids[0].clone();
                }
            }
        }
    } else if re_ofd_detect_v2.is_match(&normalized) {
        // Variant: "购 销 ChengQing 名称： 上海XXX公司 名称："
        // Buyer is between "购 销" and first "名称：", seller is after first "名称："
        let re_v2_names = Regex::new(
            r"购\s+销\s+(.+?)\s+名称[：:]\s*([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]*(?:\s*[\x{4e00}-\x{9fff}\w()（）]+)*)\s+名称[：:]"
        ).unwrap();
        if let Some(caps) = re_v2_names.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                let val = caps.get(1).unwrap().as_str().trim();
                if !val.is_empty() {
                    inv.buyer_name = val.to_string();
                }
            }
            if inv.seller_name.is_empty() {
                let val = caps.get(2).unwrap().as_str().trim();
                if !val.is_empty() {
                    inv.seller_name = val.to_string();
                }
            }
            ofd_layout_matched = true;
        }

        if ofd_layout_matched {
            let re_ofd_tax = Regex::new(
                r"信\s*(9[A-Z0-9]{15,19})?\s*统一社会信用代码\s*/\s*纳税人识别号\s*[：:]",
            )
            .unwrap();
            let tax_ids: Vec<String> = re_ofd_tax
                .captures_iter(&normalized)
                .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                .collect();
            if tax_ids.len() >= 2 {
                if inv.buyer_tax_id.is_empty() {
                    inv.buyer_tax_id = tax_ids[0].clone();
                }
                if inv.seller_tax_id.is_empty() {
                    inv.seller_tax_id = tax_ids[1].clone();
                }
            } else if tax_ids.len() == 1 {
                if inv.seller_tax_id.is_empty() {
                    inv.seller_tax_id = tax_ids[0].clone();
                }
            }
        }
    }

    if !ofd_layout_matched && (inv.buyer_name.is_empty() || inv.seller_name.is_empty()) {
        let re_table =
            Regex::new(r"购\s+名称[：:]\s*(.+?)\s+销\s+名称[：:]\s*(.+?)(?:\s+买|$)").unwrap();
        if let Some(caps) = re_table.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                let val = caps.get(1).unwrap().as_str().trim();
                if !val.is_empty() {
                    inv.buyer_name = val.to_string();
                }
            }
            if inv.seller_name.is_empty() {
                let val = caps.get(2).unwrap().as_str().trim();
                if !val.is_empty() {
                    inv.seller_name = val.to_string();
                }
            }
            table_layout_matched = true;
        }
    }

    if table_layout_matched || ofd_layout_matched {
        let re_tax_table = Regex::new(
            r"统一社会信用代码\s*/\s*纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})?.*?统一社会信用代码\s*/\s*纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})?",
        )
        .unwrap();
        if let Some(caps) = re_tax_table.captures(&normalized) {
            if let Some(m) = caps.get(1) {
                let val = m.as_str().trim();
                if !val.is_empty() && inv.buyer_tax_id.is_empty() {
                    inv.buyer_tax_id = val.to_string();
                }
            }
            if let Some(m) = caps.get(2) {
                let val = m.as_str().trim();
                if !val.is_empty() && inv.seller_tax_id.is_empty() {
                    inv.seller_tax_id = val.to_string();
                }
            }
        }

        if inv.seller_tax_id.is_empty() || inv.buyer_tax_id.is_empty() {
            let re_tax_table_rev = Regex::new(
                r"统一社会信用代码\s*/\s*纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})?\s*信\s*(9[A-Z0-9]{15,19})?\s*统一社会信用代码\s*/\s*纳税人识别号[：:]",
            )
            .unwrap();
            if let Some(caps) = re_tax_table_rev.captures(&normalized) {
                if let Some(m) = caps.get(1) {
                    let val = m.as_str().trim();
                    if !val.is_empty() && inv.buyer_tax_id.is_empty() {
                        inv.buyer_tax_id = val.to_string();
                    }
                }
                if let Some(m) = caps.get(2) {
                    let val = m.as_str().trim();
                    if !val.is_empty() && inv.seller_tax_id.is_empty() {
                        inv.seller_tax_id = val.to_string();
                    }
                }
            }
        }
    }

    let skip_label_regex = table_layout_matched || ofd_layout_matched;

    let label_keywords = [
        "名称", "项目", "规格", "型号", "单位", "数量", "单价", "金额", "税率", "税额",
    ];
    fn is_valid_label_value(val: &str, keywords: &[&str]) -> bool {
        if val.len() > 50 {
            return false;
        }
        for kw in keywords {
            if val.contains(kw) {
                return false;
            }
        }
        true
    }

    if !skip_label_regex && inv.buyer_name.is_empty() {
        let re =
            Regex::new(r"购\s*买\s*方.*?名\s*称[：:]\s*([^\s购销]+(?:\s+[^\s购销]+)*)").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            let val = caps.get(1).unwrap().as_str().trim();
            if !val.is_empty() && is_valid_label_value(val, &label_keywords) {
                inv.buyer_name = val.to_string();
            }
        }
    }

    if !skip_label_regex && inv.seller_name.is_empty() {
        let re = Regex::new(
            r"销\s*售\s*方.*?名\s*称[：:]\s*([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)",
        )
        .unwrap();
        if let Some(caps) = re.captures(&normalized) {
            let val = caps.get(1).unwrap().as_str().trim();
            if !val.is_empty() && is_valid_label_value(val, &label_keywords) {
                inv.seller_name = val.to_string();
            }
        }
    }

    if !skip_label_regex && inv.seller_tax_id.is_empty() {
        let re = Regex::new(r"销\s*售\s*方.*?纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            inv.seller_tax_id = caps.get(1).unwrap().as_str().to_string();
        }
    }

    if !skip_label_regex && inv.buyer_tax_id.is_empty() {
        let re = Regex::new(r"购\s*买\s*方.*?纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            inv.buyer_tax_id = caps.get(1).unwrap().as_str().to_string();
        }
    }

    if !skip_label_regex && (inv.seller_name.is_empty() || inv.buyer_name.is_empty()) {
        let re_names = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+([^\d]+?)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（） ]+?)\s+(9[A-Z0-9]{15,19})",
        ).unwrap();
        if let Some(caps) = re_names.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                inv.buyer_name = caps.get(2).unwrap().as_str().trim().to_string();
            }
            if inv.seller_name.is_empty() {
                inv.seller_name = caps.get(3).unwrap().as_str().trim().to_string();
            }
            if inv.seller_tax_id.is_empty() {
                inv.seller_tax_id = caps.get(4).unwrap().as_str().to_string();
            }
        }
    }

    if !skip_label_regex && (inv.seller_name.is_empty() || inv.buyer_name.is_empty()) {
        let re_names2 = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+(\S+)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)",
        )
        .unwrap();
        if let Some(caps) = re_names2.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                let val = caps.get(2).unwrap().as_str().to_string();
                if val != "购" && val != "名称" {
                    inv.buyer_name = val;
                }
            }
            if inv.seller_name.is_empty() {
                let val = caps.get(3).unwrap().as_str().to_string();
                if val != "名称" && val != "销" {
                    inv.seller_name = val;
                }
            }
        }
    }

    if inv.seller_tax_id.is_empty() {
        let re =
            Regex::new(r"统一社会信用代码\s*/\s*纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            inv.seller_tax_id = caps.get(1).unwrap().as_str().to_string();
        }
    }

    if inv.seller_tax_id.is_empty() {
        let re = Regex::new(r"纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            inv.seller_tax_id = caps.get(1).unwrap().as_str().to_string();
        }
    }

    if inv.seller_tax_id.is_empty() {
        let re =
            Regex::new(r"(9[A-Z0-9]{15,19})\s+统一社会信用代码\s*/\s*纳税人识别号[：:]").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            inv.seller_tax_id = caps.get(1).unwrap().as_str().to_string();
        }
    }

    // Traditional invoice format: "名 称:XXX" with spaced label in 购/销 sections
    // Pattern: "名 称:<buyer> ... 购" then "名 称:<seller> 销 ... 纳税人识别号:<id>"
    if inv.seller_name.is_empty() || inv.buyer_name.is_empty() {
        // Buyer: "名 称:<name>" followed later by "购"
        let re_trad_buyer = Regex::new(r"名\s*称[：:]\s*(\S+).*?购").unwrap();
        if let Some(caps) = re_trad_buyer.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                let val = caps.get(1).unwrap().as_str().trim();
                if !val.is_empty() && !val.contains("密") {
                    inv.buyer_name = val.to_string();
                }
            }
        }
        // Seller: "名 称:<Chinese name>" followed by "销"
        let re_trad_seller =
            Regex::new(r"名\s*称[：:]\s*([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)\s+销")
                .unwrap();
        if let Some(caps) = re_trad_seller.captures(&normalized) {
            if inv.seller_name.is_empty() {
                inv.seller_name = caps.get(1).unwrap().as_str().to_string();
            }
        }
    }

    // Traditional format seller tax ID: "纳税人识别号:<id>" near "销/售"
    if inv.seller_tax_id.is_empty() {
        let re_trad_seller_tax = Regex::new(r"纳税人识别号[：:]\s*(\w{15,20})").unwrap();
        // Find all tax IDs and pick the one in seller context (after seller name)
        let tax_ids: Vec<String> = re_trad_seller_tax
            .captures_iter(&normalized)
            .filter_map(|c| {
                let id = c.get(1)?.as_str();
                // Filter out garbled cipher text (contains special chars)
                if id.chars().all(|c| c.is_alphanumeric()) {
                    Some(id.to_string())
                } else {
                    None
                }
            })
            .collect();
        if let Some(id) = tax_ids.last() {
            inv.seller_tax_id = id.clone();
        }
    }

    if inv.buyer_tax_id.is_empty() {
        let re = Regex::new(r"9[A-Z0-9]{15,19}").unwrap();
        let all_tax_ids: Vec<&str> = re.find_iter(&normalized).map(|m| m.as_str()).collect();
        if all_tax_ids.len() >= 2 && !inv.seller_tax_id.is_empty() {
            for tax_id in &all_tax_ids {
                if *tax_id != inv.seller_tax_id && inv.buyer_tax_id.is_empty() {
                    inv.buyer_tax_id = tax_id.to_string();
                }
            }
        }
    }

    if inv.item_name.is_empty() {
        // Item pattern: *<category>*<item_name> where category must contain Chinese chars
        let re = Regex::new(r"\*[\x{4e00}-\x{9fff}][^*]*\*([^*\s]+)").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            inv.item_name = caps.get(1).unwrap().as_str().to_string();
        }
    }

    if inv.tax_rate == 0.0 {
        let re = Regex::new(r"(\d+)%").unwrap();
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
        let re_heji = Regex::new(r"合\s*计\s*¥?\s*(\d+\.?\d*)\s*¥\s*(\d+\.?\d*)").unwrap();
        if let Some(caps) = re_heji.captures(&normalized) {
            inv.amount = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
            inv.tax = caps.get(2).unwrap().as_str().parse().unwrap_or(0.0);
        } else {
            let re_before = Regex::new(r"(\d+\.?\d*)\s*¥").unwrap();
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
        }
    }

    if inv.total == 0.0 {
        let re_jiashui = Regex::new(r"价税合计.*?¥\s*(\d+\.?\d*)").unwrap();
        if let Some(caps) = re_jiashui.captures(&normalized) {
            inv.total = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
        } else {
            let re_after = Regex::new(r"¥\s*(\d+\.?\d*)").unwrap();
            let totals: Vec<f64> = re_after
                .captures_iter(&normalized)
                .filter_map(|c| c.get(1)?.as_str().parse().ok())
                .collect();
            if let Some(&val) = totals.last() {
                inv.total = val;
            }
        }
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
    fn test_parse_invoice_text_number() {
        let re = Regex::new(r"\d{20}").unwrap();
        let text = "发票号码 24112000000015301234 金额";
        assert!(re.find(text).is_some());
        assert_eq!(re.find(text).unwrap().as_str(), "24112000000015301234");
    }

    #[test]
    fn test_parse_invoice_text_date() {
        let re = Regex::new(r"\d{4}年\d{2}月\d{2}日").unwrap();
        let text = "开票日期 2026年04月15日";
        let m = re.find(text).unwrap().as_str();
        let d = m.replace("年", "-").replace("月", "-").replace("日", "");
        assert_eq!(d, "2026-04-15");
    }

    #[test]
    fn test_parse_invoice_text_seller_with_tax_id() {
        let re = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+([^\d]+?)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（） ]+?)\s+(9[A-Z0-9]{15,19})",
        ).unwrap();
        let text = "2026年04月07日 ChengQing 上海星巴克咖啡经营有限公司 913100006074138050";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str().trim(), "ChengQing");
        assert_eq!(
            caps.get(3).unwrap().as_str().trim(),
            "上海星巴克咖啡经营有限公司"
        );
        assert_eq!(caps.get(4).unwrap().as_str(), "913100006074138050");
    }

    #[test]
    fn test_parse_invoice_text_buyer_with_spaces() {
        let re = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+([^\d]+?)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（） ]+?)\s+(9[A-Z0-9]{15,19})",
        ).unwrap();
        let text =
            "2026年04月07日 Cheng Qing（个人） 上海望七阁餐饮管理有限公司 913100006074138050";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(2).unwrap().as_str().trim(), "Cheng Qing（个人）");
        assert_eq!(
            caps.get(3).unwrap().as_str().trim(),
            "上海望七阁餐饮管理有限公司"
        );
    }

    #[test]
    fn test_parse_invoice_text_buyer_fallback() {
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
    fn test_parse_invoice_text_item() {
        let re = Regex::new(r"\*[^*]+\*([^*\s]+)").unwrap();
        let text = "项目 *服务*技术咨询费";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "技术咨询费");
    }

    #[test]
    fn test_parse_invoice_text_type() {
        let re = Regex::new(r"电子发票[（(]([^）)]+)[)）]").unwrap();
        let text = "电子发票（普通发票） 金额";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "普通发票");
    }

    #[test]
    fn test_parse_invoice_text_spaced_type() {
        let re = Regex::new(r"电\s*子\s*发\s*票\s*[（(]\s*普\s*通\s*发\s*票\s*[)）]").unwrap();
        let text = "电 子 发 票 （ 普 通 发 票 ） 金额";
        assert!(re.find(text).is_some());
    }

    #[test]
    fn test_label_based_buyer_name() {
        let re =
            Regex::new(r"购\s*买\s*方.*?名\s*称[：:]\s*([^\s购销]+(?:\s+[^\s购销]+)*)").unwrap();
        let text = "购买方 名称： Cheng Qing 销售方 名称： 上海望七阁餐饮管理有限公司";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str().trim(), "Cheng Qing");
    }

    #[test]
    fn test_label_based_seller_name() {
        let re = Regex::new(
            r"销\s*售\s*方.*?名\s*称[：:]\s*([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)",
        )
        .unwrap();
        let text = "销售方 名称：上海望七阁餐饮管理有限公司";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "上海望七阁餐饮管理有限公司");
    }

    #[test]
    fn test_tax_id_from_label() {
        let re =
            Regex::new(r"统一社会信用代码\s*/\s*纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})").unwrap();
        let text = "统一社会信用代码/纳税人识别号：913100006074138050";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "913100006074138050");
    }

    #[test]
    fn test_amount_from_heji() {
        let re = Regex::new(r"合\s*计\s*¥?\s*(\d+\.?\d*)\s*¥\s*(\d+\.?\d*)").unwrap();
        let text = "合计 ¥100.00 ¥6.00";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "100.00");
        assert_eq!(caps.get(2).unwrap().as_str(), "6.00");
    }

    #[test]
    fn test_total_from_jiashui() {
        let re = Regex::new(r"价税合计.*?¥\s*(\d+\.?\d*)").unwrap();
        let text = "价税合计（大写） ¥106.00";
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "106.00");
    }

    #[test]
    fn test_parse_invoice_text_full_pipeline() {
        let text = "电子发票（普通发票） 发票号码 24112000000015301234 开票日期 2026年04月07日 \
                    Cheng Qing 上海星巴克咖啡经营有限公司 913100006074138050 \
                    *服务*技术咨询费 6% 合计 ¥100.00 ¥6.00 价税合计 ¥106.00";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.inv_type, "电子发票（普通发票）");
        assert_eq!(inv.number, "24112000000015301234");
        assert_eq!(inv.date, "2026-04-07");
        assert_eq!(inv.buyer_name, "Cheng Qing");
        assert_eq!(inv.seller_name, "上海星巴克咖啡经营有限公司");
        assert_eq!(inv.seller_tax_id, "913100006074138050");
        assert_eq!(inv.item_name, "技术咨询费");
        assert!((inv.tax_rate - 0.06).abs() < 0.001);
        assert!((inv.amount - 100.0).abs() < 0.01);
        assert!((inv.tax - 6.0).abs() < 0.01);
        assert!((inv.total - 106.0).abs() < 0.01);
    }

    #[test]
    fn test_label_based_seller_name_with_label_prefix() {
        let text = "电子发票（普通发票） 发票号码 24112000000015301234 开票日期 2026年04月07日 \
                    购买方 名称：Cheng Qing 销售方 名称：上海壹佰米网络科技有限公司 \
                    销售方 纳税人识别号：913100006074138050 \
                    *服务*技术咨询费 6% 合计 ¥100.00 ¥6.00 价税合计 ¥106.00";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.seller_name, "上海壹佰米网络科技有限公司");
        assert_ne!(inv.seller_name, "名称");
        assert_eq!(inv.buyer_name, "Cheng Qing");
        assert_eq!(inv.seller_tax_id, "913100006074138050");
    }

    #[test]
    fn test_label_based_seller_tax_id_variants() {
        let text1 = "统一社会信用代码/纳税人识别号：913100006074138050";
        let mut inv1 = models::Invoice::default();
        parse_invoice_text(text1, &mut inv1);
        assert_eq!(inv1.seller_tax_id, "913100006074138050");

        let text2 = "纳税人识别号：913100006074138050";
        let mut inv2 = models::Invoice::default();
        parse_invoice_text(text2, &mut inv2);
        assert_eq!(inv2.seller_tax_id, "913100006074138050");

        let text3 = "销售方 纳税人识别号：913100006074138050";
        let mut inv3 = models::Invoice::default();
        parse_invoice_text(text3, &mut inv3);
        assert_eq!(inv3.seller_tax_id, "913100006074138050");
    }

    #[test]
    fn test_label_priority_over_generic_regex() {
        let text = "电子发票（普通发票） 发票号码 24112000000015301234 开票日期 2026年04月07日 \
                    2026年04月07日 名称 上海测试网络科技有限公司 913100006074138050 \
                    购买方 名称：Cheng Qing 销售方 名称：上海壹佰米网络科技有限公司 \
                    *服务*技术咨询费 6% 合计 ¥100.00 ¥6.00 价税合计 ¥106.00";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.seller_name, "上海壹佰米网络科技有限公司");
        assert_ne!(inv.seller_name, "名称");
        assert_eq!(inv.buyer_name, "Cheng Qing");
    }

    #[test]
    fn test_tax_id_fallback() {
        let text = "纳税人识别号：913100006074138050";
        let re = Regex::new(r"纳税人识别号[：:]\s*(9[A-Z0-9]{15,19})").unwrap();
        let caps = re.captures(text).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "913100006074138050");
    }

    #[test]
    fn test_table_layout_seller_name() {
        let text = "电子发票（普通发票） 发票号码：26312000002105812051 开票日期：2026年04月07日 \
                    购 名称：Cheng Qing（个人） 销 名称：上海望七阁餐饮管理有限公司 \
                    买 售 方 方 \
                    信 统一社会信用代码/纳税人识别号： 信 统一社会信用代码/纳税人识别号：91310115MA1H73EJ5B \
                    息 息 \
                    *餐饮服务*餐费 160.40 1% 1.60 \
                    合 计 ¥160.40 ¥1.60 价税合计（大写） ¥162.00";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.seller_name, "上海望七阁餐饮管理有限公司");
        assert_ne!(inv.seller_name, "名称");
        assert_eq!(inv.buyer_name, "Cheng Qing（个人）");
        assert_ne!(inv.buyer_name, "购");
    }

    #[test]
    fn test_table_layout_tax_id() {
        let text = "购 名称：Cheng Qing（个人） 销 名称：上海望七阁餐饮管理有限公司 买 售 方 方 \
                    信 统一社会信用代码/纳税人识别号： 信 统一社会信用代码/纳税人识别号：91310115MA1H73EJ5B 息 息";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.seller_tax_id, "91310115MA1H73EJ5B");
        assert!(inv.buyer_tax_id.is_empty());
    }

    #[test]
    fn test_table_layout_with_pure_digit_buyer_tax() {
        let text = "购 名称：儒德管理咨询(上海)有限公司 销 名称：上海想点就点餐饮有限公司 买 售 方 方 \
                    信 统一社会信用代码/纳税人识别号：913101156711533820 信 统一社会信用代码/纳税人识别号：91310110MA7H3WGJ6C 息 息";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.buyer_name, "儒德管理咨询(上海)有限公司");
        assert_eq!(inv.seller_name, "上海想点就点餐饮有限公司");
        assert_eq!(inv.buyer_tax_id, "913101156711533820");
        assert_eq!(inv.seller_tax_id, "91310110MA7H3WGJ6C");
    }

    #[test]
    fn test_alphanumeric_tax_id() {
        let re = Regex::new(r"(9[A-Z0-9]{15,19})").unwrap();
        assert_eq!(
            re.find("91310115MA1H73EJ5B").unwrap().as_str(),
            "91310115MA1H73EJ5B"
        );
        assert_eq!(
            re.find("913100006074138050").unwrap().as_str(),
            "913100006074138050"
        );
    }

    #[test]
    fn test_table_layout_tax_id_pdf_oxide_reverse() {
        let text = "购 名称：Cheng Qing（个人） 销 名称：上海望七阁餐饮管理有限公司 买 售 方 方 \
                    信 统一社会信用代码/纳税人识别号： 信 91310115MA1H73EJ5B 统一社会信用代码/纳税人识别号： 息 息";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.seller_name, "上海望七阁餐饮管理有限公司");
        assert_eq!(inv.seller_tax_id, "91310115MA1H73EJ5B");
        assert!(inv.buyer_tax_id.is_empty());
    }

    #[test]
    fn test_table_layout_tax_id_pdf_oxide_both_tax() {
        let text = "购 名称：儒德管理咨询(上海)有限公司 销 名称：上海想点就点餐饮有限公司 买 售 方 方 \
                    信 统一社会信用代码/纳税人识别号：913101156711533820 信 91310110MA7H3WGJ6C 统一社会信用代码/纳税人识别号： 息 息";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.buyer_name, "儒德管理咨询(上海)有限公司");
        assert_eq!(inv.seller_name, "上海想点就点餐饮有限公司");
        assert_eq!(inv.buyer_tax_id, "913101156711533820");
        assert_eq!(inv.seller_tax_id, "91310110MA7H3WGJ6C");
    }

    #[test]
    fn test_reverse_tax_id_before_label() {
        let text = "91310115MA1H73EJ5B 统一社会信用代码/纳税人识别号：";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.seller_tax_id, "91310115MA1H73EJ5B");
    }

    #[test]
    fn test_ofd_pdf_layout_detection() {
        let re = Regex::new(r"购\s+销\s+名称[：:]").unwrap();
        assert!(re.is_match("购 销 名称：CHENG QING 名称：上海笙诚企业发展有限公司"));
        assert!(!re.is_match("购 名称：Cheng Qing 销 名称：上海望七阁餐饮管理有限公司"));
    }

    #[test]
    fn test_ofd_pdf_seller_name_extraction() {
        let text = "电子发票（普通发票） 发票号码： 26312000002379573196 开票日期： 2026年04月19日 \
                    购 销 名称： CHENG QING 名称： 上海笙诚企业发展有限公司 \
                    买 售 方 方 信 统一社会信用代码/纳税人识别号： 信 91310118MA7MD5K52Q 统一社会信用代码/纳税人识别号： 息 息 \
                    *餐饮服务*餐饮服务 1% 420.79 4.21 合 计 ¥420.79 ¥4.21 ¥425.00";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.seller_name, "上海笙诚企业发展有限公司");
    }

    #[test]
    fn test_ofd_pdf_buyer_name_extraction() {
        let text = "电子发票（普通发票） 发票号码： 26312000002379573196 开票日期： 2026年04月19日 \
                    购 销 名称： CHENG QING 名称： 上海笙诚企业发展有限公司 \
                    买 售 方 方 信 统一社会信用代码/纳税人识别号： 信 91310118MA7MD5K52Q 统一社会信用代码/纳税人识别号： 息 息";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.buyer_name, "CHENG QING");

        let text2 = "电子发票（普通发票） 发票号码： 26312000002311043146 开票日期： 2026年04月15日 \
                     购 销 名称： 程青 名称： 上海兰心荟餐饮管理有限公司 \
                     买 售 方 方 信 统一社会信用代码/纳税人识别号： 信 91310109MA1G5MRP7F 统一社会信用代码/纳税人识别号： 息 息";
        let mut inv2 = models::Invoice::default();
        parse_invoice_text(text2, &mut inv2);
        assert_eq!(inv2.buyer_name, "程青");
    }

    #[test]
    fn test_ofd_pdf_tax_id_extraction() {
        let text = "购 销 名称： CHENG QING 名称： 上海笙诚企业发展有限公司 \
                    买 售 方 方 信 统一社会信用代码 /纳税人识别号 ： 信 91310118MA7MD5K52Q 统一社会信用代码 /纳税人识别号 ： 息 息";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.seller_tax_id, "91310118MA7MD5K52Q");
        assert!(inv.buyer_tax_id.is_empty());
    }

    #[test]
    fn test_label_value_validation() {
        let label_keywords = [
            "名称", "项目", "规格", "型号", "单位", "数量", "单价", "金额", "税率", "税额",
        ];
        fn is_valid_label_value(val: &str, keywords: &[&str]) -> bool {
            if val.len() > 50 {
                return false;
            }
            for kw in keywords {
                if val.contains(kw) {
                    return false;
                }
            }
            true
        }

        assert!(!is_valid_label_value(
            "上海想点就点餐饮有限公司 买 售 方 方 信",
            &label_keywords
        ));
        assert!(!is_valid_label_value(
            "购 销 名称：儒德管理咨询(上海)有限公司 名称：",
            &label_keywords
        ));
        assert!(is_valid_label_value(
            "上海望七阁餐饮管理有限公司",
            &label_keywords
        ));
        assert!(is_valid_label_value("Cheng Qing", &label_keywords));

        let long_val = "A".repeat(51);
        assert!(!is_valid_label_value(&long_val, &label_keywords));
    }

    #[test]
    fn test_extract_from_pdf_with_ocr_no_model() {
        let result = extract_from_pdf_with_ocr(&[], Some("/tmp/nonexistent_ocr_model_test"));
        assert!(result.is_ok());
        let inv = result.unwrap();
        assert!(inv.number.is_empty());
    }

    #[test]
    fn test_extract_from_pdf_with_ocr_invalid_data() {
        let result = extract_from_pdf_with_ocr(&[0x00, 0x01, 0x02, 0x03], None);
        assert!(result.is_ok());
        let inv = result.unwrap();
        assert!(inv.number.is_empty());
    }

    #[test]
    fn test_ofd_pdf_full_pipeline() {
        let text = "电 子 发 票 （ 普 通 发 票 ） 发 票 号 码 ： 26312000002379573196 \
                    开 票 日 期 ： 2026年04月19日 \
                    购 销 名称： CHENG QING 名称： 上海笙诚企业发展有限公司 \
                    买 售 方 方 信 统一社会信用代码 /纳税人识别号 ： 信 91310118MA7MD5K52Q 统一社会信用代码 /纳税人识别号 ： 息 息 \
                    项目名称 规格型号 单 位 数 量 单 价 金 额 税率/征收率 税 额 \
                    *餐饮服务*餐饮服务 1 420.79 420.79 1% 4.21 \
                    合 计 ¥420.79 ¥4.21 肆佰贰拾伍圆整 ¥425.00 \
                    价税合计（ 大写） （ 小写） 备 注 开 票 人： 李海燕";
        let mut inv = models::Invoice::default();
        parse_invoice_text(text, &mut inv);
        assert_eq!(inv.number, "26312000002379573196");
        assert_eq!(inv.date, "2026-04-19");
        assert_eq!(inv.buyer_name, "CHENG QING");
        assert_eq!(inv.seller_name, "上海笙诚企业发展有限公司");
        assert_eq!(inv.seller_tax_id, "91310118MA7MD5K52Q");
        assert_eq!(inv.item_name, "餐饮服务");
        assert!((inv.tax_rate - 0.01).abs() < 0.001);
    }
}
