use crate::extract_xml;
use crate::models;
use std::io::Read;

pub fn extract_from_ofd(data: &[u8]) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;

    let mut inv = models::Invoice::default();

    if let Ok(content) = read_zip_file(&mut archive, "OFD.xml") {
        extract_custom_data_from_ofd_xml(&content, &mut inv);
    }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.contains("original_invoice.xml")
            || name.contains("Attach") && name.ends_with(".xml")
        {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            if let Ok(xml_inv) = extract_xml::extract_from_xml(&content) {
                merge_from_xml(&mut inv, &xml_inv);
                return Ok(inv);
            }
        }
    }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.contains("Tag") && name.ends_with(".xml") {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            if let Ok(xml_inv) = extract_xml::extract_from_xml(&content) {
                merge_from_xml(&mut inv, &xml_inv);
                return Ok(inv);
            }
        }
    }

    if has_any_field(&inv) {
        Ok(inv)
    } else {
        Err("No invoice data found in OFD file".into())
    }
}

fn read_zip_file(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = archive.by_name(name)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn extract_custom_data_from_ofd_xml(xml_content: &str, inv: &mut models::Invoice) {
    if let Ok(doc) = roxmltree::Document::parse(xml_content) {
        let mut custom_data: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for node in doc
            .descendants()
            .filter(|n| n.has_tag_name("CustomData") || n.tag_name().name() == "CustomData")
        {
            let name = node.attribute("Name").unwrap_or("");
            let value = node.text().unwrap_or("");
            if !name.is_empty() && !value.is_empty() {
                custom_data.insert(name.to_string(), value.to_string());
            }
        }

        if let Some(v) = custom_data.get("发票号码") {
            inv.number = v.clone();
        }
        if let Some(v) = custom_data.get("开票日期") {
            inv.date = v.replace("年", "-").replace("月", "-").replace("日", "");
        }
        if let Some(v) = custom_data.get("合计金额") {
            inv.amount = v.trim().parse().unwrap_or(0.0);
        }
        if let Some(v) = custom_data.get("合计税额") {
            inv.tax = v.trim().parse().unwrap_or(0.0);
        }
        if let Some(v) = custom_data.get("价税合计") {
            inv.total = v.trim().parse().unwrap_or(0.0);
        }
        if let Some(v) = custom_data.get("销售方纳税人识别号") {
            inv.seller_tax_id = v.clone();
        }
    }
}

fn merge_from_xml(target: &mut models::Invoice, source: &models::Invoice) {
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

fn has_any_field(inv: &models::Invoice) -> bool {
    !inv.number.is_empty() || !inv.date.is_empty() || inv.amount != 0.0 || inv.total != 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn create_ofd_zip(files: &[(&str, &str)]) -> Vec<u8> {
        let buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (path, content) in files {
            zip.start_file(*path, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        let buf = zip.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn test_extract_from_ofd_with_custom_data() {
        let ofd_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016">
    <ofd:DocBody>
        <ofd:DocInfo>
            <ofd:CustomData Name="发票号码">24112000000015301234</ofd:CustomData>
            <ofd:CustomData Name="开票日期">2026年04月07日</ofd:CustomData>
            <ofd:CustomData Name="合计金额">160.40</ofd:CustomData>
            <ofd:CustomData Name="合计税额">1.60</ofd:CustomData>
            <ofd:CustomData Name="价税合计">162.00</ofd:CustomData>
            <ofd:CustomData Name="销售方纳税人识别号">913101150934986600</ofd:CustomData>
        </ofd:DocInfo>
    </ofd:DocBody>
</ofd:OFD>"#;
        let data = create_ofd_zip(&[("OFD.xml", ofd_xml)]);
        let inv = extract_from_ofd(&data).unwrap();
        assert_eq!(inv.number, "24112000000015301234");
        assert_eq!(inv.date, "2026-04-07");
        assert_eq!(inv.amount, 160.40);
        assert_eq!(inv.tax, 1.60);
        assert_eq!(inv.total, 162.0);
        assert_eq!(inv.seller_tax_id, "913101150934986600");
    }

    #[test]
    fn test_extract_from_ofd_with_original_invoice_xml() {
        let ofd_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016">
    <ofd:DocBody><ofd:DocInfo/></ofd:DocBody>
</ofd:OFD>"#;
        let einv_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<EInvoice>
    <TaxSupervisionInfo>
        <InvoiceNumber>98765432109876543210</InvoiceNumber>
        <IssueTime>2026-03-20</IssueTime>
    </TaxSupervisionInfo>
    <EInvoiceData>
        <BasicInformation>
            <TotalAmWithoutTax>500.00</TotalAmWithoutTax>
            <TotalTaxAm>30.00</TotalTaxAm>
        </BasicInformation>
        <SellerInformation>
            <SellerName>OFD测试公司</SellerName>
        </SellerInformation>
    </EInvoiceData>
</EInvoice>"#;
        let data = create_ofd_zip(&[
            ("OFD.xml", ofd_xml),
            ("Doc_0/Attach/original_invoice.xml", einv_xml),
        ]);
        let inv = extract_from_ofd(&data).unwrap();
        assert_eq!(inv.number, "98765432109876543210");
        assert_eq!(inv.date, "2026-03-20");
        assert_eq!(inv.amount, 500.0);
        assert_eq!(inv.seller_name, "OFD测试公司");
    }

    #[test]
    fn test_extract_from_ofd_no_data() {
        let data = create_ofd_zip(&[("readme.txt", "no invoice data")]);
        let result = extract_from_ofd(&data);
        assert!(result.is_err());
    }
}
