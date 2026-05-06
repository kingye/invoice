use crate::models;

pub fn generate_detail_report(
    invoices: &[models::Invoice],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use rust_xlsxwriter::*;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("明细表")?;

    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    let num_fmt = Format::new()
        .set_num_format("#,##0.00")
        .set_border(FormatBorder::Thin);

    let pct_fmt = Format::new()
        .set_num_format("0%")
        .set_border(FormatBorder::Thin);

    let cell_fmt = Format::new().set_border(FormatBorder::Thin);

    let headers = [
        "序号",
        "发票号码",
        "日期",
        "类型",
        "项目名称",
        "金额",
        "税率",
        "税金",
        "价税合计",
        "销售方",
        "销售方税号",
        "购买方",
        "购买方税号",
        "分类",
        "备注",
    ];

    for (col, h) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *h, &header_fmt)?;
    }

    for (row_idx, inv) in invoices.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        worksheet.write_number_with_format(row, 0, inv.id as f64, &cell_fmt)?;
        worksheet.write_string_with_format(row, 1, &inv.number, &cell_fmt)?;
        worksheet.write_string_with_format(row, 2, &inv.date, &cell_fmt)?;
        worksheet.write_string_with_format(row, 3, &inv.inv_type, &cell_fmt)?;
        worksheet.write_string_with_format(row, 4, &inv.item_name, &cell_fmt)?;
        worksheet.write_number_with_format(row, 5, inv.amount, &num_fmt)?;
        worksheet.write_number_with_format(row, 6, inv.tax_rate, &pct_fmt)?;
        worksheet.write_number_with_format(row, 7, inv.tax, &num_fmt)?;
        worksheet.write_number_with_format(row, 8, inv.total, &num_fmt)?;
        worksheet.write_string_with_format(row, 9, &inv.seller_name, &cell_fmt)?;
        worksheet.write_string_with_format(row, 10, &inv.seller_tax_id, &cell_fmt)?;
        worksheet.write_string_with_format(row, 11, &inv.buyer_name, &cell_fmt)?;
        worksheet.write_string_with_format(row, 12, &inv.buyer_tax_id, &cell_fmt)?;
        worksheet.write_string_with_format(row, 13, &inv.category, &cell_fmt)?;
        worksheet.write_string_with_format(row, 14, &inv.remark, &cell_fmt)?;
    }

    worksheet.set_column_width(0, 6)?;
    worksheet.set_column_width(1, 14)?;
    worksheet.set_column_width(2, 12)?;
    worksheet.set_column_width(5, 14)?;
    worksheet.set_column_width(8, 14)?;

    workbook.save(output_path)?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct SummaryEntry {
    pub category: String,
    pub invoice_type: String,
    pub count: u32,
    pub total_amount: f64,
    pub total_tax: f64,
    pub total: f64,
    pub weighted_tax_rate: f64,
}

pub fn compute_summary(invoices: &[models::Invoice]) -> Vec<SummaryEntry> {
    let mut map: std::collections::HashMap<(String, String), SummaryEntry> =
        std::collections::HashMap::new();

    for inv in invoices {
        let key = (inv.category.clone(), inv.inv_type.clone());
        map.entry(key)
            .and_modify(|entry| {
                entry.count += 1;
                entry.total_amount += inv.amount;
                entry.total_tax += inv.tax;
                entry.total += inv.total;
            })
            .or_insert(SummaryEntry {
                category: inv.category.clone(),
                invoice_type: inv.inv_type.clone(),
                count: 1,
                total_amount: inv.amount,
                total_tax: inv.tax,
                total: inv.total,
                weighted_tax_rate: 0.0,
            });
    }

    let mut entries: Vec<SummaryEntry> = map.into_values().collect();
    for entry in &mut entries {
        if entry.total_amount > 0.0 {
            entry.weighted_tax_rate = entry.total_tax / entry.total_amount;
        }
    }
    entries
}

pub fn generate_summary_report(
    entries: &[SummaryEntry],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use rust_xlsxwriter::*;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("汇总表")?;

    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x70AD47))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    let num_fmt = Format::new()
        .set_num_format("#,##0.00")
        .set_border(FormatBorder::Thin);

    let pct_fmt = Format::new()
        .set_num_format("0.00%")
        .set_border(FormatBorder::Thin);

    let cell_fmt = Format::new().set_border(FormatBorder::Thin);

    let headers = [
        "分类",
        "发票类型",
        "数量",
        "金额合计",
        "税金合计",
        "价税合计",
        "加权平均税率",
    ];

    for (col, h) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *h, &header_fmt)?;
    }

    for (row_idx, entry) in entries.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        worksheet.write_string_with_format(row, 0, &entry.category, &cell_fmt)?;
        worksheet.write_string_with_format(row, 1, &entry.invoice_type, &cell_fmt)?;
        worksheet.write_number_with_format(row, 2, entry.count as f64, &cell_fmt)?;
        worksheet.write_number_with_format(row, 3, entry.total_amount, &num_fmt)?;
        worksheet.write_number_with_format(row, 4, entry.total_tax, &num_fmt)?;
        worksheet.write_number_with_format(row, 5, entry.total, &num_fmt)?;
        worksheet.write_number_with_format(row, 6, entry.weighted_tax_rate, &pct_fmt)?;
    }

    worksheet.set_column_width(0, 14)?;
    worksheet.set_column_width(1, 14)?;
    worksheet.set_column_width(3, 14)?;
    worksheet.set_column_width(5, 14)?;

    workbook.save(output_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_summary() {
        let invoices = vec![
            models::Invoice {
                category: "服务".to_string(),
                inv_type: "电子发票".to_string(),
                amount: 1000.0,
                tax: 60.0,
                total: 1060.0,
                ..Default::default()
            },
            models::Invoice {
                category: "服务".to_string(),
                inv_type: "电子发票".to_string(),
                amount: 2000.0,
                tax: 120.0,
                total: 2120.0,
                ..Default::default()
            },
        ];
        let summary = compute_summary(&invoices);
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].count, 2);
        assert_eq!(summary[0].total_amount, 3000.0);
    }
}
