use crate::models;
use regex::Regex;

pub fn extract_from_pdf(data: &[u8]) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let mut inv = models::Invoice::default();

    if let Ok(doc) = pdf_oxide::PdfDocument::from_bytes(data.to_vec()) {
        let text = doc.extract_all_text().unwrap_or_default();
        if !text.trim().is_empty() {
            parse_invoice_text(&text, &mut inv);
        }
    }

    Ok(inv)
}

fn parse_invoice_text(text: &str, inv: &mut models::Invoice) {
    let normalized = match Regex::new(r"\s+") {
        Ok(re) => re.replace_all(text, " ").to_string(),
        Err(_) => return,
    };

    if inv.number.is_empty() {
        let re = Regex::new(r"\d{20}").unwrap();
        if let Some(m) = re.find(&normalized) {
            inv.number = m.as_str().to_string();
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
            }
        }
    }

    if inv.seller_name.is_empty() || inv.buyer_name.is_empty() {
        let re_names = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+([^\d]+?)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（） ]+?)\s+(9\d{14,17})",
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

    if inv.seller_name.is_empty() || inv.buyer_name.is_empty() {
        let re_names2 = Regex::new(
            r"(\d{4}年\d{2}月\d{2}日)\s+(\S+)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)",
        )
        .unwrap();
        if let Some(caps) = re_names2.captures(&normalized) {
            if inv.buyer_name.is_empty() {
                inv.buyer_name = caps.get(2).unwrap().as_str().to_string();
            }
            if inv.seller_name.is_empty() {
                inv.seller_name = caps.get(3).unwrap().as_str().to_string();
            }
        }
    }

    if inv.buyer_name.is_empty() {
        let re =
            Regex::new(r"购\s*买\s*方.*?名\s*称[：:]\s*([^\s购销]+(?:\s+[^\s购销]+)*)").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            let val = caps.get(1).unwrap().as_str().trim();
            if !val.is_empty() {
                inv.buyer_name = val.to_string();
            }
        }
    }

    if inv.seller_name.is_empty() {
        let re = Regex::new(
            r"销\s*售\s*方.*?名\s*称[：:]\s*([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（）]+)",
        )
        .unwrap();
        if let Some(caps) = re.captures(&normalized) {
            let val = caps.get(1).unwrap().as_str().trim();
            if !val.is_empty() {
                inv.seller_name = val.to_string();
            }
        }
    }

    if inv.seller_tax_id.is_empty() {
        let re = Regex::new(r"统一社会信用代码\s*/\s*纳税人识别号[：:]\s*(9\d{14,17})").unwrap();
        if let Some(caps) = re.captures(&normalized) {
            inv.seller_tax_id = caps.get(1).unwrap().as_str().to_string();
        }
    }

    if inv.item_name.is_empty() {
        let re = Regex::new(r"\*[^*]+\*([^*\s]+)").unwrap();
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
            r"(\d{4}年\d{2}月\d{2}日)\s+([^\d]+?)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（） ]+?)\s+(9\d{14,17})",
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
            r"(\d{4}年\d{2}月\d{2}日)\s+([^\d]+?)\s+([\x{4e00}-\x{9fff}][\x{4e00}-\x{9fff}\w()（） ]+?)\s+(9\d{14,17})",
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
        let re = Regex::new(r"统一社会信用代码\s*/\s*纳税人识别号[：:]\s*(9\d{14,17})").unwrap();
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
}
