use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i64,
    pub number: String,
    pub date: String,
    #[serde(rename = "type")]
    pub inv_type: String,
    pub item_name: String,
    pub amount: f64,
    pub tax_rate: f64,
    pub tax: f64,
    pub total: f64,
    pub seller_name: String,
    pub seller_tax_id: String,
    pub buyer_name: String,
    pub buyer_tax_id: String,
    pub category: String,
    pub remark: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct InvoiceUpdate {
    pub number: Option<String>,
    pub date: Option<String>,
    pub inv_type: Option<String>,
    pub item_name: Option<String>,
    pub amount: Option<f64>,
    pub tax_rate: Option<f64>,
    pub tax: Option<f64>,
    pub total: Option<f64>,
    pub seller_name: Option<String>,
    pub seller_tax_id: Option<String>,
    pub buyer_name: Option<String>,
    pub buyer_tax_id: Option<String>,
    pub category: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Attachment {
    pub id: i64,
    pub invoice_id: i64,
    pub filename: String,
    pub filepath: String,
    pub file_hash: String,
    pub file_size: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportResult {
    pub detail_path: String,
    pub summary_path: String,
    pub output_dir: String,
    pub period: String,
}

#[derive(Debug, Clone, Default)]
pub struct Closing {
    pub id: i64,
    pub close_type: String,
    pub period: String,
    pub closed_at: String,
    pub archive_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_fields() {
        let inv = Invoice {
            id: 1,
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
        assert_eq!(inv.number, "FP001");
        assert_eq!(inv.date, "2026-04-01");
        assert_eq!(inv.amount, 1000.0);
        assert_eq!(inv.tax_rate, 0.06);
    }

    #[test]
    fn test_attachment_fields() {
        let att = Attachment {
            id: 1,
            invoice_id: 1,
            filename: "test.pdf".to_string(),
            filepath: ".invoice/data/FP001/test.pdf".to_string(),
            file_hash: "abc123".to_string(),
            file_size: 1024,
            ..Default::default()
        };
        assert_eq!(att.filename, "test.pdf");
        assert_eq!(att.file_size, 1024);
    }
}
